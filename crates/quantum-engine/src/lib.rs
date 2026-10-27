pub mod audit;
pub mod engine;
pub mod hamiltonian;
pub mod phase;
pub mod sor;

pub use audit::PortfolioAuditor;
pub use engine::{QuantumAnalyticsSnapshot, QuantumMarketEngine};
pub use hamiltonian::MarketHamiltonian;
pub use phase::{MarketQuantumPhase, PhaseClassifier};
pub use sor::{OrderIntent, SmartOrderRouter};

#[cfg(test)]
mod tests {
    use super::*;
    use market_microstructure::{Quote, Side, Trade};

    #[test]
    fn test_quantum_market_engine_pipeline() {
        let mut engine = QuantumMarketEngine::new(0.01);

        // Quote 1: balanced market
        let q1 = Quote::new(1_000_000_000, 100.0, 50.0, 100.05, 50.0);
        let snap1 = engine.on_quote(q1).expect("Snapshot 1 generated");

        // High entropy under 50/50 balance
        assert!(snap1.normalized_entropy > 0.85);
        assert_eq!(snap1.active_phase, MarketQuantumPhase::ThermalNoise);
        assert!(snap1.depth_variance > 0.0);

        // Heavy aggressive buy trades occur
        engine.on_trade(Trade::new(1_000_100_000, 100.05, 200.0, Side::Buy));

        // Quote 2: Bid side heavily stacks, ask depletes, price shifts
        let q2 = Quote::new(1_000_200_000, 100.04, 300.0, 100.05, 10.0);
        let snap2 = engine.on_quote(q2).expect("Snapshot 2 generated");

        // Entropy drops significantly as order flow becomes pure and directional
        assert!(snap2.normalized_entropy < snap1.normalized_entropy);
        assert!(snap2.tunneling_prob_ask > snap2.tunneling_prob_bid);
        assert!(snap2.state_bias > 0.0);
    }

    #[test]
    fn test_smart_order_router_and_l2() {
        use market_microstructure::L2Book;

        let mut engine = QuantumMarketEngine::new(0.01);
        let mut book = L2Book::<5>::new(1_000_000_000);
        book.set_bid(0, 100.0, 50.0, 5);
        book.set_ask(0, 100.05, 50.0, 5);

        let snap = engine.on_l2_book(&book).expect("L2 snapshot");
        let intent = SmartOrderRouter::route_intent(&snap, 100.0, 100.05, 1.0);
        // Thermal noise -> NeutralHold
        assert_eq!(intent, OrderIntent::NeutralHold);
    }

    #[test]
    fn test_physical_consistency_no_breakout_during_toxic_or_supersonic_flow() {
        let mut engine = QuantumMarketEngine::new(0.01);
        let q = Quote::new(1_000_000_000, 100.0, 10.0, 100.05, 10.0);
        let mut snap = engine.on_quote(q).expect("Snapshot");

        // Case 1: Supersonic flow (Mach >= 1.0) with low coherence
        snap.fluid_metrics.mach_number = 4.5;
        snap.quantum_coherence = 0.01;
        snap.vpin = 0.80;
        snap.is_toxic_flow = true;
        snap.tunneling_prob_ask = 0.95; // high tunneling must NOT trick classifier into breakout

        let (phase, _) = PhaseClassifier::classify(&snap);
        assert_eq!(phase, MarketQuantumPhase::LiquidityCavitation);

        let intent = SmartOrderRouter::route_intent(&snap, 100.0, 100.05, 1.0);
        assert!(matches!(intent, OrderIntent::EmergencyCancelAll { .. }));

        // Case 2: Zero coherence cannot declare SuperconductingBreakout
        snap.fluid_metrics.mach_number = 0.5; // subsonic
        snap.quantum_coherence = 0.05; // decohered!
        snap.vpin = 0.20;
        snap.is_toxic_flow = false;
        let (phase2, _) = PhaseClassifier::classify(&snap);
        assert_ne!(phase2, MarketQuantumPhase::SuperconductingBreakout);
        assert_eq!(phase2, MarketQuantumPhase::ThermalNoise);
    }
}
