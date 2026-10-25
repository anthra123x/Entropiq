use quantum_engine::QuantumMarketEngine;
use market_microstructure::{Quote, Side, Trade};
use std::time::Instant;

fn main() {
    println!("=== Quantum Market Microstructure Engine Benchmark ===");
    let mut engine = QuantumMarketEngine::new(0.01);
    let iterations: u64 = 1_000_000;

    let start = Instant::now();
    for i in 0..iterations {
        let ts = 1_000_000_000 + i * 1_000;
        let p_bid = 100.0 + (i as f64 * 0.0001).sin() * 0.5;
        let p_ask = p_bid + 0.02;

        let q = Quote::new(ts, p_bid, 50.0 + (i % 20) as f64, p_ask, 60.0 - (i % 20) as f64);
        let _ = engine.on_quote(q);

        if i % 5 == 0 {
            let tr = Trade::new(ts + 500, p_ask, 10.0, Side::Buy);
            engine.on_trade(tr);
        }
    }
    let elapsed = start.elapsed();
    let nanos_per_tick = elapsed.as_nanos() as f64 / iterations as f64;
    let ticks_per_sec = iterations as f64 / elapsed.as_secs_f64();

    println!("Processed {} ticks in {:.4?}", iterations, elapsed);
    println!("Latency per tick: {:.2} ns ({:.4} µs)", nanos_per_tick, nanos_per_tick / 1000.0);
    println!("Throughput: {:.2} million ticks/sec", ticks_per_sec / 1_000_000.0);

    if let Some(snap) = engine.compute_snapshot(0.001) {
        println!("\nFinal State Snapshot:");
        println!("  Mid Price:            {:.4}", snap.mid_price);
        println!("  Micro Price:          {:.4}", snap.micro_price);
        println!("  von Neumann Entropy:  {:.4} nats", snap.von_neumann_entropy);
        println!("  Normalized Entropy:   {:.4}", snap.normalized_entropy);
        println!("  Quantum Coherence:    {:.4}", snap.quantum_coherence);
        println!("  Ask Tunneling Prob:   {:.4}", snap.tunneling_prob_ask);
        println!("  Bid Tunneling Prob:   {:.4}", snap.tunneling_prob_bid);
        println!("  Net State Bias:       {:.4}", snap.state_bias);
    }
}
