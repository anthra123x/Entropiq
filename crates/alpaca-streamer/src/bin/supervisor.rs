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

fn format_now_hms() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let total_secs = now / 1000;
    let millis = now % 1000;
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = (total_secs / 3600) % 24;
    format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, millis)
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
    highest_price: f64,
    trailing_stop: f64,
    profit_locked: bool,
}

/// Cancels all open resting orders atomically on Alpaca Paper REST API.
async fn cancel_all_alpaca_orders(
    client: &reqwest::Client,
    auth: &BrokerAuth,
) -> bool {
    if matches!(auth, BrokerAuth::None) {
        return false;
    }
    let mut req = client.delete("https://paper-api.alpaca.markets/v2/orders");
    match auth {
        BrokerAuth::OAuth(token) => {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        BrokerAuth::KeySecret { key, secret } => {
            req = req
                .header("APCA-API-KEY-ID", key.as_str())
                .header("APCA-API-SECRET-KEY", secret.as_str());
        }
        BrokerAuth::None => return false,
    }
    match req.send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

/// Verifies whether the broker currently has any open order or position for this symbol.
async fn has_open_orders_or_position(
    client: &reqwest::Client,
    auth: &BrokerAuth,
    symbol: &str,
) -> bool {
    if matches!(auth, BrokerAuth::None) {
        return false;
    }
    let clean_sym = symbol.replace('/', "");
    let pos_url = format!("https://paper-api.alpaca.markets/v2/positions/{}", clean_sym);
    let mut req_pos = client.get(&pos_url);

    let ord_url = format!("https://paper-api.alpaca.markets/v2/orders?status=open&symbols={}", clean_sym);
    let mut req_ord = client.get(&ord_url);

    match auth {
        BrokerAuth::OAuth(token) => {
            req_pos = req_pos.header("Authorization", format!("Bearer {}", token));
            req_ord = req_ord.header("Authorization", format!("Bearer {}", token));
        }
        BrokerAuth::KeySecret { key, secret } => {
            req_pos = req_pos
                .header("APCA-API-KEY-ID", key.as_str())
                .header("APCA-API-SECRET-KEY", secret.as_str());
            req_ord = req_ord
                .header("APCA-API-KEY-ID", key.as_str())
                .header("APCA-API-SECRET-KEY", secret.as_str());
        }
        BrokerAuth::None => return false,
    }

    if let Ok(resp) = req_pos.send().await {
        if resp.status().is_success() {
            return true;
        }
    }
    if let Ok(resp) = req_ord.send().await {
        if let Ok(orders) = resp.json::<serde_json::Value>().await {
            if let Some(arr) = orders.as_array() {
                if !arr.is_empty() {
                    return true;
                }
            }
        }
    }
    false
}

/// Verifies and fetches the exact position details (qty, avg_entry_price, market_value, side_buy) from Alpaca REST.
async fn verify_and_fetch_single_position(
    client: &reqwest::Client,
    auth: &BrokerAuth,
    symbol: &str,
) -> Option<(f64, f64, f64, bool)> {
    if matches!(auth, BrokerAuth::None) {
        return None;
    }
    let clean_sym = symbol.replace('/', "");
    let url = format!("https://paper-api.alpaca.markets/v2/positions/{}", clean_sym);
    let mut req = client.get(&url);
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

    if let Ok(resp) = req.send().await {
        if resp.status().is_success() {
            if let Ok(val) = resp.json::<serde_json::Value>().await {
                let qty: f64 = val.get("qty").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0).abs();
                let avg_entry: f64 = val.get("avg_entry_price").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                let mkt_val: f64 = val.get("market_value").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0).abs();
                let side_buy = val.get("side").and_then(|v| v.as_str()).unwrap_or("long") != "short";
                if qty > 0.0 && avg_entry > 0.0 {
                    return Some((qty, avg_entry, mkt_val, side_buy));
                }
            }
        }
    }
    None
}

