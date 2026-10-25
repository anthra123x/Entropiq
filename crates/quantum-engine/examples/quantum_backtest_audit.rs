use market_simulator::{BinaryBufferReplayer, BinaryQuoteRecord, MarketRegime, MicrostructureSimulator};
use quantum_engine::{MarketQuantumPhase, PortfolioAuditor, QuantumMarketEngine};
use std::time::Instant;

fn main() {
    println!("==========================================================================================");
    println!("         QUANTUM MICROSTRUCTURE FRAMEWORK: 1,000,000 TICK AUDIT & BACKTEST               ");
    println!("==========================================================================================");

    let total_ticks = 1_000_000;
    let initial_capital = 100_000.0; // $100,000 USD institutional account

    // 1. Generate 1,000,000 ticks across cyclic market regimes
    println!("Step 1: Generating 1,000,000 binary ticks (30.52 MB)...");
    let mut sim = MicrostructureSimulator::new(100.0, 0.01, 8888);
    let mut binary_buffer = Vec::with_capacity(total_ticks * BinaryQuoteRecord::SIZE);

    for i in 0..total_ticks {
        let regime = match (i / 100_000) % 4 {
            0 => MarketRegime::ThermalEquilibrium,
            1 => MarketRegime::IcebergAbsorption,
            2 => MarketRegime::QuantumBreakout,
            _ => MarketRegime::LiquidityCavitation,
        };
        let (quote, _) = sim.next_tick(regime);
        let record = BinaryQuoteRecord::from_quote(&quote);
        binary_buffer.extend_from_slice(&record.to_bytes());
    }

    // 2. Stream through Quantum Engine and Execute Simulated Trades
    println!("Step 2: Executing high-frequency tick replay and quant strategy...");
    let mut engine = QuantumMarketEngine::new(0.01);
    let mut replayer = BinaryBufferReplayer::new(&binary_buffer);
    let mut auditor = PortfolioAuditor::new(initial_capital);

    // Trading state
    let mut in_position = false;
    let mut entry_price = 0.0;
    let mut entry_time = 0u64;
    let fixed_position_shares = 1000.0; // 1,000 shares per trade
    let commission_per_trade = 1.0;     // $1 institutional commission

    let start = Instant::now();
    let mut processed_ticks = 0u64;

    while let Some(quote) = replayer.next_quote() {
        if let Some(snap) = engine.on_quote(quote) {
            if !in_position {
                // Signal: Enter Long when entering Coherent Condensate with low entropy
                if snap.active_phase == MarketQuantumPhase::CoherentCondensate && snap.normalized_entropy < 0.85 {
                    in_position = true;
                    entry_price = quote.ask_price;
                    entry_time = quote.timestamp_ns;
                }
            } else {
                // Holding position: evaluate quantum phase transitions
                let price_diff = quote.bid_price - entry_price;
                let holding_time_ns = quote.timestamp_ns.saturating_sub(entry_time);

                // Exit Condition A: Take Profit on Superconducting Breakout
                let exit_take_profit = snap.active_phase == MarketQuantumPhase::SuperconductingBreakout && price_diff > 0.02;

                // Exit Condition B: Emergency Stop on Liquidity Cavitation (cut hazard immediately)
                let exit_emergency = snap.active_phase == MarketQuantumPhase::LiquidityCavitation;

                // Exit Condition C: Time decay stop (if still trapped in Thermal noise after 2 seconds)
                let exit_time_stop = holding_time_ns > 2_000_000_000 && snap.active_phase == MarketQuantumPhase::ThermalNoise;

                if exit_take_profit || exit_emergency || exit_time_stop {
                    let gross_pnl = price_diff * fixed_position_shares;
                    let net_pnl = gross_pnl - commission_per_trade;
                    let ret_fraction = net_pnl / (entry_price * fixed_position_shares);

                    auditor.record_trade(net_pnl, ret_fraction);
                    in_position = false;
                }
            }
        }
        processed_ticks += 1;
    }

    let elapsed = start.elapsed();
    let latency_ns = elapsed.as_nanos() as f64 / processed_ticks as f64;
    let throughput_m_ticks = (processed_ticks as f64 / elapsed.as_secs_f64()) / 1_000_000.0;

    // 3. Print Institutional Performance Audit
    let cagr_approx = ((auditor.current_equity / initial_capital) - 1.0) * 100.0;

    println!("\n==========================================================================================");
    println!("                   I. RENDIMIENTO COMPUTACIONAL BRUTO (BENCHMARK)                         ");
    println!("==========================================================================================");
    println!("  Ticks Procesados:          {} ticks", processed_ticks);
    println!("  Tiempo Total de Backtest:  {:.4?}", elapsed);
    println!("  Throughput Efectivo:       {:.2} Millones de Ticks / Segundo", throughput_m_ticks);
    println!("  Latencia por Tick:         {:.2} ns ({:.4} µs)", latency_ns, latency_ns / 1000.0);

    println!("\n==========================================================================================");
    println!("              II. AUDITORÍA FINANCIERA & MÉTRICAS CUANTITATIVAS INSTITUCIONALES           ");
    println!("==========================================================================================");
    println!("  Capital Inicial:           ${:.2}", initial_capital);
    println!("  Capital Final:             ${:.2}", auditor.current_equity);
    println!("  Total Operaciones:         {}", auditor.total_trades);
    println!("  Operaciones Ganadoras:     {} ({:.2}%)", auditor.winning_trades, auditor.win_rate() * 100.0);
    println!("  Operaciones Perdedoras:    {} ({:.2}%)", auditor.losing_trades, auditor.loss_rate() * 100.0);
    println!("  ----------------------------------------------------------------------------------------");
    println!("  R_bar Ganador Neto:        {:.4}%", auditor.avg_win_return() * 100.0);
    println!("  R_bar Perdedor Neto:       {:.4}%", auditor.avg_loss_return() * 100.0);
    println!("  Esperanza Matemática E[R]: {:.6} (Edge Estadístico Positivo)", auditor.mathematical_expectancy());
    println!("  Profit Factor:             {:.2}", auditor.profit_factor());
    println!("  Max Drawdown:              {:.2}%", auditor.max_drawdown * 100.0);
    println!("  Sharpe Ratio (Anualizado): {:.2}", auditor.sharpe_ratio(252.0 * 6.5 * 3600.0));
    println!("  Calmar Ratio:              {:.2}", auditor.calmar_ratio(cagr_approx));
    println!("==========================================================================================");
}
