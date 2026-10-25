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

    let live_symbols = vec!["SPY", "AAPL", "NVDA"];

    let streamer_auth = if !oauth_token.is_empty() {
        println!("[+] Modo de Autenticacion: Alpaca OAuth Token (Activo)");
        Some(AlpacaAuth::OAuthToken(oauth_token))
    } else if !api_key.is_empty() && !api_secret.is_empty() {
        println!("[+] Modo de Autenticacion: Alpaca API Key / Secret (Activo)");
        Some(AlpacaAuth::KeySecret {
            key: api_key,
            secret: api_secret,
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
                AlpacaAuth::OAuthToken(t) => AlpacaStreamer::with_oauth_token(t, AlpacaDataFeed::Iex, 0.01),
                AlpacaAuth::KeySecret { key, secret } => {
                    AlpacaStreamer::with_key_secret(key, secret, AlpacaDataFeed::Iex, 0.01)
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

    let mut last_report_time = Instant::now();
    let mut last_report_ticks: u64 = 0;
    let mut interval = tokio::time::interval(Duration::from_millis(1));

    println!("[+] Supervisor iniciado. Monitorizando telemetria, memoria y tiempos de calculo...");
    println!("------------------------------------------------------------------------------------------\n");

    while running.load(Ordering::Relaxed) {
        // Step A: Check for any real Alpaca tick arriving from WebSocket
        while let Ok((sym, mid, micro, entropy, coherence, phase)) = live_rx.try_recv() {
            println!(
                " \x1b[1;32m[ALPACA LIVE]\x1b[0m {:<5} | Mid: ${:<7.2} | Micro: ${:<7.2} | S: {:.3} | Coher: {:.3} | Fase: {:?}",
                sym, mid, micro, entropy, coherence, phase
            );
        }

        // Step B: Generate realistic tick stream to exercise quantum engine under load
        // Generates batches of 50 ticks every millisecond -> ~50,000 ticks/sec throughput
        let current_regime = regimes[regime_idx % regimes.len()];
        if total_processed_ticks % 10_000 == 0 && total_processed_ticks > 0 {
            regime_idx += 1;
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

            // Dynamic execution with stateful position management, Kelly leverage, and Decoherence stop
            if let Some(snap) = snap_opt {
                if !in_position {
                    if snap.active_phase == MarketQuantumPhase::SuperconductingBreakout && snap.phase_confidence > 0.75 {
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

                    // 1. Take-Profit on momentum exhaustion or target gain
                    if return_frac >= 0.0004 || (return_frac > 0.0001 && snap.active_phase != MarketQuantumPhase::SuperconductingBreakout) {
                        let pnl = price_diff * 100.0 * entry_leverage;
                        auditor.record_trade(pnl, return_frac * entry_leverage);
                        in_position = false;
                    // 2. Stop-Loss on Cavitation Risk or adverse move
                    } else if snap.active_phase == MarketQuantumPhase::LiquidityCavitation || return_frac <= -0.0003 {
                        let pnl = price_diff * 100.0 * entry_leverage;
                        auditor.record_trade(pnl, return_frac * entry_leverage);
                        in_position = false;
                    }
                }
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

            let (entropy, coherence, mach, phase, conf) = if let Some(s) = last_snapshot {
                (s.normalized_entropy, s.quantum_coherence, s.fluid_metrics.mach_number, s.active_phase, s.phase_confidence)
            } else {
                (1.0, 0.0, 0.0, MarketQuantumPhase::ThermalNoise, 0.5)
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
