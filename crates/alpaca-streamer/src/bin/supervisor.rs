use alpaca_streamer::{AlpacaAuth, AlpacaDataFeed, AlpacaStreamer, EnvConfig};
use market_microstructure::Side;
use market_simulator::{MarketRegime, MicrostructureSimulator};
use quantum_engine::{
    MarketQuantumPhase, OrderIntent, PortfolioAuditor, QuantumAnalyticsSnapshot, QuantumMarketEngine,
    SmartOrderRouter,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// DEF-09 fix: Zero-allocation RSS read — no Vec, no collect.
fn get_memory_rss_mb() -> f64 {
    let Ok(content) = std::fs::read_to_string("/proc/self/statm") else { return 0.0; };
    // statm fields are space-separated: total rss shared text lib data dirty
    // Skip first token, parse second
    let mut iter = content.split_ascii_whitespace();
    iter.next(); // skip "total"
    if let Some(rss_pages) = iter.next().and_then(|s| s.parse::<u64>().ok()) {
        return (rss_pages * 4096) as f64 / (1024.0 * 1024.0);
    }
    0.0
}

#[derive(Clone)]
enum BrokerAuth {
    OAuth(Arc<String>),
    KeySecret { key: Arc<String>, secret: Arc<String> },
    None,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct LiveBrokerPosition {
    symbol: String,
    side_buy: bool,
    entry_price: f64,
    qty: f64,
    notional: f64,
    entry_time: Instant,
}

/// Closes 100% of an open position atomically on Alpaca Paper REST API.
/// Uses DELETE /v2/positions/{symbol} to eliminate decimal rounding errors and 403 insufficient balance rejections.
async fn close_alpaca_position(
    client: &reqwest::Client,
    auth: &BrokerAuth,
    symbol: &str,
) -> bool {
    if matches!(auth, BrokerAuth::None) {
        return false;
    }
    let clean_symbol = symbol.replace('/', "");
    let url = format!("https://paper-api.alpaca.markets/v2/positions/{}", clean_symbol);
    let mut req = client.delete(&url);

    match auth {
        BrokerAuth::OAuth(token) => {
            req = req.header("Authorization", format!("Bearer {}", *token));
        }
        BrokerAuth::KeySecret { key, secret } => {
            req = req
                .header("APCA-API-KEY-ID", key.as_str())
                .header("APCA-API-SECRET-KEY", secret.as_str());
        }
        BrokerAuth::None => return false,
    }

    match req.send().await {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                println!(
                    " \x1b[1;32m[ALPACA POSITION CLOSED]\x1b[0m Posicion liquidada atomicamente en Broker: {}",
                    symbol
                );
                true
            } else if status.as_u16() == 404 {
                println!(
                    " \x1b[1;33m[ALPACA POSITION SYNC]\x1b[0m Posicion ya liquidada previamente en Broker: {}",
                    symbol
                );
                true
            } else {
                let err_text = resp.text().await.unwrap_or_default();
                eprintln!(
                    " \x1b[1;31m[ALPACA CLOSE ERROR ({})]\x1b[0m Fallo al liquidar {}: {}",
                    status, symbol, err_text.trim()
                );
                false
            }
        }
        Err(e) => {
            eprintln!(" \x1b[1;31m[ALPACA NETWORK ERROR]\x1b[0m Error enviando DELETE para {}: {:?}", symbol, e);
            false
        }
    }
}

