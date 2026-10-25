use market_simulator::{
    BinaryBufferReplayer, BinaryQuoteRecord, MarketRegime, MicrostructureSimulator,
};
use quantum_engine::{MarketQuantumPhase, QuantumMarketEngine};
use std::time::Instant;

fn main() {
    println!("==========================================================================================");
    println!("          ZERO-COPY BINARY REPLAY & QUANTUM PHASE TRANSITION BENCHMARK                    ");
    println!("==========================================================================================");

    let total_ticks = 1_000_000;
    println!("Step 1: Generating {} binary ticks (32 bytes/tick = {:.2} MB)...",
        total_ticks, (total_ticks * BinaryQuoteRecord::SIZE) as f64 / (1024.0 * 1024.0));

    let mut sim = MicrostructureSimulator::new(100.0, 0.01, 9999);
    let mut binary_buffer = Vec::with_capacity(total_ticks * BinaryQuoteRecord::SIZE);

    // Alternate regimes across the million ticks to create real phase transitions
    for i in 0..total_ticks {
        let regime = match (i / 250_000) % 4 {
            0 => MarketRegime::ThermalEquilibrium,
            1 => MarketRegime::IcebergAbsorption,
            2 => MarketRegime::QuantumBreakout,
            _ => MarketRegime::LiquidityCavitation,
        };
        let (quote, _) = sim.next_tick(regime);
        let record = BinaryQuoteRecord::from_quote(&quote);
        binary_buffer.extend_from_slice(&record.to_bytes());
    }

    println!("Step 2: Streaming and analyzing binary buffer with complete quantum physics engine...");
    let mut engine = QuantumMarketEngine::new(0.01);
    let mut replayer = BinaryBufferReplayer::new(&binary_buffer);

    let mut phase_thermal_count = 0u64;
    let mut phase_condensate_count = 0u64;
    let mut phase_breakout_count = 0u64;
    let mut phase_cavitation_count = 0u64;

    let start = Instant::now();
    let mut processed = 0u64;

    while let Some(quote) = replayer.next_quote() {
        if let Some(snap) = engine.on_quote(quote) {
            match snap.active_phase {
                MarketQuantumPhase::ThermalNoise => phase_thermal_count += 1,
                MarketQuantumPhase::CoherentCondensate => phase_condensate_count += 1,
                MarketQuantumPhase::SuperconductingBreakout => phase_breakout_count += 1,
                MarketQuantumPhase::LiquidityCavitation => phase_cavitation_count += 1,
            }
        }
        processed += 1;
    }

    let elapsed = start.elapsed();
    let nanos_per_tick = elapsed.as_nanos() as f64 / processed as f64;
    let ticks_per_sec = processed as f64 / elapsed.as_secs_f64();

    println!("\nReplay Results:");
    println!("  Total Ticks Processed:     {}", processed);
    println!("  Total Execution Time:      {:.4?}", elapsed);
    println!("  Throughput:                {:.2} million ticks/sec", ticks_per_sec / 1_000_000.0);
    println!("  Latency per Tick:          {:.2} ns ({:.4} µs)", nanos_per_tick, nanos_per_tick / 1000.0);

    println!("\nDetected Quantum Phase Distribution:");
    println!("  Phase 0 [Thermal Noise]:            {:>8} ticks ({:.1}%)",
        phase_thermal_count, (phase_thermal_count as f64 / processed as f64) * 100.0);
    println!("  Phase 1 [Coherent Condensate]:      {:>8} ticks ({:.1}%)",
        phase_condensate_count, (phase_condensate_count as f64 / processed as f64) * 100.0);
    println!("  Phase 2 [Superconducting Breakout]: {:>8} ticks ({:.1}%)",
        phase_breakout_count, (phase_breakout_count as f64 / processed as f64) * 100.0);
    println!("  Phase 3 [Liquidity Cavitation]:     {:>8} ticks ({:.1}%)",
        phase_cavitation_count, (phase_cavitation_count as f64 / processed as f64) * 100.0);

    println!("==========================================================================================");
}
