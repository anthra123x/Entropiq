use market_simulator::{MarketRegime, MicrostructureSimulator};
use quantum_engine::QuantumMarketEngine;

#[allow(dead_code)]
struct RegimeStats {
    name: &'static str,
    final_mid: f64,
    final_spread: f64,
    avg_entropy: f64,
    final_entropy: f64,
    final_coherence: f64,
    tunnel_ask: f64,
    tunnel_bid: f64,
    max_mach: f64,
    cavitation_events: u32,
}

fn evaluate_regime(
    regime: MarketRegime,
    name: &'static str,
    ticks: usize,
    sim: &mut MicrostructureSimulator,
) -> RegimeStats {
    let mut engine = QuantumMarketEngine::new(0.01);
    let mut entropy_sum = 0.0;
    let mut max_mach = 0.0_f64;
    let mut cavitation_events = 0;
    let mut last_snap = None;

    for _ in 0..ticks {
        let (quote, maybe_trade) = sim.next_tick(regime);
        if let Some(trade) = maybe_trade {
            engine.on_trade(trade);
        }
        if let Some(snap) = engine.on_quote(quote) {
            entropy_sum += snap.normalized_entropy;
            if snap.fluid_metrics.mach_number > max_mach {
                max_mach = snap.fluid_metrics.mach_number;
            }
            if snap.fluid_metrics.is_cavitation_risk {
                cavitation_events += 1;
            }
            last_snap = Some(snap);
        }
    }

    let snap = last_snap.expect("Must have at least one snapshot");
    let quote = engine.tracker.current_quote.unwrap();

    RegimeStats {
        name,
        final_mid: snap.mid_price,
        final_spread: quote.spread(),
        avg_entropy: entropy_sum / (ticks as f64),
        final_entropy: snap.normalized_entropy,
        final_coherence: snap.quantum_coherence,
        tunnel_ask: snap.tunneling_prob_ask,
        tunnel_bid: snap.tunneling_prob_bid,
        max_mach,
        cavitation_events,
    }
}

fn main() {
    println!("==========================================================================================");
    println!("             QUANTUM & HYDRODYNAMIC MARKET FRAMEWORK: REGIME EVALUATION                   ");
    println!("==========================================================================================");

    let mut sim = MicrostructureSimulator::new(100.0, 0.01, 777);
    let ticks_per_regime = 2000;

    let s1 = evaluate_regime(MarketRegime::ThermalEquilibrium, "1. Thermal Equilibrium (Noise)", ticks_per_regime, &mut sim);
    let s2 = evaluate_regime(MarketRegime::IcebergAbsorption, "2. Iceberg Absorption (Stealth)", ticks_per_regime, &mut sim);
    let s3 = evaluate_regime(MarketRegime::QuantumBreakout, "3. Quantum Breakout (Momentum)", ticks_per_regime, &mut sim);
    let s4 = evaluate_regime(MarketRegime::LiquidityCavitation, "4. Liquidity Cavitation (Flash Crash)", ticks_per_regime, &mut sim);

    let stats = [s1, s2, s3, s4];

    println!("{:<36} | {:<7} | {:<7} | {:<7} | {:<7} | {:<10} | {:<12}",
        "Market Regime", "Spread", "S(norm)", "Coher.", "TunnelAsk", "Max Mach", "Cavitations");
    println!("-------------------------------------+---------+---------+---------+---------+------------+------------");

    for s in &stats {
        println!("{:<36} | ${:<6.4} | {:<7.4} | {:<7.4} | {:<7.4} | {:<10.4} | {:<12}",
            s.name, s.final_spread, s.final_entropy, s.final_coherence, s.tunnel_ask, s.max_mach, s.cavitation_events);
    }

    println!("==========================================================================================");
    println!("\nPhysical Interpretation of Results:");
    println!("1. Thermal Equilibrium: Maximum entropy (~0.85-0.99), laminar subsonic flow (Mach < 1.0).");
    println!("2. Iceberg Absorption:  Entropy collapses (<0.20) revealing pure institutional directional intent.");
    println!("3. Quantum Breakout:    Ask tunneling probability surges with high directional kinetic momentum.");
    println!("4. Liquidity Cavitation: Mach number spikes (>1.0), triggering continuous cavitation alerts.");
}
