use crate::model::{AlpacaStreamMessage, AuthMessage, SubscribeMessage};
use futures_util::{SinkExt, StreamExt};
use quantum_engine::{QuantumAnalyticsSnapshot, QuantumMarketEngine};
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;

pub enum AlpacaDataFeed {
    Iex, // Free tier IEX feed
    Sip, // Institutional full CTA/UTP SIP feed
}

#[derive(Debug, Clone)]
pub enum AlpacaAuth {
    KeySecret { key: String, secret: String },
    OAuthToken(String),
}

pub struct AlpacaStreamer {
    auth: AlpacaAuth,
    feed: AlpacaDataFeed,
    engine: QuantumMarketEngine,
}

impl AlpacaStreamer {
    pub fn with_key_secret(key: String, secret: String, feed: AlpacaDataFeed, tick_size: f64) -> Self {
        Self {
            auth: AlpacaAuth::KeySecret { key, secret },
            feed,
            engine: QuantumMarketEngine::new(tick_size),
        }
    }

    pub fn with_oauth_token(token: String, feed: AlpacaDataFeed, tick_size: f64) -> Self {
        Self {
            auth: AlpacaAuth::OAuthToken(token),
            feed,
            engine: QuantumMarketEngine::new(tick_size),
        }
    }

    /// WebSocket URL according to feed type
    pub fn websocket_url(&self) -> &'static str {
        match self.feed {
            AlpacaDataFeed::Iex => "wss://stream.data.alpaca.markets/v2/iex",
            AlpacaDataFeed::Sip => "wss://stream.data.alpaca.markets/v2/sip",
        }
    }

    /// Connects to Alpaca, authenticates, subscribes to symbols, and streams directly into QuantumMarketEngine.
    pub async fn stream_symbol<F>(
        &mut self,
        symbols: Vec<&str>,
        mut on_snapshot: F,
    ) -> Result<(), Box<dyn Error>>
    where
        F: FnMut(&QuantumAnalyticsSnapshot),
    {
        self.stream_with_symbol(symbols, |_sym, snap| on_snapshot(snap)).await
    }

    /// Connects to Alpaca, authenticates, subscribes to symbols, and streams providing the symbol string with each snapshot.
    pub async fn stream_with_symbol<F>(
        &mut self,
        symbols: Vec<&str>,
        mut on_snapshot: F,
    ) -> Result<(), Box<dyn Error>>
    where
        F: FnMut(&str, &QuantumAnalyticsSnapshot),
    {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let url = self.websocket_url();
        println!("[AlpacaStreamer] Connecting to {}", url);

        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();

        println!("[AlpacaStreamer] Connected. Waiting for server greeting...");

        // 1. Receive Greeting
        if let Some(msg) = read.next().await {
            let text = msg?.into_text()?;
            println!("[AlpacaStreamer] Server greeting: {}", text);
        }

        // 2. Send Authentication (Key/Secret or OAuth Bearer Token)
        let auth_msg = match &self.auth {
            AlpacaAuth::KeySecret { key, secret } => AuthMessage {
                action: "auth",
                key: Some(key),
                secret: Some(secret),
                token: None,
            },
            AlpacaAuth::OAuthToken(token) => AuthMessage {
                action: "auth",
                key: Some("oauth"),
                secret: Some(token.as_str()),
                token: None,
            },
        };
        let auth_json = serde_json::to_string(&auth_msg)?;
        write.send(Message::Text(auth_json)).await?;

        // 3. Receive Auth Response
        if let Some(msg) = read.next().await {
            let text = msg?.into_text()?;
            println!("[AlpacaStreamer] Auth response: {}", text);
        }

        // 4. Send Subscription Request
        let sub_msg = SubscribeMessage {
            action: "subscribe",
            trades: symbols.clone(),
            quotes: symbols.clone(),
        };
        let sub_json = serde_json::to_string(&sub_msg)?;
        write.send(Message::Text(sub_json)).await?;
        println!("[AlpacaStreamer] Subscribed to {:?}", symbols);

        // 5. Real-time Ingestion Loop
        while let Some(msg_result) = read.next().await {
            let msg = match msg_result {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("[AlpacaStreamer] WebSocket read error: {:?}", e);
                    break;
                }
            };

            if let Message::Text(text) = msg {
                let current_time_ns = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64;

                // Alpaca sends an array of messages per frame: [{"T":"q", ...}, {"T":"t", ...}]
                let parsed_messages: Result<Vec<AlpacaStreamMessage>, _> = serde_json::from_str(&text);
                match parsed_messages {
                    Ok(messages) => {
                        for m in messages {
                            match m {
                                AlpacaStreamMessage::Quote(q) => {
                                    let quote = q.to_microstructure_quote(current_time_ns);
                                    if let Some(snap) = self.engine.on_quote(quote) {
                                        on_snapshot(&q.symbol, &snap);
                                    }
                                }
                                AlpacaStreamMessage::Trade(t) => {
                                    let mid = self
                                        .engine
                                        .tracker
                                        .current_quote
                                        .map(|q| q.mid_price())
                                        .unwrap_or(t.price);
                                    let trade = t.to_microstructure_trade(current_time_ns, mid);
                                    if let Some(snap) = self.engine.on_trade(trade) {
                                        on_snapshot(&t.symbol, &snap);
                                    }
                                }
                                AlpacaStreamMessage::Subscription { trades, quotes } => {
                                    println!("[AlpacaStreamer] Suscripcion confirmada: trades={:?}, quotes={:?}", trades, quotes);
                                }
                                AlpacaStreamMessage::Error { msg, code } => {
                                    eprintln!("[AlpacaStreamer] Error from Alpaca (code {}): {}", code, msg);
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(e) => {
                        if let Ok(single) = serde_json::from_str::<AlpacaStreamMessage>(&text) {
                            match single {
                                AlpacaStreamMessage::Quote(q) => {
                                    let quote = q.to_microstructure_quote(current_time_ns);
                                    if let Some(snap) = self.engine.on_quote(quote) {
                                        on_snapshot(&q.symbol, &snap);
                                    }
                                }
                                AlpacaStreamMessage::Trade(t) => {
                                    let mid = self
                                        .engine
                                        .tracker
                                        .current_quote
                                        .map(|q| q.mid_price())
                                        .unwrap_or(t.price);
                                    let trade = t.to_microstructure_trade(current_time_ns, mid);
                                    if let Some(snap) = self.engine.on_trade(trade) {
                                        on_snapshot(&t.symbol, &snap);
                                    }
                                }
                                AlpacaStreamMessage::Subscription { trades, quotes } => {
                                    println!("[AlpacaStreamer] Suscripcion confirmada: trades={:?}, quotes={:?}", trades, quotes);
                                }
                                AlpacaStreamMessage::Error { msg, code } => {
                                    eprintln!("[AlpacaStreamer] Error from Alpaca (code {}): {}", code, msg);
                                }
                                _ => {}
                            }
                        } else {
                            eprintln!("[AlpacaStreamer] Warning al deserializar: {:?} | Raw: {}", e, text);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
