use market_microstructure::{Quote, Side, Trade};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct AuthMessage<'a> {
    pub action: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<&'a str>,
}

#[derive(Debug, Serialize)]
pub struct SubscribeMessage<'a> {
    pub action: &'static str,
    pub trades: Vec<&'a str>,
    pub quotes: Vec<&'a str>,
}

/// Raw message from Alpaca Market Data WebSocket stream (SIP or IEX)
#[derive(Debug, Deserialize)]
#[serde(tag = "T")]
pub enum AlpacaStreamMessage {
    #[serde(rename = "success")]
    Success { msg: String },

    #[serde(rename = "error")]
    Error { msg: String, code: u32 },

    #[serde(rename = "subscription")]
    Subscription {
        #[serde(default)]
        trades: Vec<String>,
        #[serde(default)]
        quotes: Vec<String>,
    },

    #[serde(rename = "q")]
    Quote(AlpacaQuote),

    #[serde(rename = "t")]
    Trade(AlpacaTrade),

    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct AlpacaQuote {
    #[serde(rename = "S")]
    pub symbol: String,
    #[serde(rename = "bp")]
    pub bid_price: f64,
    #[serde(rename = "bs")]
    pub bid_size: f64,
    #[serde(rename = "ap")]
    pub ask_price: f64,
    #[serde(rename = "as")]
    pub ask_size: f64,
    #[serde(rename = "t", default)]
    pub timestamp: Option<String>,
}

impl AlpacaQuote {
    /// Converts to internal zero-cost Quote struct.
    pub fn to_microstructure_quote(&self, timestamp_ns: u64) -> Quote {
        Quote::new(
            timestamp_ns,
            self.bid_price,
            self.bid_size,
            self.ask_price,
            self.ask_size,
        )
    }
}

#[derive(Debug, Deserialize)]
pub struct AlpacaTrade {
    #[serde(rename = "S")]
    pub symbol: String,
    #[serde(rename = "p")]
    pub price: f64,
    #[serde(rename = "s")]
    pub size: f64,
    #[serde(rename = "t", default)]
    pub timestamp: Option<String>,
}

impl AlpacaTrade {
    /// Infers taker side by comparing trade price to prevailing best bid/ask
    pub fn to_microstructure_trade(&self, timestamp_ns: u64, prevailing_mid: f64) -> Trade {
        let side = if self.price >= prevailing_mid {
            Side::Buy
        } else {
            Side::Sell
        };
        Trade::new(timestamp_ns, self.price, self.size, side)
    }
}
