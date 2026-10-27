use alpaca_streamer::{AlpacaAuth, AlpacaDataFeed, AlpacaStreamer};
use market_simulator::{MarketRegime, MicrostructureSimulator};
use quantum_engine::{MarketQuantumPhase, PortfolioAuditor, QuantumMarketEngine};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

fn load_env_file() -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(content) = fs::read_to_string(".env") {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    map
}

/// Reads Linux Resident Set Size (RSS) in megabytes from /proc/self/statm.
fn get_memory_rss_mb() -> f64 {
    if let Ok(content) = fs::read_to_string("/proc/self/statm") {
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Ok(pages) = parts[1].parse::<u64>() {
                // Page size is typically 4096 bytes
                return (pages * 4096) as f64 / (1024.0 * 1024.0);
            }
        }
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

fn execute_alpaca_paper_order(
    client: reqwest::Client,
    auth: BrokerAuth,
    symbol: String,
    side: &'static str,
    qty_or_notional: (Option<f64>, Option<f64>),
) {
    if matches!(auth, BrokerAuth::None) {
        return;
    }
    tokio::spawn(async move {
        let payload = if let Some(qty) = qty_or_notional.0 {
            format!(
                r#"{{"symbol":"{}","qty":"{:.2}","side":"{}","type":"market","time_in_force":"gtc"}}"#,
                symbol, qty, side
            )
        } else if let Some(notional) = qty_or_notional.1 {
            format!(
                r#"{{"symbol":"{}","notional":"{:.2}","side":"{}","type":"market","time_in_force":"gtc"}}"#,
                symbol, notional, side
            )
        } else {
            return;
        };

        let mut req = client
            .post("https://paper-api.alpaca.markets/v2/orders")
            .header("Content-Type", "application/json")
            .body(payload);

        match &auth {
            BrokerAuth::OAuth(token) => {
                req = req.header("Authorization", format!("Bearer {}", *token));
            }
            BrokerAuth::KeySecret { key, secret } => {
                req = req
                    .header("APCA-API-KEY-ID", key.as_str())
                    .header("APCA-API-SECRET-KEY", secret.as_str());
            }
            BrokerAuth::None => return,
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
                    } else if !text.trim().is_empty() {
                        eprintln!(
                            " \x1b[1;31m[ALPACA BROKER RECHAZO]\x1b[0m Respuesta de Broker: {}",
                            text.trim()
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!(" \x1b[1;31m[ALPACA NETWORK ERROR]\x1b[0m Fallo HTTP al enviar orden: {:?}", e);
            }
        }
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("==========================================================================================");
    println!("               ENTROPIQ QUANTUM MICROSTRUCTURE LOCAL PERFORMANCE SUPERVISOR               ");
    println!("==========================================================================================");

    let env_file = load_env_file();

    let oauth_token = env::var("ALPACA_ACCESS_TOKEN")
        .ok()
        .or_else(|| env_file.get("ALPACA_ACCESS_TOKEN").cloned())
        .unwrap_or_default();

    let api_key = env::var("ALPACA_API_KEY")
        .ok()
        .or_else(|| env_file.get("ALPACA_API_KEY").cloned())
        .unwrap_or_default();

    let api_secret = env::var("ALPACA_API_SECRET")
        .ok()
        .or_else(|| env_file.get("ALPACA_API_SECRET").cloned())
        .unwrap_or_default();

    let feed_choice = env::var("ALPACA_FEED")
        .ok()
        .or_else(|| env_file.get("ALPACA_FEED").cloned())
        .unwrap_or_else(|| "equities".to_string());

    let (feed, live_symbols, tick_size) = if feed_choice.to_lowercase() == "crypto" {
        println!("[+] Canal de Mercado: Alpaca Cripto (Secundario)");
        (AlpacaDataFeed::Crypto, vec!["BTC/USD", "ETH/USD", "SOL/USD"], 0.50)
    } else {
        println!("[+] Canal de Mercado: Alpaca Equities Wall Street IEX (Top 10 Stocks & ETFs)");
        (
            AlpacaDataFeed::Iex,
            vec!["SPY", "QQQ", "AAPL", "NVDA", "TSLA", "AMD", "MSFT", "AMZN", "META", "GOOGL"],
            0.01,
        )
    };

    let streamer_auth = if !oauth_token.is_empty() {
        println!("[+] Modo de Autenticacion: Alpaca OAuth Token (Activo)");
        Some(AlpacaAuth::OAuthToken(oauth_token.clone()))
    } else if !api_key.is_empty() && !api_secret.is_empty() {
        println!("[+] Modo de Autenticacion: Alpaca API Key / Secret (Activo)");
        Some(AlpacaAuth::KeySecret {
            key: api_key.clone(),
            secret: api_secret.clone(),
        })
    } else {
        println!("[!] Sin credenciales remotas en .env; ejecutando en modo simulador puro de ultra-baja latencia.");
        None
    };

    let running = Arc::new(AtomicBool::new(true));
    let r_clone = running.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("\n[!] Senal de parada recibida. Cerrando supervisor...");
        r_clone.store(false, Ordering::SeqCst);
    });

    let (live_tx, mut live_rx) = mpsc::channel::<(String, f64, f64, f64, f64, MarketQuantumPhase)>(1000);
    let alpaca_connected = Arc::new(AtomicBool::new(false));
    let alpaca_ticks_count = Arc::new(AtomicU64::new(0));

    // 1. Spawns Alpaca WebSocket client in background task
    if let Some(auth) = streamer_auth {
        let conn_flag = alpaca_connected.clone();
        let ticks_flag = alpaca_ticks_count.clone();
        let syms = live_symbols.clone();

        tokio::spawn(async move {
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
                    let _ = live_tx.try_send((
                        sym.to_string(),
                        snap.mid_price,
                        snap.micro_price,
                        snap.normalized_entropy,
                        snap.quantum_coherence,
                        snap.active_phase,
                    ));
                })
                .await;

            if let Err(e) = res {
                eprintln!("[AlpacaStreamer Worker] Error en canal WebSocket: {:?}", e);
            }
            conn_flag.store(false, Ordering::SeqCst);
        });
    }

    // 2. Quantum Engine Pipeline and Simulator Generator
    let mut engine = QuantumMarketEngine::new(0.01);
    let mut simulator = MicrostructureSimulator::new(500.0, 0.01, 1337);
    let mut auditor = PortfolioAuditor::new(100_000.0);

    let regimes = [
        MarketRegime::ThermalEquilibrium,
        MarketRegime::IcebergAbsorption,
        MarketRegime::QuantumBreakout,
        MarketRegime::LiquidityCavitation,
    ];
    let mut regime_idx = 0;
    let mut last_snapshot = None;

    let mut total_processed_ticks: u64 = 0;
    let mut total_latency_ns: u64 = 0;
    let mut min_latency_ns: u64 = u64::MAX;
    let mut max_latency_ns: u64 = 0;

    // Sliding window latencies for percentile estimation
    let mut recent_latencies_ns: Vec<u64> = Vec::with_capacity(20_000);

    let mut in_position = false;
    let mut entry_price = 0.0;
    let mut entry_side_buy = true;
    let mut entry_leverage = 1.0;

    let broker_auth = if !oauth_token.is_empty() {
        BrokerAuth::OAuth(Arc::new(oauth_token.clone()))
    } else if !api_key.is_empty() && !api_secret.is_empty() {
        BrokerAuth::KeySecret {
            key: Arc::new(api_key.clone()),
            secret: Arc::new(api_secret.clone()),
        }
    } else {
        BrokerAuth::None
    };

    let http_client = reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(10)
        .tcp_nodelay(true)
        .build()?;
    let mut live_positions: HashMap<String, LiveBrokerPosition> = HashMap::new();
    let mut last_broker_close_time: HashMap<String, Instant> = HashMap::new();
    let mut current_market_bias: f64 = 0.0;

    let mut last_report_time = Instant::now();
    let mut last_report_ticks: u64 = 0;
    let mut interval = tokio::time::interval(Duration::from_millis(1));

    println!("[+] Supervisor iniciado. Monitorizando telemetria, memoria y tiempos de calculo...");
    println!("[+] Especializacion de Mercado: WALL STREET EQUITIES IEX (SPY, QQQ, NVDA, AAPL, TSLA...)");
    println!("[+] Lead-Lag Cuantico: SPY/QQQ Macro Index Bias -> Arbitraje Estadistico en Acciones");
    println!("[+] Parametros: TP=+0.18% (decay +0.08%), SL=-0.12% (cavit -0.06%), Cooldown=45s, Size=$25 USD");
    println!("------------------------------------------------------------------------------------------\n");

    while running.load(Ordering::Relaxed) {
        // Step A: Check for any real Alpaca tick arriving from WebSocket
        while let Ok((sym, mid, micro, entropy, coherence, phase)) = live_rx.try_recv() {
            println!(
                " \x1b[1;32m[ALPACA LIVE]\x1b[0m {:<7} | Mid: ${:<9.2} | Micro: ${:<9.2} | S: {:.3} | Coher: {:.3} | Fase: {:?}",
                sym, mid, micro, entropy, coherence, phase
            );
            if sym == "SPY" || sym == "QQQ" {
                let bias = ((micro - mid) / (mid * 0.00005)).clamp(-1.0, 1.0);
                current_market_bias = bias;
                engine.set_external_index_bias(bias);
            }

            // Real-time stateful live trading execution on Alpaca Broker (paper account)
            if let Some(pos) = live_positions.get(&sym).cloned() {
                // Monitor active open position for Take-Profit or Stop-Loss
                let price_diff = if pos.side_buy {
                    mid - pos.entry_price
                } else {
                    pos.entry_price - mid
                };
                let return_frac = price_diff / pos.entry_price.max(1.0);

                // 1. Take-Profit (+0.18% target or +0.08% with momentum exhaustion)
                if return_frac >= 0.0018 || (return_frac >= 0.0008 && coherence < 0.20) {
                    let exit_side = if pos.side_buy { "sell" } else { "buy" };
                    execute_alpaca_paper_order(http_client.clone(), broker_auth.clone(), sym.clone(), exit_side, (Some(pos.qty), None));
                    let profit_usd = return_frac * pos.notional;
                    auditor.record_trade(profit_usd, return_frac);
                    println!(
                        " \x1b[1;32m[ALPACA TAKE-PROFIT]\x1b[0m Posicion cerrada en {}: +{:.2}% | Ganancia: +${:.3} USD",
                        sym, return_frac * 100.0, profit_usd
                    );
                    live_positions.remove(&sym);
                    last_broker_close_time.insert(sym.clone(), Instant::now());
                // 2. Stop-Loss (-0.12% max adverse risk or -0.06% with adverse liquidity cavitation)
                } else if return_frac <= -0.0012 || (phase == MarketQuantumPhase::LiquidityCavitation && return_frac <= -0.0006) {
                    let exit_side = if pos.side_buy { "sell" } else { "buy" };
                    execute_alpaca_paper_order(http_client.clone(), broker_auth.clone(), sym.clone(), exit_side, (Some(pos.qty), None));
                    let loss_usd = return_frac * pos.notional;
                    auditor.record_trade(loss_usd, return_frac);
                    println!(
                        " \x1b[1;31m[ALPACA STOP-LOSS]\x1b[0m Riesgo cortado en {}: {:.2}% | Perdida: ${:.3} USD",
                        sym, return_frac * 100.0, loss_usd
                    );
                    live_positions.remove(&sym);
                    last_broker_close_time.insert(sym.clone(), Instant::now());
                }
            } else {
                // No open position in this asset -> evaluate High-Precision quantum entry criteria
                let can_enter = last_broker_close_time
                    .get(&sym)
                    .map(|t| t.elapsed() >= Duration::from_secs(45))
                    .unwrap_or(true);

                let is_skewed = (micro - mid).abs() > (mid * 0.00008);
                let has_quantum_order = coherence >= 0.25 || entropy <= 0.85;

                let trigger_breakout = phase == MarketQuantumPhase::SuperconductingBreakout && coherence >= 0.35 && entropy <= 0.90;
                let trigger_cavitation = phase == MarketQuantumPhase::LiquidityCavitation && is_skewed && has_quantum_order;

                if can_enter && (trigger_breakout || trigger_cavitation) && micro != mid {
                    let side_buy = micro > mid;
                    let order_side = if side_buy { "buy" } else { "sell" };

                    // Lead-Lag Index alignment check for individual equities
                    let is_aligned_with_macro = if side_buy {
                        current_market_bias >= -0.20
                    } else {
                        current_market_bias <= 0.20
                    };

                    if is_aligned_with_macro {
                        let notional = 25.0; // High-precision $25.00 USD notional in equities
                        let qty = ((notional / mid) * 100.0).round() / 100.0;
                        let qty = qty.max(0.01);
                        execute_alpaca_paper_order(http_client.clone(), broker_auth.clone(), sym.clone(), order_side, (None, Some(notional)));
                        let trigger_reason = if trigger_breakout { "BREAKOUT" } else { "CAVITACION" };
                        println!(
                            " \x1b[1;36m[ALPACA EQUITIES: ENTRADA {} ({})]\x1b[0m {} a ${:.2} (qty: {:.2}) | Micro: ${:.2} | Bias SPY: {:.2} | Coher: {:.3} | S: {:.3}",
                            order_side.to_uppercase(), trigger_reason, sym, mid, qty, micro, current_market_bias, coherence, entropy
                        );
                        live_positions.insert(
                            sym.clone(),
                            LiveBrokerPosition {
                                symbol: sym.clone(),
                                side_buy,
                                entry_price: mid,
                                qty,
                                notional,
                                entry_time: Instant::now(),
                            },
                        );
                    }
                }
            }
        }

        // Step B: Generate realistic tick stream to exercise quantum engine under load
        // Generates batches of 50 ticks every millisecond -> ~50,000 ticks/sec throughput
        let current_regime = regimes[regime_idx % regimes.len()];
        if total_processed_ticks % 10_000 == 0 && total_processed_ticks > 0 {
            regime_idx += 1;
        }

        // Cross-asset lead-lag coupling: QuantumBreakout induces macro index drift
        if current_regime == MarketRegime::QuantumBreakout {
            engine.set_external_index_bias(0.60);
        } else {
            engine.set_external_index_bias(0.0);
        }

        for _ in 0..50 {
            let (quote, trade_opt) = simulator.next_tick(current_regime);

            let t_start = Instant::now();
            let snap_opt = engine.on_quote(quote);
            if let Some(trade) = trade_opt {
                engine.on_trade(trade);
            }
            let elapsed_ns = t_start.elapsed().as_nanos() as u64;

            total_processed_ticks += 1;
            total_latency_ns += elapsed_ns;
            if elapsed_ns < min_latency_ns {
                min_latency_ns = elapsed_ns;
            }
            if elapsed_ns > max_latency_ns {
                max_latency_ns = elapsed_ns;
            }

            if recent_latencies_ns.len() < 20_000 {
                recent_latencies_ns.push(elapsed_ns);
            }

            // Dynamic execution with Maker/Taker spread capture, Kelly leverage, and Decoherence stop
            // Dynamic execution: Only record synthetic trades into auditor if Alpaca is NOT active (pure local simulation mode)
            let is_alpaca_live = alpaca_connected.load(Ordering::Relaxed);
            if !is_alpaca_live {
                if let Some(snap) = snap_opt {
                    if !in_position {
                        if snap.active_phase == MarketQuantumPhase::SuperconductingBreakout && snap.quantum_coherence >= 0.70 && snap.phase_confidence > 0.78 {
                            in_position = true;
                            entry_price = snap.mid_price;
                            entry_side_buy = snap.tunneling_prob_ask > snap.tunneling_prob_bid;
                            entry_leverage = auditor.kelly_leverage(snap.quantum_coherence);
                        }
                    } else {
                        let price_diff = if entry_side_buy {
                            snap.mid_price - entry_price
                        } else {
                            entry_price - snap.mid_price
                        };
                        let return_frac = price_diff / entry_price.max(1.0);

                        // Maker spread capture bonus (+0.8 bps on orderly passive fill)
                        let maker_bonus = if snap.tunneling_prob_ask.max(snap.tunneling_prob_bid) <= 0.88 { 0.00008 } else { 0.0 };

                        // 1. Take-Profit on momentum exhaustion or target gain (+0.35%)
                        if return_frac >= 0.0035 || (return_frac >= 0.0018 && snap.active_phase != MarketQuantumPhase::SuperconductingBreakout) {
                            let net_return = (return_frac + maker_bonus) * entry_leverage;
                            let pnl = price_diff * 100.0 * entry_leverage + (maker_bonus * entry_price * 100.0);
                            auditor.record_trade(pnl, net_return);
                            in_position = false;
                        // 2. Stop-Loss on Cavitation Risk or adverse move (-0.15%)
                        } else if (snap.active_phase == MarketQuantumPhase::LiquidityCavitation && return_frac < -0.0005) || return_frac <= -0.0015 {
                            let pnl = price_diff * 100.0 * entry_leverage;
                            auditor.record_trade(pnl, return_frac * entry_leverage);
                            in_position = false;
                        }
                    }
                }
            }
            if let Some(snap) = snap_opt {
                last_snapshot = Some(snap);
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

            // Calculate p50 and p99 from sliding sample
            let (p50, p99) = if !recent_latencies_ns.is_empty() {
                recent_latencies_ns.sort_unstable();
                let p50_idx = (recent_latencies_ns.len() as f64 * 0.50) as usize;
                let p99_idx = (recent_latencies_ns.len() as f64 * 0.99).min(recent_latencies_ns.len() as f64 - 1.0) as usize;
                (recent_latencies_ns[p50_idx], recent_latencies_ns[p99_idx])
            } else {
                (0, 0)
            };
            recent_latencies_ns.clear();

            let rss_mb = get_memory_rss_mb();
            let is_alpaca_up = alpaca_connected.load(Ordering::Relaxed);
            let alpaca_total = alpaca_ticks_count.load(Ordering::Relaxed);

            let (entropy, coherence, mach, phase, conf, vpin, is_toxic) = if let Some(s) = last_snapshot {
                (s.normalized_entropy, s.quantum_coherence, s.fluid_metrics.mach_number, s.active_phase, s.phase_confidence, s.vpin, s.is_toxic_flow)
            } else {
                (1.0, 0.0, 0.0, MarketQuantumPhase::ThermalNoise, 0.5, 0.0, false)
            };

            println!("┌────────────────────────────────────────────────────────────────────────────────────────┐");
            println!(
                "│ \x1b[1;36mENTROPIQ MONITOR\x1b[0m | WebSocket: {:<12} | Ticks Alpaca Recibidos: {:<10} │",
                if is_alpaca_up { "\x1b[1;32mCONECTADO\x1b[0m" } else { "\x1b[1;31mDESCONECTADO\x1b[0m" },
                alpaca_total
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
                if is_toxic { "\x1b[1;31mBLOQUEO\x1b[0m" } else { "\x1b[1;32mPERMISIVO\x1b[0m" },
                engine.external_index_bias
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

            last_report_time = now;
            last_report_ticks = total_processed_ticks;
        }

        interval.tick().await;
    }

    println!("[+] Supervisor detenido de forma segura.");
    Ok(())
}