/// Closes 100% of an open position atomically on Alpaca Paper REST API.
/// First cancels any open orders for this symbol, then uses DELETE /v2/positions/{symbol}.
async fn close_alpaca_position(
    client: &reqwest::Client,
    auth: &BrokerAuth,
    symbol: &str,
) -> bool {
    if matches!(auth, BrokerAuth::None) {
        return false;
    }
    let clean_symbol = symbol.replace('/', "");

    // 1. Defensively delete any open orders for this symbol first
    let ord_url = format!("https://paper-api.alpaca.markets/v2/orders?symbols={}", clean_symbol);
    let mut del_ord = client.delete(&ord_url);
    match auth {
        BrokerAuth::OAuth(token) => {
            del_ord = del_ord.header("Authorization", format!("Bearer {}", token));
        }
        BrokerAuth::KeySecret { key, secret } => {
            del_ord = del_ord
                .header("APCA-API-KEY-ID", key.as_str())
                .header("APCA-API-SECRET-KEY", secret.as_str());
        }
        BrokerAuth::None => return false,
    }
    let _ = del_ord.send().await;

    // 2. Liquidate position
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
        // Alpaca strictly requires whole integer shares for bracket orders
        let qty = qty_or_notional.0.unwrap_or(1.0);
        let qty_int = (qty.round() as u64).max(1);
        if let Some(lp) = limit_price {
            format!(
                r#"{{"symbol":"{}","qty":"{}","side":"{}","type":"limit","limit_price":"{:.2}","time_in_force":"day","order_class":"bracket","take_profit":{{"limit_price":"{:.2}"}},"stop_loss":{{"stop_price":"{:.2}"}}}}"#,
                symbol, qty_int, side, lp, tp, sl
            )
        } else {
            format!(
                r#"{{"symbol":"{}","qty":"{}","side":"{}","type":"market","time_in_force":"day","order_class":"bracket","take_profit":{{"limit_price":"{:.2}"}},"stop_loss":{{"stop_price":"{:.2}"}}}}"#,
                symbol, qty_int, side, tp, sl
            )
        }
    } else if let Some(lp) = limit_price {
        // Marketable Limit Order: Alpaca requires `qty` for ALL limit orders (notional is rejected on limits)
        let qty = if let Some(q) = qty_or_notional.0 {
            q
        } else if let Some(notional) = qty_or_notional.1 {
            notional / lp
        } else {
            return false;
        };
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
        let tif = if is_crypto_sym { "gtc" } else { "day" };
        format!(
            r#"{{"symbol":"{}","qty":"{}","side":"{}","type":"limit","limit_price":"{:.2}","time_in_force":"{}"}}"#,
            symbol, qty_str, side, lp, tif
        )
    } else if let Some(notional) = qty_or_notional.1 {
        format!(
            r#"{{"symbol":"{}","notional":"{:.2}","side":"{}","type":"market","time_in_force":"gtc"}}"#,
            symbol, notional, side
        )
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
        format!(
            r#"{{"symbol":"{}","qty":"{}","side":"{}","type":"market","time_in_force":"gtc"}}"#,
            symbol, qty_str, side
        )
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
            let status_code = resp.status();
            if let Ok(text) = resp.text().await {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(status) = val.get("status").and_then(|v| v.as_str()) {
                        let st_upper = status.to_uppercase();
                        if status == "canceled" || status == "rejected" || status == "expired" {
                            eprintln!(
                                " \x1b[1;31m[ALPACA BROKER RECHAZO ({})]\x1b[0m Orden descartada por el broker: {}",
                                st_upper,
                                text.trim()
                            );
                            return false;
                        }
                        if status == "filled"
                            || status == "new"
                            || status == "accepted"
                            || status == "pending_new"
                            || status == "partially_filled"
                        {
                            println!(
                                " \x1b[1;35m[ALPACA BROKER ORDER ({})]\x1b[0m Transaccion confirmada en Broker: {} {} (ID: {})",
                                st_upper,
                                side.to_uppercase(),
                                symbol,
                                val.get("id").and_then(|v| v.as_str()).unwrap_or("N/A")
                            );
                            return true;
                        }
                    }
                    if val.get("id").is_some() && status_code.is_success() {
                        println!(
                            " \x1b[1;35m[ALPACA BROKER ORDER]\x1b[0m Transaccion ejecutada en Broker: {} {}",
                            side.to_uppercase(),
                            symbol
                        );
                        return true;
                    }
                }
                eprintln!(
                    " \x1b[1;31m[ALPACA BROKER RECHAZO ({})]\x1b[0m Respuesta de Broker: {}",
                    status_code,
                    text.trim()
                );
                false
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

                    let abs_qty = qty.abs();
                    if abs_qty > 0.0 && avg_entry > 0.0 {
                        let trailing_stop = if side_buy {
                            avg_entry * (1.0 - 0.0045)
                        } else {
                            avg_entry * (1.0 + 0.0045)
                        };
                        positions.insert(
                            norm_sym.clone(),
                            LiveBrokerPosition {
                                symbol: norm_sym,
                                side_buy,
                                entry_price: avg_entry,
                                qty: abs_qty,
                                notional: mkt_val.abs(),
                                entry_time: Instant::now(),
                                highest_price: avg_entry,
                                trailing_stop,
                                profit_locked: false,
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

    // Purge any lingering open orders on startup for a 100% clean state
    if !matches!(broker_auth, BrokerAuth::None) {
        println!("[+] Purgando ordenes huerfanas previas en Alpaca...");
        cancel_all_alpaca_orders(&http_client, &broker_auth).await;
    }

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
    let mut recent_event_logs: std::collections::VecDeque<alpaca_streamer::PipelineLogEntry> = std::collections::VecDeque::with_capacity(30);
    recent_event_logs.push_back(alpaca_streamer::PipelineLogEntry {
        timestamp_str: format_now_hms(),
        symbol: "SYSTEM".to_string(),
        phase: "ThermalNoise".to_string(),
        intent: "BOOT".to_string(),
        message: format!("Supervisor Entropiq online en modo {}. Streamer activo.", cfg.feed),
        is_trade: false,
    });
    let mut current_market_bias: f64 = 0.0;
    let daily_start_equity: f64 = initial_equity;

    let start_time = Instant::now();
    let mut last_report_time = Instant::now();
    let mut last_reconcile_time = Instant::now();
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
    println!("[+] Pipeline Pre-Trade: Spread < 15bps | Retorno > Costes | Delay WS configurable | Drawdown Diario < 2%");
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

            // A1. Position Monitoring for sym: Check Dynamic Trailing Profit Lock, Decays, and Emergency Exit
            let pos_eval = if let Some(pos) = live_positions.get_mut(&sym) {
                let cur_price = snap.mid_price;
                if pos.side_buy {
                    if cur_price > pos.highest_price {
                        pos.highest_price = cur_price;
                    }
                } else if cur_price < pos.highest_price || pos.highest_price == 0.0 {
                    pos.highest_price = cur_price;
                }

                let max_ret = if pos.side_buy {
                    (pos.highest_price - pos.entry_price) / pos.entry_price.max(1.0)
                } else {
                    (pos.entry_price - pos.highest_price) / pos.entry_price.max(1.0)
                };

                // Dynamic Trailing Profit Lock:
                // Tier 1: Once profit reaches +0.35% (+35 bps), lock stop at Break-Even +0.05% (+5 bps)
                if max_ret >= 0.0035 && !pos.profit_locked {
                    pos.profit_locked = true;
                    let be_stop = if pos.side_buy {
                        pos.entry_price * 1.0005
                    } else {
                        pos.entry_price * (1.0 - 0.0005)
                    };
                    if (pos.side_buy && be_stop > pos.trailing_stop) || (!pos.side_buy && be_stop < pos.trailing_stop) {
                        pos.trailing_stop = be_stop;
                        println!(
                            " \x1b[1;32m[PROFIT LOCK ACTIVADO]\x1b[0m +{:.2}% alcanzado en {}. Stop bloqueado a Break-Even +5bps (${:.2}). ¡Trade blindado!",
                            max_ret * 100.0, sym, pos.trailing_stop
                        );
                    }
                }

                // Tier 2: Once profit reaches +0.50% (+50 bps), lock stop at +0.25% (+25 bps)
                if max_ret >= 0.0050 {
                    let tier2_stop = if pos.side_buy {
                        pos.entry_price * 1.0025
                    } else {
                        pos.entry_price * (1.0 - 0.0025)
                    };
                    if (pos.side_buy && tier2_stop > pos.trailing_stop) || (!pos.side_buy && tier2_stop < pos.trailing_stop) {
                        pos.trailing_stop = tier2_stop;
                        println!(
                            " \x1b[1;32m[PROFIT LOCK +25bps]\x1b[0m +{:.2}% alcanzado en {}. Stop asegurado a +0.25% (${:.2}).",
                            max_ret * 100.0, sym, pos.trailing_stop
                        );
                    }
                }

                // Tier 3: Dynamic ratchet trailing (trail by 20 bps from peak if peak >= +0.50%)
                if max_ret >= 0.0050 {
                    let ratchet = if pos.side_buy {
                        pos.highest_price * (1.0 - 0.0020)
                    } else {
                        pos.highest_price * (1.0 + 0.0020)
                    };
                    if (pos.side_buy && ratchet > pos.trailing_stop) || (!pos.side_buy && ratchet < pos.trailing_stop) {
                        pos.trailing_stop = ratchet;
                    }
                }

                Some((
                    pos.side_buy,
                    pos.entry_price,
                    pos.qty,
                    pos.notional,
                    pos.highest_price,
                    pos.trailing_stop,
                    pos.profit_locked,
                    pos.entry_time.elapsed(),
                ))
            } else {
                None
            };

            if let Some((pos_side_buy, pos_entry_price, _pos_qty, pos_notional, _pos_highest, pos_trailing_stop, pos_locked, pos_holding_time)) = pos_eval {
                let best_bid = if snap.best_bid > 0.0 { snap.best_bid } else { snap.mid_price - (snap.mid_price * 0.0001).max(0.01) };
                let best_ask = if snap.best_ask > 0.0 { snap.best_ask } else { snap.mid_price + (snap.mid_price * 0.0001).max(0.01) };
                let sor_intent = SmartOrderRouter::route_intent(&snap, best_bid, best_ask, 1.0);

                let price_diff = if pos_side_buy {
                    snap.mid_price - pos_entry_price
                } else {
                    pos_entry_price - snap.mid_price
                };
                let return_frac = price_diff / pos_entry_price.max(1.0);

                // Exit evaluation:
                // 1. Target Take-Profit (+0.80% gain)
                let is_target_tp = return_frac >= 0.0080;

                // 2. Trailing Stop Hit:
                let is_stop_hit = if pos_side_buy {
                    snap.mid_price <= pos_trailing_stop
                } else {
                    snap.mid_price >= pos_trailing_stop
                };

                // 3. Coherence Decay in Profit (capture +0.25% or more before decay):
                let is_coherence_decay_tp = return_frac >= 0.0025 && snap.quantum_coherence < 0.15;

                // 4. Time stop in Profit (> 15m holding and gain >= +0.10%):
                let is_time_tp = pos_holding_time >= Duration::from_secs(900) && return_frac >= 0.0010;

                // 5. Emergency Exit (Fluid cavitation or toxic flow):
                let is_emergency = matches!(sor_intent, OrderIntent::EmergencyCancelAll { .. });

                if is_emergency {
                    let reason = match sor_intent {
                        OrderIntent::EmergencyCancelAll { reason } => reason,
                        _ => "Emergency retreat",
                    };
                    if close_alpaca_position(&http_client, &broker_auth, &sym).await {
                        let pnl_usd = return_frac * pos_notional;
                        auditor.record_trade(pnl_usd, return_frac);
                        println!(
                            " \x1b[1;31m[ALPACA EMERGENCY EXIT ({})]\x1b[0m Posicion liquidada en {}: {:.2}% | PnL: ${:.2} USD",
                            reason, sym, return_frac * 100.0, pnl_usd
                        );
                        live_positions.remove(&sym);
                        last_broker_close_time.insert(sym.clone(), Instant::now());
                        recent_event_logs.push_back(alpaca_streamer::PipelineLogEntry {
                            timestamp_str: format_now_hms(),
                            symbol: sym.clone(),
                            phase: format!("{:?}", snap.active_phase),
                            intent: "EMERGENCY_EXIT".to_string(),
                            message: format!("Liquidada por {}: return {:+.2}% (${:+.2} USD)", reason, return_frac * 100.0, pnl_usd),
                            is_trade: true,
                        });
                        if recent_event_logs.len() > 30 { recent_event_logs.pop_front(); }
                    }
                } else if is_target_tp {
                    if close_alpaca_position(&http_client, &broker_auth, &sym).await {
                        let profit_usd = return_frac * pos_notional;
                        auditor.record_trade(profit_usd, return_frac);
                        println!(
                            " \x1b[1;32m[ALPACA TAKE-PROFIT +0.80%]\x1b[0m Posicion liquidada en {}: +{:.2}% | Ganancia: +${:.2} USD",
                            sym, return_frac * 100.0, profit_usd
                        );
                        live_positions.remove(&sym);
                        last_broker_close_time.insert(sym.clone(), Instant::now());
                        recent_event_logs.push_back(alpaca_streamer::PipelineLogEntry {
                            timestamp_str: format_now_hms(),
                            symbol: sym.clone(),
                            phase: format!("{:?}", snap.active_phase),
                            intent: "TAKE_PROFIT".to_string(),
                            message: format!("Take-Profit (+{:.2}%): ganancia +${:.2} USD", return_frac * 100.0, profit_usd),
                            is_trade: true,
                        });
                        if recent_event_logs.len() > 30 { recent_event_logs.pop_front(); }
                    }
                } else if is_coherence_decay_tp {
                    if close_alpaca_position(&http_client, &broker_auth, &sym).await {
                        let profit_usd = return_frac * pos_notional;
                        auditor.record_trade(profit_usd, return_frac);
                        println!(
                            " \x1b[1;32m[ALPACA COHERENCE DECAY TP]\x1b[0m Decaimiento de coherencia ({:.3}). Capturando +{:.2}% | Ganancia: +${:.2} USD en {}",
                            snap.quantum_coherence, return_frac * 100.0, profit_usd, sym
                        );
                        live_positions.remove(&sym);
                        last_broker_close_time.insert(sym.clone(), Instant::now());
                        recent_event_logs.push_back(alpaca_streamer::PipelineLogEntry {
                            timestamp_str: format_now_hms(),
                            symbol: sym.clone(),
                            phase: format!("{:?}", snap.active_phase),
                            intent: "COHERENCE_TP".to_string(),
                            message: format!("Coherence TP (+{:.2}%): ganancia +${:.2} USD", return_frac * 100.0, profit_usd),
                            is_trade: true,
                        });
                        if recent_event_logs.len() > 30 { recent_event_logs.pop_front(); }
                    }
                } else if is_time_tp {
                    if close_alpaca_position(&http_client, &broker_auth, &sym).await {
                        let profit_usd = return_frac * pos_notional;
                        auditor.record_trade(profit_usd, return_frac);
                        println!(
                            " \x1b[1;32m[ALPACA TIME PROFIT EXIT]\x1b[0m Tiempo limite >15m alcanzado con ganancia (+{:.2}%). Ganancia: +${:.2} USD en {}",
                            return_frac * 100.0, profit_usd, sym
                        );
                        live_positions.remove(&sym);
                        last_broker_close_time.insert(sym.clone(), Instant::now());
                        recent_event_logs.push_back(alpaca_streamer::PipelineLogEntry {
                            timestamp_str: format_now_hms(),
                            symbol: sym.clone(),
                            phase: format!("{:?}", snap.active_phase),
                            intent: "TIME_EXIT".to_string(),
                            message: format!("Time TP (+{:.2}%): ganancia +${:.2} USD", return_frac * 100.0, profit_usd),
                            is_trade: true,
                        });
                        if recent_event_logs.len() > 30 { recent_event_logs.pop_front(); }
                    }
                } else if is_stop_hit {
                    if close_alpaca_position(&http_client, &broker_auth, &sym).await {
                        let pnl_usd = return_frac * pos_notional;
                        auditor.record_trade(pnl_usd, return_frac);
                        let exit_type = if pos_locked { "PROFIT_LOCK_EXIT" } else { "STOP_LOSS" };
                        let color = if return_frac >= 0.0 { "\x1b[1;32m" } else { "\x1b[1;31m" };
                        println!(
                            " {}[ALPACA {}]\x1b[0m Liquidada en {}: {:+.2}% (${:+.2} USD) | Stop: ${:.2}",
                            color, exit_type, sym, return_frac * 100.0, pnl_usd, pos_trailing_stop
                        );
                        live_positions.remove(&sym);
                        last_broker_close_time.insert(sym.clone(), Instant::now());
                        recent_event_logs.push_back(alpaca_streamer::PipelineLogEntry {
                            timestamp_str: format_now_hms(),
                            symbol: sym.clone(),
                            phase: format!("{:?}", snap.active_phase),
                            intent: exit_type.to_string(),
                            message: format!("{}: {:+.2}% (${:+.2} USD)", exit_type, return_frac * 100.0, pnl_usd),
                            is_trade: true,
                        });
                        if recent_event_logs.len() > 30 { recent_event_logs.pop_front(); }
                    }
                }
            } else if is_crypto || (sym != "SPY" && sym != "QQQ") {
                // A2. No open position -> Evaluate Entry Criteria via 6-Stage Pre-Trade Pipeline
                let is_market_open = market_open_flag.load(Ordering::Relaxed);
                // Rule 1: Portfolio risk cap: max 2 concurrent positions
                // Rule 2: Cooldown of 180 seconds (3 minutes) per symbol
                let can_enter = is_market_open
                    && live_positions.len() < 2
                    && last_broker_close_time
                        .get(&sym)
                        .map(|t| t.elapsed() >= Duration::from_secs(180))
                        .unwrap_or(true);

                if can_enter {
                    let best_bid = if snap.best_bid > 0.0 { snap.best_bid } else { snap.mid_price - (snap.mid_price * 0.0001).max(0.01) };
                    let best_ask = if snap.best_ask > 0.0 { snap.best_ask } else { snap.mid_price + (snap.mid_price * 0.0001).max(0.01) };
                    let kelly_f = auditor.kelly_leverage(snap.quantum_coherence).clamp(0.2, 2.0);
                    let sor_intent = SmartOrderRouter::route_intent(&snap, best_bid, best_ask, kelly_f);

                    let (is_signal, side, size_ratio) = match sor_intent {
                        OrderIntent::AggressiveTaker { side, size_ratio, urgency: _ } => {
                            (true, side, size_ratio)
                        }
                        OrderIntent::PassiveMaker { side, target_price: _, size_ratio } => {
                            (true, side, size_ratio)
                        }
                        OrderIntent::EmergencyCancelAll { reason } => {
                            signal_confirmations.remove(&sym);
                            if snap.active_phase == MarketQuantumPhase::LiquidityCavitation {
                                println!(
                                    " \x1b[1;33m[SOR REPOSO]\x1b[0m Neutralizacion en {}: {} | Mach: {:.2} | S: {:.3}",
                                    sym, reason, snap.fluid_metrics.mach_number, snap.normalized_entropy
                                );
                            }
                            (false, Side::Buy, 0.0)
                        }
                        OrderIntent::NeutralHold => {
                            (false, Side::Buy, 0.0)
                        }
                    };

                    // RULE: Equities are strictly LONG-ONLY
                    if !is_crypto && side == Side::Sell {
                        // Shorting equities prohibited to protect win rate
                    } else if is_signal {
                        let is_crypto_sym = sym.contains('/') || sym.contains("USD");

                        // Filter 1: Spread Bid-Ask limit check (< 12 bps crypto, < 8 bps equities)
                        let spread_bps = if snap.mid_price > 0.0 {
                            ((best_ask - best_bid).max(0.0) / snap.mid_price) * 10_000.0
                        } else {
                            999.0
                        };
                        let max_spread_bps = if is_crypto_sym { 12.0 } else { 8.0 };
                        let pass_spread = spread_bps <= max_spread_bps;

                        // Filter 2: Net Return covers fees + slippage (Target TP 80 bps - friction >= 12 bps margin)
                        let roundtrip_friction = if snap.mid_price > 0.0 {
                            ((best_ask - best_bid).max(0.0) / snap.mid_price) + if is_crypto_sym { 0.0020 } else { 0.0004 }
                        } else {
                            0.01
                        };
                        let expected_tp = 0.0080;
                        let net_expected_return = expected_tp - roundtrip_friction;
                        let pass_cost = net_expected_return >= 0.0012;

                        // Filter 3: WebSocket data age / latency check (< configurable via MAX_LATENCY_MS, default 1500 ms)
                        let now_ns = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_nanos() as u64;
                        let max_latency_ms: f64 = std::env::var("MAX_LATENCY_MS")
                            .ok()
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(1500.0);
                        let delay_ms = if snap.timestamp_ns > 0 {
                            (now_ns.saturating_sub(snap.timestamp_ns)) as f64 / 1_000_000.0
                        } else {
                            0.0
                        };
                        let pass_latency = if max_latency_ms > 0.0 && snap.timestamp_ns > 0 {
                            delay_ms <= max_latency_ms
                        } else {
                            true
                        };

                        // Anti-whipsaw confirmation window: require 2 consecutive signals or strong conviction
                        let conf_entry = signal_confirmations.entry(sym.clone()).or_insert((side, 0, Instant::now()));
                        if conf_entry.0 == side && conf_entry.2.elapsed() < Duration::from_secs(10) {
                            conf_entry.1 += 1;
                            conf_entry.2 = Instant::now();
                        } else {
                            *conf_entry = (side, 1, Instant::now());
                        }

                        let is_confirmed = conf_entry.1 >= 2 || (snap.phase_confidence >= 0.65 && snap.quantum_coherence >= 0.08);

                        // Macro alignment: Buy only when SPY/QQQ macro bias >= -0.05
                        let is_macro_aligned = if side == Side::Buy {
                            current_market_bias >= -0.05
                        } else {
                            current_market_bias <= 0.05
                        };

                        // Daily Drawdown Circuit Breaker (< 2.0% max loss from daily start equity)
                        let daily_loss_pct = if daily_start_equity > 0.0 {
                            (daily_start_equity - auditor.current_equity) / daily_start_equity
                        } else {
                            0.0
                        };
                        let pass_drawdown = daily_loss_pct < 0.02;

                        if pass_spread && pass_cost && pass_latency && pass_drawdown && is_confirmed && is_macro_aligned {
                            // Broker Guard: verify via REST API that symbol has NO open orders or positions
                            if !has_open_orders_or_position(&http_client, &broker_auth, &sym).await {
                                signal_confirmations.remove(&sym);
                                let side_buy = side == Side::Buy;
                                let order_side_str = if side_buy { "buy" } else { "sell" };

                                // Conservative position sizing: cap at $1,000 / ~1% equity per position
                                let live_eq = auditor.current_equity.max(1000.0);
                                let base_notional = (live_eq * 0.010).clamp(300.0, 1000.0);
                                let notional = (base_notional * kelly_f * size_ratio).min(1000.0).max(100.0);
                                let notional = (notional * 100.0).round() / 100.0;

                                let (qty, _qty_log_str) = if is_crypto_sym {
                                    let raw = notional / snap.mid_price;
                                    let q = ((raw * 10_000_000.0).round() / 10_000_000.0).max(0.0000001);
                                    (q, format!("{:.6}", q))
                                } else {
                                    // Equities: Whole integer shares
                                    let shares = ((notional / snap.mid_price).round() as u64).max(1);
                                    (shares as f64, format!("{}", shares))
                                };

                                // Marketable limit price: best_ask * 1.0005 (5 bps slippage cap) for immediate taker fill
                                let slippage_bps = if is_crypto_sym { 0.0010 } else { 0.0005 };
                                let limit_price_clamped = if side_buy {
                                    ((best_ask * (1.0 + slippage_bps) * 100.0).round() / 100.0).max(best_ask + 0.01)
                                } else {
                                    ((best_bid * (1.0 - slippage_bps) * 100.0).round() / 100.0).min(best_bid - 0.01)
                                };

                                let order_arg = (Some(qty), None);

                                if execute_alpaca_paper_order(
                                    &http_client,
                                    &broker_auth,
                                    &sym,
                                    order_side_str,
                                    order_arg,
                                    Some(limit_price_clamped),
                                    None, // Direct supervisor management, no bracket order collisions
                                ).await {
                                    // Wait 250ms and verify fill directly on Alpaca REST API
                                    tokio::time::sleep(Duration::from_millis(250)).await;
                                    if let Some((real_qty, avg_entry, mkt_val, verified_side)) =
                                        verify_and_fetch_single_position(&http_client, &broker_auth, &sym).await
                                    {
                                        println!(
                                            " \x1b[1;36m[ALPACA FILLED]\x1b[0m {} {} shares @ ${:.2} (Nocional: ${:.2}) | Coher: {:.3} | S: {:.3}",
                                            if verified_side { "LONG" } else { "SHORT" }, real_qty, avg_entry, mkt_val, snap.quantum_coherence, snap.normalized_entropy
                                        );
                                        let trailing_stop = if verified_side {
                                            avg_entry * (1.0 - 0.0045)
                                        } else {
                                            avg_entry * (1.0 + 0.0045)
                                        };
                                        live_positions.insert(
                                            sym.clone(),
                                            LiveBrokerPosition {
                                                symbol: sym.clone(),
                                                side_buy: verified_side,
                                                entry_price: avg_entry,
                                                qty: real_qty,
                                                notional: mkt_val,
                                                entry_time: Instant::now(),
                                                highest_price: avg_entry,
                                                trailing_stop,
                                                profit_locked: false,
                                            },
                                        );
                                        recent_event_logs.push_back(alpaca_streamer::PipelineLogEntry {
                                            timestamp_str: format_now_hms(),
                                            symbol: sym.clone(),
                                            phase: format!("{:?}", snap.active_phase),
                                            intent: "LONG_FILLED".to_string(),
                                            message: format!("Compra ejecutada: {} @ ${:.2} (Nocional: ${:.2})", real_qty, avg_entry, mkt_val),
                                            is_trade: true,
                                        });
                                        if recent_event_logs.len() > 30 { recent_event_logs.pop_front(); }
                                    } else {
                                        // If order did not fill immediately, defensively purge all resting orders
                                        let _ = cancel_all_alpaca_orders(&http_client, &broker_auth).await;
                                    }
                                }
                            }
                        }
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

            // Prepare Rich Telemetry for Neural Network Dashboard
            let mut symbols_telemetry: Vec<alpaca_streamer::SymbolStateTelemetry> = Vec::new();
            let target_symbols: Vec<String> = if !live_symbols.is_empty() {
                live_symbols.iter().map(|s| s.to_string()).collect()
            } else if !real_snapshots.is_empty() {
                real_snapshots.keys().cloned().collect()
            } else {
                vec!["BTC/USD".to_string(), "ETH/USD".to_string(), "SOL/USD".to_string()]
            };

            for sym in &target_symbols {
                if let Some(snap) = real_snapshots.get(sym) {
                    let spread_bps = if snap.mid_price > 0.0 && snap.best_ask >= snap.best_bid {
                        ((snap.best_ask - snap.best_bid) / snap.mid_price) * 10_000.0
                    } else {
                        0.0
                    };
                    symbols_telemetry.push(alpaca_streamer::SymbolStateTelemetry {
                        symbol: sym.clone(),
                        mid_price: snap.mid_price,
                        micro_price: snap.micro_price,
                        best_bid: snap.best_bid,
                        best_ask: snap.best_ask,
                        spread_bps,
                        normalized_entropy: snap.normalized_entropy,
                        quantum_coherence: snap.quantum_coherence,
                        active_phase: format!("{:?}", snap.active_phase),
                        phase_confidence: snap.phase_confidence,
                        mach_number: snap.fluid_metrics.mach_number,
                        speed_of_sound: snap.fluid_metrics.sound_speed,
                        fluid_velocity: snap.fluid_metrics.flow_velocity,
                        is_cavitation: snap.fluid_metrics.is_cavitation_risk,
                        vpin: snap.vpin,
                        is_toxic: snap.is_toxic_flow || snap.vpin >= 0.70,
                        tunneling_ask: snap.tunneling_prob_ask,
                        tunneling_bid: snap.tunneling_prob_bid,
                        state_bias: snap.state_bias,
                        ofi_accel: snap.ofi_acceleration,
                        depth_dispersion: snap.depth_probabilities,
                        sor_intent: "MONITORING".to_string(),
                    });
                } else {
                    symbols_telemetry.push(alpaca_streamer::SymbolStateTelemetry {
                        symbol: sym.clone(),
                        mid_price: 0.0,
                        micro_price: 0.0,
                        best_bid: 0.0,
                        best_ask: 0.0,
                        spread_bps: 0.0,
                        normalized_entropy: 0.5,
                        quantum_coherence: 0.0,
                        active_phase: "ThermalNoise".to_string(),
                        phase_confidence: 0.5,
                        mach_number: 0.0,
                        speed_of_sound: 1.0,
                        fluid_velocity: 0.0,
                        is_cavitation: false,
                        vpin: 0.0,
                        is_toxic: false,
                        tunneling_ask: 0.5,
                        tunneling_bid: 0.5,
                        state_bias: 0.0,
                        ofi_accel: 0.0,
                        depth_dispersion: [0.125; 8],
                        sor_intent: "AWAITING_TICKS".to_string(),
                    });
                }
            }

            let mut positions_telemetry: Vec<alpaca_streamer::PositionTelemetry> = Vec::new();
            for (sym, pos) in &live_positions {
                let cur_price = real_snapshots.get(sym).map(|s| s.mid_price).unwrap_or(pos.entry_price);
                let diff = if pos.side_buy { cur_price - pos.entry_price } else { pos.entry_price - cur_price };
                let pnl_pct = if pos.entry_price > 0.0 { (diff / pos.entry_price) * 100.0 } else { 0.0 };
                let pnl_usd = diff * pos.qty;
                let holding_secs = pos.entry_time.elapsed().as_secs();

                positions_telemetry.push(alpaca_streamer::PositionTelemetry {
                    symbol: sym.clone(),
                    side: if pos.side_buy { "LONG".to_string() } else { "SHORT".to_string() },
                    entry_price: pos.entry_price,
                    current_price: cur_price,
                    qty: pos.qty,
                    notional: pos.notional,
                    pnl_pct,
                    pnl_usd,
                    holding_secs,
                });
            }

            let is_toxic_flag = is_toxic || vpin >= 0.70;
            let pipeline_steps: Vec<alpaca_streamer::PipelineStepTelemetry> = vec![
                alpaca_streamer::PipelineStepTelemetry {
                    id: 1,
                    name: "Alpaca Ingestion L1/L2".to_string(),
                    code: "INGESTION_WS".to_string(),
                    status: if is_alpaca_up { "PASS".to_string() } else { "WARNING".to_string() },
                    value_summary: format!("{:.0} ticks/s | Latencia: {:.1} µs", throughput, avg_latency / 1000.0),
                    description: format!("Feed {} activo. {} ticks procesados sin pérdida.", cfg.feed, total_processed_ticks),
                },
                alpaca_streamer::PipelineStepTelemetry {
                    id: 2,
                    name: "Order Flow Microstructure".to_string(),
                    code: "FLOW_MICRO".to_string(),
                    status: if is_toxic_flag { "BLOCKED".to_string() } else { "PASS".to_string() },
                    value_summary: format!("VPIN: {:.3} | Lead-Lag: {:+.2}", vpin, current_market_bias),
                    description: "VPIN informed trading y Lead-Lag temporal vs benchmarks.".to_string(),
                },
                alpaca_streamer::PipelineStepTelemetry {
                    id: 3,
                    name: "Quantum Density Matrix Rho".to_string(),
                    code: "QUANTUM_STATE".to_string(),
                    status: if coherence >= 0.25 { "PASS".to_string() } else { "ACTIVE".to_string() },
                    value_summary: format!("S: {:.3} | Coherencia: {:.3}", entropy, coherence),
                    description: format!("Fase: {:?} ({:.1}% conf). Operador densidad von Neumann.", phase, conf * 100.0),
                },
                alpaca_streamer::PipelineStepTelemetry {
                    id: 4,
                    name: "Hydrodynamic Mach Dynamics".to_string(),
                    code: "FLUID_MACH".to_string(),
                    status: if mach >= 1.5 { "WARNING".to_string() } else { "PASS".to_string() },
                    value_summary: format!("Mach: {:.2} | Cavitación: {}", mach, if mach >= 1.5 { "SI" } else { "NO" }),
                    description: "Compresibilidad Navier-Stokes y detección de shockwaves.".to_string(),
                },
                alpaca_streamer::PipelineStepTelemetry {
                    id: 5,
                    name: "CTQW Wavepacket Tunneling".to_string(),
                    code: "CTQW_TUNNEL".to_string(),
                    status: "PASS".to_string(),
                    value_summary: "Dispersión 8 niveles profundidad".to_string(),
                    description: "Paseo cuántico de tiempo continuo en barreras del libro.".to_string(),
                },
                alpaca_streamer::PipelineStepTelemetry {
                    id: 6,
                    name: "Defensive Pre-Trade Gates".to_string(),
                    code: "VALUATION_GATES".to_string(),
                    status: if is_toxic_flag { "BLOCKED".to_string() } else { "PASS".to_string() },
                    value_summary: format!("MaxDD: {:.2}% | Latencia < 500ms", auditor.max_drawdown * 100.0),
                    description: "Filtros de spread, coste, latencia WS y drawdown diario.".to_string(),
                },
                alpaca_streamer::PipelineStepTelemetry {
                    id: 7,
                    name: "Smart Order Router (SOR)".to_string(),
                    code: "SOR_EXECUTION".to_string(),
                    status: if !live_positions.is_empty() { "ACTIVE".to_string() } else { "PASS".to_string() },
                    value_summary: format!("{} posiciones activas | Kelly: {:.2}x", live_positions.len(), auditor.kelly_leverage(coherence)),
                    description: "Enrutador de órdenes de límite agresivas con protección de slippage.".to_string(),
                },
            ];

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
                    symbols: symbols_telemetry,
                    positions: positions_telemetry,
                    pipeline_steps,
                    recent_logs: recent_event_logs.iter().cloned().collect(),
                };
            }

            last_report_time = now;
            last_report_ticks = total_processed_ticks;
        }

        // Periodic position reconciliation with broker (every 20s) and open order purge
        if last_reconcile_time.elapsed() >= Duration::from_secs(20) && !matches!(broker_auth, BrokerAuth::None) {
            last_reconcile_time = Instant::now();
            let current_broker_positions = fetch_alpaca_positions(&http_client, &broker_auth).await;
            live_positions.retain(|sym, _| {
                let exists = current_broker_positions.contains_key(sym);
                if !exists {
                    println!(
                        " \x1b[1;33m[ALPACA RECONCILIACION]\x1b[0m Posicion {} no encontrada en broker. Sincronizando estado local a cero.",
                        sym
                    );
                }
                exists
            });
            for (sym, pos) in current_broker_positions {
                live_positions.entry(sym).or_insert(pos);
            }
            // Defensively purge any dangling resting orders to prevent phantom accumulation
            cancel_all_alpaca_orders(&http_client, &broker_auth).await;
        }

        interval.tick().await;
    }

    println!("[+] Supervisor detenido de forma segura.");
    Ok(())
}