async fn execute_alpaca_paper_order(
    client: &reqwest::Client,
    auth: &BrokerAuth,
    symbol: &str,
    side: &str,
    qty_or_notional: (Option<f64>, Option<f64>),
    limit_price: Option<f64>,
    bracket: Option<(f64, f64)>,
) -> bool {
    if matches!(auth, BrokerAuth::None) {
        return false;
    }
    let is_crypto_sym = symbol.contains('/') || symbol.contains("USD");
    let payload = if let Some((tp, sl)) = bracket.filter(|_| !is_crypto_sym) {
        // Native Bracket Order for Equities (NYSE/NASDAQ/IEX)
        let qty = qty_or_notional.0.unwrap_or(1.0);
        if let Some(lp) = limit_price {
            format!(
                r#"{{"symbol":"{}","qty":"{:.2}","side":"{}","type":"limit","limit_price":"{:.2}","time_in_force":"day","order_class":"bracket","take_profit":{{"limit_price":"{:.2}"}},"stop_loss":{{"stop_price":"{:.2}"}}}}"#,
                symbol, qty, side, lp, tp, sl
            )
        } else {
            format!(
                r#"{{"symbol":"{}","qty":"{:.2}","side":"{}","type":"market","time_in_force":"gtc","order_class":"bracket","take_profit":{{"limit_price":"{:.2}"}},"stop_loss":{{"stop_price":"{:.2}"}}}}"#,
                symbol, qty, side, tp, sl
            )
        }
    } else if let Some(notional) = qty_or_notional.1 {
        // Aggressive Limit Order for Crypto: Protect against latency slippage using IOC (Immediate or Cancel)
        if let Some(lp) = limit_price {
            format!(
                r#"{{"symbol":"{}","notional":"{:.2}","side":"{}","type":"limit","limit_price":"{:.2}","time_in_force":"ioc"}}"#,
                symbol, notional, side, lp
            )
        } else {
            format!(
                r#"{{"symbol":"{}","notional":"{:.2}","side":"{}","type":"market","time_in_force":"gtc"}}"#,
                symbol, notional, side
            )
        }
    } else if let Some(qty) = qty_or_notional.0 {
        let qty_str = if is_crypto_sym {
            let s = format!("{:.8}", qty);
            let trimmed = s.trim_end_matches('0').trim_end_matches('.');
            if trimmed.is_empty() || trimmed == "0" {
                format!("{:.8}", qty)
            } else {
                trimmed.to_string()
            }
        } else {
            format!("{:.2}", qty)
        };
        if let Some(lp) = limit_price {
            format!(
                r#"{{"symbol":"{}","qty":"{}","side":"{}","type":"limit","limit_price":"{:.2}","time_in_force":"ioc"}}"#,
                symbol, qty_str, side, lp
            )
        } else {
            format!(
                r#"{{"symbol":"{}","qty":"{}","side":"{}","type":"market","time_in_force":"gtc"}}"#,
                symbol, qty_str, side
            )
        }
    } else {
        return false;
    };

    let mut req = client
        .post("https://paper-api.alpaca.markets/v2/orders")
        .header("Content-Type", "application/json")
        .body(payload);

    match auth {
        BrokerAuth::OAuth(token) => {
            req = req.header("Authorization", format!("Bearer {}", *token));
        }
        BrokerAuth::KeySecret { key, secret } => {
            req = req
                .header("APCA-API-KEY-ID", key.as_str())
                .header("APCA-API-SECRET-KEY", secret.as_str());
        }
        BrokerAuth::None => return false,
    }

    match req.send().await {
        Ok(resp) => {
            if let Ok(text) = resp.text().await {
                if text.contains(r#""id""#) {
                    println!(
                        " \x1b[1;35m[ALPACA BROKER ORDER]\x1b[0m Transaccion ejecutada en Broker: {} {}",
                        side.to_uppercase(),
                        symbol
                    );
                    true
                } else {
                    eprintln!(
                        " \x1b[1;31m[ALPACA BROKER RECHAZO]\x1b[0m Respuesta de Broker: {}",
                        text.trim()
                    );
                    false
                }
            } else {
                false
            }
        }
        Err(e) => {
            eprintln!(" \x1b[1;31m[ALPACA NETWORK ERROR]\x1b[0m Fallo HTTP al enviar orden: {:?}", e);
            false
        }
    }
}

/// Fetches whether Wall Street regular trading hours are currently open from Alpaca Clock API.
async fn fetch_alpaca_clock(
    client: &reqwest::Client,
    auth: &BrokerAuth,
) -> Option<bool> {
    let mut req = client.get("https://paper-api.alpaca.markets/v2/clock");
    match auth {
        BrokerAuth::OAuth(token) => {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        BrokerAuth::KeySecret { key, secret } => {
            req = req
                .header("APCA-API-KEY-ID", key.as_str())
                .header("APCA-API-SECRET-KEY", secret.as_str());
        }
        BrokerAuth::None => return None,
    }

    let resp = req.send().await.ok()?;
    let val: serde_json::Value = resp.json().await.ok()?;
    val.get("is_open").and_then(|v| v.as_bool())
}

/// Fetches the live account equity, cash, and buying power from Alpaca Paper Trading REST API.
async fn fetch_alpaca_account(
    client: &reqwest::Client,
    auth: &BrokerAuth,
) -> Option<(String, f64, f64, f64)> {
    let mut req = client.get("https://paper-api.alpaca.markets/v2/account");
    match auth {
        BrokerAuth::OAuth(token) => {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        BrokerAuth::KeySecret { key, secret } => {
            req = req
                .header("APCA-API-KEY-ID", key.as_str())
                .header("APCA-API-SECRET-KEY", secret.as_str());
        }
        BrokerAuth::None => return None,
    }

    let resp = req.send().await.ok()?;
    let val: serde_json::Value = resp.json().await.ok()?;
    let acc_num = val
        .get("account_number")
        .and_then(|v| v.as_str())
        .unwrap_or("PA3NGD2B8")
        .to_string();
    let equity: f64 = val
        .get("equity")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000.0);
    let cash: f64 = val
        .get("cash")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(equity);
    let bp: f64 = val
        .get("buying_power")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(equity * 4.0);
    Some((acc_num, equity, cash, bp))
}

/// Fetches existing open positions from Alpaca Paper Trading REST API.
async fn fetch_alpaca_positions(
    client: &reqwest::Client,
    auth: &BrokerAuth,
) -> HashMap<String, LiveBrokerPosition> {
    let mut positions = HashMap::new();
    let mut req = client.get("https://paper-api.alpaca.markets/v2/positions");
    match auth {
        BrokerAuth::OAuth(token) => {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        BrokerAuth::KeySecret { key, secret } => {
            req = req
                .header("APCA-API-KEY-ID", key.as_str())
                .header("APCA-API-SECRET-KEY", secret.as_str());
        }
        BrokerAuth::None => return positions,
    }

    if let Ok(resp) = req.send().await {
        if let Ok(val) = resp.json::<serde_json::Value>().await {
            if let Some(arr) = val.as_array() {
                for item in arr {
                    let sym_raw = item.get("symbol").and_then(|v| v.as_str()).unwrap_or("");
                    let qty: f64 = item
                        .get("qty")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0.0);
                    let avg_entry: f64 = item
                        .get("avg_entry_price")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0.0);
                    let mkt_val: f64 = item
                        .get("market_value")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0.0);
                    let side_str = item.get("side").and_then(|v| v.as_str()).unwrap_or("long");
                    let side_buy = side_str != "short";

                    let norm_sym = if sym_raw.ends_with("USD") && !sym_raw.contains('/') {
                        let base = &sym_raw[..sym_raw.len() - 3];
                        format!("{}/USD", base)
                    } else {
                        sym_raw.to_string()
                    };

                    if qty > 0.0 && avg_entry > 0.0 {
                        positions.insert(
                            norm_sym.clone(),
                            LiveBrokerPosition {
                                symbol: norm_sym,
                                side_buy,
                                entry_price: avg_entry,
                                qty,
                                notional: mkt_val.abs(),
                                entry_time: Instant::now(),
                            },
                        );
                    }
                }
            }
        }
    }
    positions
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("==========================================================================================");
    println!("               ENTROPIQ QUANTUM MICROSTRUCTURE LOCAL PERFORMANCE SUPERVISOR               ");
    println!("==========================================================================================");

    // DEF-01 fix: single source of truth for credentials
    let cfg = EnvConfig::load();

    let is_crypto = cfg.feed.to_lowercase() == "crypto";
    let (feed, live_symbols, tick_size) = if is_crypto {
        println!("[+] Canal de Mercado: Alpaca Cripto 24/7 (BTC/USD, ETH/USD, SOL/USD)");
        (AlpacaDataFeed::Crypto, vec!["BTC/USD", "ETH/USD", "SOL/USD"], 0.50)
    } else {
        println!("[+] Canal de Mercado: Alpaca Equities Wall Street IEX (Top 10 Stocks & ETFs)");
        (
            AlpacaDataFeed::Iex,
            vec!["SPY", "QQQ", "AAPL", "NVDA", "TSLA", "AMD", "MSFT", "AMZN", "META", "GOOGL"],
            0.01,
        )
    };

    cfg.log_auth_mode();
    let streamer_auth = if let Some(ref token) = cfg.oauth_token {
        Some(AlpacaAuth::OAuthToken(token.clone()))
    } else if let (Some(ref key), Some(ref secret)) = (&cfg.api_key, &cfg.api_secret) {
        Some(AlpacaAuth::KeySecret { key: key.clone(), secret: secret.clone() })
    } else {
        None
    };

    let broker_auth = if let Some(ref token) = cfg.oauth_token {
        BrokerAuth::OAuth(Arc::new(token.clone()))
    } else if let (Some(ref key), Some(ref secret)) = (&cfg.api_key, &cfg.api_secret) {
        BrokerAuth::KeySecret {
            key: Arc::new(key.clone()),
            secret: Arc::new(secret.clone()),
        }
    } else {
        BrokerAuth::None
    };

    let http_client = reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(10)
        .tcp_nodelay(true)
        .build()?;

    // Sync real account equity from Alpaca REST API
    let (initial_account_label, initial_equity, initial_buying_power) = if !matches!(broker_auth, BrokerAuth::None) {
        if let Some((acc, eq, _cash, bp)) = fetch_alpaca_account(&http_client, &broker_auth).await {
            println!(
                "[+] Sincronizacion Alpaca Exitosa: Cuenta {} (Paper) | Capital: ${:.2} USD | Poder Compra: ${:.2} USD",
                acc, eq, bp
            );
            (format!("{} (Paper)", acc), eq, bp)
        } else {
            ("Alpaca Paper".to_string(), 100_000.0, 400_000.0)
        }
    } else {
        ("Simulación Local".to_string(), 100_000.0, 100_000.0)
    };

    // Sync market clock status (Crypto is 24/7/365, Equities check NYSE clock)
    let initial_market_open = if is_crypto {
        true
    } else if !matches!(broker_auth, BrokerAuth::None) {
        fetch_alpaca_clock(&http_client, &broker_auth).await.unwrap_or(false)
    } else {
        true
    };
    let market_open_flag = Arc::new(AtomicBool::new(initial_market_open));
    if initial_market_open {
        if is_crypto {
            println!("[+] Estado Criptomercado: ABIERTO CONTINUO 24/7/365");
        } else {
            println!("[+] Estado Wall Street: ABIERTO (Rueda bursatil activa)");
        }
    } else {
        println!("[!] Estado Wall Street: CERRADO (Esperando apertura oficial 9:30 AM EST)");
    }

    let live_broker_info = Arc::new(std::sync::RwLock::new((
        initial_account_label.clone(),
        initial_equity,
        initial_buying_power,
    )));

    let running = Arc::new(AtomicBool::new(true));
    let r_clone = running.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("\n[!] Senal de parada recibida. Cerrando supervisor...");
        r_clone.store(false, Ordering::SeqCst);
    });

    // Background task to continuously re-sync real Alpaca account equity and clock every 15s
    if !matches!(broker_auth, BrokerAuth::None) {
        let client_bg = http_client.clone();
        let auth_bg = broker_auth.clone();
        let broker_info_bg = live_broker_info.clone();
        let market_open_bg = market_open_flag.clone();
        let running_bg = running.clone();
        tokio::spawn(async move {
            let mut sync_interval = tokio::time::interval(Duration::from_secs(15));
            while running_bg.load(Ordering::Relaxed) {
                sync_interval.tick().await;
                if let Some((acc, eq, _cash, bp)) = fetch_alpaca_account(&client_bg, &auth_bg).await {
                    if let Ok(mut lock) = broker_info_bg.write() {
                        *lock = (format!("{} (Paper)", acc), eq, bp);
                    }
                }
                if !is_crypto {
                    if let Some(is_open) = fetch_alpaca_clock(&client_bg, &auth_bg).await {
                        market_open_bg.store(is_open, Ordering::Relaxed);
                    }
                } else {
                    market_open_bg.store(true, Ordering::Relaxed);
                }
            }
        });
    }

    // Web Dashboard and Render Health-Check Server (Port 10000 / $PORT)
    let web_state = Arc::new(std::sync::RwLock::new(alpaca_streamer::WebDashboardState {
        market_open: initial_market_open,
        feed: cfg.feed.clone(),
        broker_account: initial_account_label.clone(),
        buying_power: initial_buying_power,
        current_equity: initial_equity,
        ..Default::default()
    }));
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(10000);
    let web_state_server = web_state.clone();
    tokio::spawn(async move {
        alpaca_streamer::start_web_server(web_state_server, port).await;
    });

    // CPU Pinning HFT: Pin main calculation pipeline to Core 0
    if alpaca_streamer::pin_current_thread_to_core(0) {
        println!("[+] Afinidad CPU HFT: Hilo principal (Motor Cuántico) fijado a Core 0");
    }

    // DEF-03 fix: 4096-element channel sending full QuantumAnalyticsSnapshot
    let (live_tx, mut live_rx) = mpsc::channel::<(String, QuantumAnalyticsSnapshot)>(4096);
    let alpaca_connected = Arc::new(AtomicBool::new(false));
    let alpaca_ticks_count = Arc::new(AtomicU64::new(0));
    let alpaca_ticks_dropped = Arc::new(AtomicU64::new(0));

    // 1. Spawns Alpaca WebSocket client in background task
    if let Some(auth) = streamer_auth {
        let conn_flag = alpaca_connected.clone();
        let ticks_flag = alpaca_ticks_count.clone();
        let dropped_flag = alpaca_ticks_dropped.clone();
        let syms = live_symbols.clone();

        tokio::spawn(async move {
            // CPU Pinning HFT: Pin WebSocket ingestion worker to Core 2
            if alpaca_streamer::pin_current_thread_to_core(2) {
                println!("[+] Afinidad CPU HFT: Hilo de red WebSocket fijado a Core 2");
            }

            let mut streamer = match auth {
                AlpacaAuth::OAuthToken(t) => AlpacaStreamer::with_oauth_token(t, feed, tick_size),
                AlpacaAuth::KeySecret { key, secret } => {
                    AlpacaStreamer::with_key_secret(key, secret, feed, tick_size)
                }
            };

            conn_flag.store(true, Ordering::SeqCst);
            let res = streamer
                .stream_with_symbol(syms, |sym, snap| {
                    ticks_flag.fetch_add(1, Ordering::Relaxed);
                    // DEF-03 fix: count silently dropped ticks on channel saturation
                    if live_tx.try_send((sym.to_string(), *snap)).is_err() {
                        dropped_flag.fetch_add(1, Ordering::Relaxed);
                    }
                })
                .await;

            if let Err(e) = res {
                eprintln!("[AlpacaStreamer Worker] Error en canal WebSocket: {:?}", e);
            }
            conn_flag.store(false, Ordering::SeqCst);
        });
    }

    // 2. Quantum Engine Telemetry, Smart Order Router, and Auditor
    let mut offline_engine = QuantumMarketEngine::new(0.01);
    let mut simulator = MicrostructureSimulator::new(500.0, 0.01, 1337);
    let mut auditor = PortfolioAuditor::new(initial_equity);

    let regimes = [
        MarketRegime::ThermalEquilibrium,
        MarketRegime::IcebergAbsorption,
        MarketRegime::QuantumBreakout,
        MarketRegime::LiquidityCavitation,
    ];
    let mut regime_idx = 0;
    let mut last_real_snapshot: Option<QuantumAnalyticsSnapshot> = None;
    let mut real_snapshots: HashMap<String, QuantumAnalyticsSnapshot> = HashMap::new();

    let mut total_processed_ticks: u64 = 0;
    let mut total_latency_ns: u64 = 0;
    let mut min_latency_ns: u64 = u64::MAX;
    let mut max_latency_ns: u64 = 0;

    // DEF-05 fix: Running P-square approximate quantile estimation (no sort needed).
    let mut p50_ema: f64 = 2_000.0;
    let mut p99_ema: f64 = 8_000.0;
    const EMA_ALPHA_P50: f64 = 0.001;
    const EMA_ALPHA_P99: f64 = 0.0005;

    let mut live_positions: HashMap<String, LiveBrokerPosition> = if !matches!(broker_auth, BrokerAuth::None) {
        let synced = fetch_alpaca_positions(&http_client, &broker_auth).await;
        if !synced.is_empty() {
            println!("[+] Posiciones Abiertas Sincronizadas del Broker ({} activas):", synced.len());
            for (s, p) in &synced {
                println!(
                    "    - {}: qty={:.6}, entrada=${:.2}, nocional=${:.2} | TP/SL activados",
                    s, p.qty, p.entry_price, p.notional
                );
            }
        }
        synced
    } else {
        HashMap::new()
    };
    let mut last_broker_close_time: HashMap<String, Instant> = HashMap::new();
    let mut signal_confirmations: HashMap<String, (Side, u32, Instant)> = HashMap::new();
    let mut current_market_bias: f64 = 0.0;
    let daily_start_equity: f64 = initial_equity;

    let start_time = Instant::now();
    let mut last_report_time = Instant::now();
    let mut last_report_ticks: u64 = 0;
    let mut interval = tokio::time::interval(Duration::from_millis(1));

    println!("[+] Supervisor iniciado. Monitorizando telemetria, memoria y pipeline pre-trade...");
    if is_crypto {
        println!("[+] Especializacion de Mercado: ALTO RENDIMIENTO CRIPTO 24/7 (BTC/USD, ETH/USD, SOL/USD)");
        println!("[+] Lead-Lag Cuantico: BTC/USD Benchmark Bias -> Arbitraje Estadistico en Altcoins");
    } else {
        println!("[+] Especializacion de Mercado: WALL STREET EQUITIES IEX (SPY, QQQ, NVDA, AAPL, TSLA...)");
        println!("[+] Lead-Lag Cuantico: SPY/QQQ Macro Index Bias -> Arbitraje Estadistico en Acciones");
    }
    println!("[+] Pipeline Pre-Trade: Spread < 15bps | Retorno > Costes | Delay WS < 50ms | Drawdown Diario < 2%");
    println!("[+] Risk Engine: Kelly Dinamico segun Stop-Loss (-0.20%) | TP=+0.45% (decay +0.25%) | Techo 1.5% Equity");
    println!("------------------------------------------------------------------------------------------\n");

    let is_live_mode = alpaca_connected.load(Ordering::Relaxed) || live_symbols.len() > 0;

    while running.load(Ordering::Relaxed) {
        // Step A: Process real market ticks from Alpaca WebSocket
        while let Ok((sym, snap)) = live_rx.try_recv() {
            let t_tick_start = Instant::now();
            total_processed_ticks += 1;
            last_real_snapshot = Some(snap);
            real_snapshots.insert(sym.clone(), snap);

            if sym == "SPY" || sym == "QQQ" || (is_crypto && sym.starts_with("BTC")) {
                let scale = if sym.starts_with("BTC") { 0.0001 } else { 0.00005 };
                current_market_bias = ((snap.micro_price - snap.mid_price) / (snap.mid_price * scale)).clamp(-1.0, 1.0);
            }

            println!(
                " \x1b[1;32m[ALPACA LIVE]\x1b[0m {:<7} | Mid: ${:<9.2} | Micro: ${:<9.2} | S: {:.3} | Coher: {:.3} | VPIN: {:.3} | Mach: {:.2} | Fase: {:?}",
                sym, snap.mid_price, snap.micro_price, snap.normalized_entropy, snap.quantum_coherence, snap.vpin, snap.fluid_metrics.mach_number, snap.active_phase
            );

            // A1. Position Monitoring for sym: Check Take-Profit, Stop-Loss, and Emergency Exit
            let pos_opt_data = live_positions.get(&sym).map(|pos| {
                (pos.side_buy, pos.entry_price, pos.qty, pos.notional)
            });

            if let Some((pos_side_buy, pos_entry_price, _pos_qty, pos_notional)) = pos_opt_data {
                let best_bid = if snap.best_bid > 0.0 { snap.best_bid } else { snap.mid_price - (snap.mid_price * 0.0001).max(0.01) };
                let best_ask = if snap.best_ask > 0.0 { snap.best_ask } else { snap.mid_price + (snap.mid_price * 0.0001).max(0.01) };
                let sor_intent = SmartOrderRouter::route_intent(&snap, best_bid, best_ask, 1.0);

                let price_diff = if pos_side_buy {
                    snap.mid_price - pos_entry_price
                } else {
                    pos_entry_price - snap.mid_price
                };
                let return_frac = price_diff / pos_entry_price.max(1.0);

                // Emergency Exit: SmartOrderRouter triggers EmergencyCancelAll on Cavitation shock (Mach >= 1.5) or Toxic flow (VPIN >= 0.70)
                if let OrderIntent::EmergencyCancelAll { reason } = sor_intent {
                    if close_alpaca_position(&http_client, &broker_auth, &sym).await {
                        let pnl_usd = return_frac * pos_notional;
                        auditor.record_trade(pnl_usd, return_frac);
                        println!(
                            " \x1b[1;31m[ALPACA EMERGENCY EXIT ({})]\x1b[0m Posicion liquidada en {}: {:.2}% | PnL: ${:.3} USD",
                            reason, sym, return_frac * 100.0, pnl_usd
                        );
                        live_positions.remove(&sym);
                        last_broker_close_time.insert(sym.clone(), Instant::now());
                    }
                // 1. Take-Profit (+0.45% target gain, or +0.25% if coherence decays below 0.20)
                } else if return_frac >= 0.0045 || (return_frac >= 0.0025 && snap.quantum_coherence < 0.20) {
                    if close_alpaca_position(&http_client, &broker_auth, &sym).await {
                        let profit_usd = return_frac * pos_notional;
                        auditor.record_trade(profit_usd, return_frac);
                        println!(
                            " \x1b[1;32m[ALPACA TAKE-PROFIT]\x1b[0m Posicion liquidada en {}: +{:.2}% | Ganancia: +${:.2} USD",
                            sym, return_frac * 100.0, profit_usd
                        );
                        live_positions.remove(&sym);
                        last_broker_close_time.insert(sym.clone(), Instant::now());
                    }
                // 2. Stop-Loss (-0.20% hard risk cutoff)
                } else if return_frac <= -0.0020 {
                    if close_alpaca_position(&http_client, &broker_auth, &sym).await {
                        let loss_usd = return_frac * pos_notional;
                        auditor.record_trade(loss_usd, return_frac);
                        println!(
                            " \x1b[1;31m[ALPACA STOP-LOSS]\x1b[0m Riesgo cortado en {}: {:.2}% | Perdida: -${:.2} USD",
                            sym, return_frac * 100.0, loss_usd.abs()
                        );
                        live_positions.remove(&sym);
                        last_broker_close_time.insert(sym.clone(), Instant::now());
                    }
                }
            } else if is_crypto || (sym != "SPY" && sym != "QQQ") {
                // A2. No open position -> Evaluate Entry Criteria via 6-Stage Pre-Trade Pipeline
                let is_market_open = market_open_flag.load(Ordering::Relaxed);
                let can_enter = is_market_open && last_broker_close_time
                    .get(&sym)
                    .map(|t| t.elapsed() >= Duration::from_secs(45))
                    .unwrap_or(true);

                if can_enter {
                    let best_bid = if snap.best_bid > 0.0 { snap.best_bid } else { snap.mid_price - (snap.mid_price * 0.0001).max(0.01) };
                    let best_ask = if snap.best_ask > 0.0 { snap.best_ask } else { snap.mid_price + (snap.mid_price * 0.0001).max(0.01) };
                    let kelly_f = auditor.kelly_leverage(snap.quantum_coherence).clamp(0.2, 3.0);
                    let sor_intent = SmartOrderRouter::route_intent(&snap, best_bid, best_ask, kelly_f);

                    match sor_intent {
                        OrderIntent::AggressiveTaker { side, size_ratio, urgency: _ } => {
                            let is_crypto_sym = sym.contains('/') || sym.contains("USD");

                            // Filter 1: Spread Bid-Ask limit check (< 15 bps crypto, < 8 bps equities)
                            let spread_bps = if snap.mid_price > 0.0 {
                                ((best_ask - best_bid).max(0.0) / snap.mid_price) * 10_000.0
                            } else {
                                999.0
                            };
                            let max_spread_bps = if is_crypto_sym { 15.0 } else { 8.0 };
                            let pass_spread = spread_bps <= max_spread_bps;

                            // Filter 2: Net Return covers fees + slippage (Target TP 45 bps - roundtrip friction >= 10 bps margin)
                            let roundtrip_friction = if snap.mid_price > 0.0 {
                                ((best_ask - best_bid).max(0.0) / snap.mid_price) + if is_crypto_sym { 0.0020 } else { 0.0004 }
                            } else {
                                0.01
                            };
                            let expected_tp = 0.0045;
                            let net_expected_return = expected_tp - roundtrip_friction;
                            let pass_cost = net_expected_return >= 0.0010;

                            // Filter 3: WebSocket data age / latency check (< 50 ms)
                            let now_ns = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_nanos() as u64;
                            let pass_latency = if snap.timestamp_ns > 0 {
                                let delay_ms = (now_ns.saturating_sub(snap.timestamp_ns)) as f64 / 1_000_000.0;
                                delay_ms <= 50.0
                            } else {
                                true
                            };

                            // Anti-whipsaw confirmation window: require 2 consecutive signals or strong conviction
                            let conf_entry = signal_confirmations.entry(sym.clone()).or_insert((side, 0, Instant::now()));
                            if conf_entry.0 == side && conf_entry.2.elapsed() < Duration::from_secs(5) {
                                conf_entry.1 += 1;
                                conf_entry.2 = Instant::now();
                            } else {
                                *conf_entry = (side, 1, Instant::now());
                            }

                            let is_confirmed = conf_entry.1 >= 2 || (snap.phase_confidence >= 0.88 && snap.quantum_coherence >= 0.38);

                            let is_macro_aligned = if side == Side::Buy {
                                current_market_bias >= -0.20
                            } else {
                                current_market_bias <= 0.20
                            };

                            // Filter 5: Daily Drawdown Circuit Breaker (< 2.0% max loss from daily start equity)
                            let daily_loss_pct = if daily_start_equity > 0.0 {
                                (daily_start_equity - auditor.current_equity) / daily_start_equity
                            } else {
                                0.0
                            };
                            let pass_drawdown = daily_loss_pct < 0.02;

                            if !pass_drawdown {
                                println!(
                                    " \x1b[1;31m[CIRCUIT BREAKER ACTIVADO]\x1b[0m Drawdown diario ({:.2}%) >= 2.0%. Bloqueando nuevas entradas.",
                                    daily_loss_pct * 100.0
                                );
                            }

                            if pass_spread && pass_cost && pass_latency && pass_drawdown && is_confirmed && is_macro_aligned {
                                signal_confirmations.remove(&sym);
                                let side_buy = side == Side::Buy;
                                let order_side_str = if side_buy { "buy" } else { "sell" };

                                // Filter 4: Risk Engine size calculation according to Stop-Loss (-0.20%) and Kelly leverage
                                let live_eq = auditor.current_equity.max(1000.0);
                                let base_notional = live_eq * 0.0075;
                                let max_notional = live_eq * 0.015;
                                let notional = (base_notional * kelly_f * size_ratio).min(max_notional).max(50.0);
                                let notional = (notional * 100.0).round() / 100.0;
                                let qty = if is_crypto_sym {
                                    let raw = notional / snap.mid_price;
                                    ((raw * 10_000_000.0).round() / 10_000_000.0).max(0.0000001)
                                } else {
                                    let q = ((notional / snap.mid_price) * 100.0).round() / 100.0;
                                    q.max(0.01)
                                };

                                // Slippage Cap: 8 bps on crypto, 4 bps on equities to prevent latency penalty
                                let slippage_cap = if is_crypto_sym { 0.0008 } else { 0.0004 };
                                let limit_price = if side_buy {
                                    snap.mid_price * (1.0 + slippage_cap)
                                } else {
                                    snap.mid_price * (1.0 - slippage_cap)
                                };
                                let limit_price_clamped = (limit_price * 100.0).round() / 100.0;

                                // Filter 6: Execution (Aggressive Limit Order with Slippage Cap)
                                let bracket_opt = if !is_crypto_sym {
                                    let tp = if side_buy { snap.mid_price * 1.0045 } else { snap.mid_price * (1.0 - 0.0045) };
                                    let sl = if side_buy { snap.mid_price * (1.0 - 0.0020) } else { snap.mid_price * 1.0020 };
                                    Some((((tp * 100.0).round() / 100.0), ((sl * 100.0).round() / 100.0)))
                                } else {
                                    None
                                };

                                let order_arg = if is_crypto_sym {
                                    (None, Some(notional))
                                } else {
                                    (Some(qty), None)
                                };

                                if execute_alpaca_paper_order(
                                    &http_client,
                                    &broker_auth,
                                    &sym,
                                    order_side_str,
                                    order_arg,
                                    Some(limit_price_clamped),
                                    bracket_opt,
                                ).await {
                                    println!(
                                        " \x1b[1;36m[ALPACA ENTRADA {} (SOR: {:?})]\x1b[0m {} a ${:.2} (Limit: ${:.2}, qty: {:.6}) | Notional: ${:.2} (Kelly: {:.2}x) | VPIN: {:.3} | Coher: {:.3} | S: {:.3}",
                                        order_side_str.to_uppercase(), snap.active_phase, sym, snap.mid_price, limit_price_clamped, qty, notional, kelly_f, snap.vpin, snap.quantum_coherence, snap.normalized_entropy
                                    );
                                    live_positions.insert(
                                        sym.clone(),
                                        LiveBrokerPosition {
                                            symbol: sym.clone(),
                                            side_buy,
                                            entry_price: snap.mid_price,
                                            qty,
                                            notional,
                                            entry_time: Instant::now(),
                                        },
                                    );
                                }
                            }
                        }
                        OrderIntent::EmergencyCancelAll { reason } => {
                            signal_confirmations.remove(&sym);
                            if snap.active_phase == MarketQuantumPhase::LiquidityCavitation {
                                println!(
                                    " \x1b[1;33m[SOR REPOSO]\x1b[0m Neutralizacion en {}: {} | Mach: {:.2} | S: {:.3}",
                                    sym, reason, snap.fluid_metrics.mach_number, snap.normalized_entropy
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }

            let elapsed_ns = t_tick_start.elapsed().as_nanos() as u64;
            total_latency_ns += elapsed_ns;
            if elapsed_ns < min_latency_ns { min_latency_ns = elapsed_ns; }
            if elapsed_ns > max_latency_ns { max_latency_ns = elapsed_ns; }
            let elapsed_f = elapsed_ns as f64;
            if elapsed_f <= p50_ema { p50_ema -= EMA_ALPHA_P50 * p50_ema * 0.5; } else { p50_ema += EMA_ALPHA_P50 * p50_ema; }
            if elapsed_f <= p99_ema { p99_ema -= EMA_ALPHA_P99 * p99_ema * 0.01; } else { p99_ema += EMA_ALPHA_P99 * p99_ema; }
        }

        // Step B: Offline benchmark simulator ONLY when running without real data stream
        // When Alpaca is connected, the quantum engine is 100% fed by real market data, completely isolated from synthetic ticks.
        if !is_live_mode {
            let current_regime = regimes[regime_idx % regimes.len()];
            if total_processed_ticks % 10_000 == 0 && total_processed_ticks > 0 {
                regime_idx += 1;
            }

            let (quote, trade_opt) = simulator.next_tick(current_regime);
            let t_start = Instant::now();
            let snap_opt = offline_engine.on_quote(quote);
            if let Some(trade) = trade_opt {
                offline_engine.on_trade(trade);
            }
            let elapsed_ns = t_start.elapsed().as_nanos() as u64;

            total_processed_ticks += 1;
            total_latency_ns += elapsed_ns;
            if elapsed_ns < min_latency_ns { min_latency_ns = elapsed_ns; }
            if elapsed_ns > max_latency_ns { max_latency_ns = elapsed_ns; }

            if let Some(snap) = snap_opt {
                last_real_snapshot = Some(snap);
            }
        }

        // Step C: Periodic supervisor telemetry display (every 2.0 seconds)
        let now = Instant::now();
        let report_elapsed = now.duration_since(last_report_time);
        if report_elapsed >= Duration::from_secs(2) {
            let delta_ticks = total_processed_ticks - last_report_ticks;
            let throughput = delta_ticks as f64 / report_elapsed.as_secs_f64();
            let avg_latency = if delta_ticks > 0 {
                total_latency_ns as f64 / total_processed_ticks as f64
            } else {
                0.0
            };

            let p50 = p50_ema as u64;
            let p99 = p99_ema as u64;

            let rss_mb = get_memory_rss_mb();
            let is_alpaca_up = alpaca_connected.load(Ordering::Relaxed);
            let alpaca_total = alpaca_ticks_count.load(Ordering::Relaxed);
            let alpaca_dropped = alpaca_ticks_dropped.load(Ordering::Relaxed);

            let (entropy, coherence, mach, phase, conf, vpin, is_toxic) = if let Some(s) = last_real_snapshot {
                (s.normalized_entropy, s.quantum_coherence, s.fluid_metrics.mach_number, s.active_phase, s.phase_confidence, s.vpin, s.is_toxic_flow)
            } else {
                (1.0, 0.0, 0.0, MarketQuantumPhase::ThermalNoise, 0.5, 0.0, false)
            };

            println!("┌────────────────────────────────────────────────────────────────────────────────────────┐");
            println!(
                "│ \x1b[1;36mENTROPIQ MONITOR\x1b[0m | WS: {:<12} | Ticks: {:<10} | Drops: {:<10}       │",
                if is_alpaca_up { "\x1b[1;32mCONECTADO\x1b[0m" } else { "\x1b[1;31mDESCONECTADO\x1b[0m" },
                alpaca_total,
                alpaca_dropped
            );
            println!("├────────────────────────────────────────────────────────────────────────────────────────┤");
            println!(
                "│ Rendimiento:     {:<10.0} ticks/s  | Total Ticks Proc: {:<12} | RSS RAM: {:>6.2} MB │",
                throughput, total_processed_ticks, rss_mb
            );
            println!(
                "│ Latencia Tick:   Prom: {:>6.0} ns | Min: {:>6.0} ns | p50: {:>6.0} ns | p99: {:>6.0} ns │",
                avg_latency, min_latency_ns, p50, p99
            );
            println!(
                "│ Estado Cuantico: Entropia S: {:>5.3} | Coherencia: {:>5.3} | Mach Fluidico: {:>6.2}   │",
                entropy, coherence, mach
            );
            println!(
                "│ Flujo Toxico:    VPIN: {:>5.3}       | Escudo Toxico: {:<12} | Lead-Lag SPY: {:>6.2}  │",
                vpin,
                if is_toxic || vpin >= 0.70 { "\x1b[1;31mBLOQUEO\x1b[0m" } else { "\x1b[1;32mPERMISIVO\x1b[0m" },
                current_market_bias
            );
            println!(
                "│ Fase de Mercado: \x1b[1;33m{:<24?}\x1b[0m (Confianza: {:>5.1}%)                     │",
                phase, conf * 100.0
            );
            println!(
                "│ Auditor PnL:     Equity: ${:<10.2} | Sharpe: {:>5.2} | WinRate: {:>5.1}% | Kelly: {:>4.2}x │",
                auditor.current_equity,
                auditor.sharpe_ratio(252.0),
                auditor.win_rate() * 100.0,
                auditor.kelly_leverage(coherence)
            );
            println!(
                "│ Metricas Riesgo: ProfitFactor: {:>5.2} | MaxDD: {:>5.2}% | Trades: {:<14}           │",
                auditor.profit_factor(),
                auditor.max_drawdown * 100.0,
                auditor.total_trades
            );
            println!("└────────────────────────────────────────────────────────────────────────────────────────┘\n");

            let (broker_acc_str, live_equity_val, live_bp_val) = live_broker_info
                .read()
                .map(|g| (g.0.clone(), g.1, g.2))
                .unwrap_or((initial_account_label.clone(), auditor.current_equity, 100_000.0));

            // Sync equity if updated from broker
            if (auditor.current_equity - live_equity_val).abs() > 0.001 && auditor.total_trades == 0 {
                auditor.current_equity = live_equity_val;
            }

            // Update web state for live Render HTTP dashboard and health checks
            if let Ok(mut w) = web_state.write() {
                *w = alpaca_streamer::WebDashboardState {
                    uptime_secs: start_time.elapsed().as_secs(),
                    websocket_connected: is_alpaca_up,
                    alpaca_ticks_received: alpaca_total,
                    alpaca_ticks_dropped: alpaca_dropped,
                    total_processed_ticks,
                    throughput_ticks_per_sec: throughput,
                    rss_mb,
                    avg_latency_ns: avg_latency,
                    min_latency_ns,
                    p50_latency_ns: p50,
                    p99_latency_ns: p99,
                    entropy,
                    coherence,
                    mach_number: mach,
                    market_phase: format!("{:?}", phase),
                    phase_confidence: conf,
                    vpin,
                    toxic_flow: is_toxic || vpin >= 0.70,
                    lead_lag_spy_bias: current_market_bias,
                    market_open: market_open_flag.load(Ordering::Relaxed),
                    feed: cfg.feed.clone(),
                    broker_account: broker_acc_str,
                    buying_power: live_bp_val,
                    current_equity: auditor.current_equity,
                    sharpe_ratio: auditor.sharpe_ratio(252.0),
                    win_rate: auditor.win_rate(),
                    kelly_leverage: auditor.kelly_leverage(coherence),
                    profit_factor: auditor.profit_factor(),
                    max_drawdown: auditor.max_drawdown,
                    total_trades: auditor.total_trades,
                };
            }

            last_report_time = now;
            last_report_ticks = total_processed_ticks;
        }

        interval.tick().await;
    }

    println!("[+] Supervisor detenido de forma segura.");
    Ok(())
}
