pub mod audit;
pub mod engine;
pub mod hamiltonian;
pub mod phase;

pub use audit::PortfolioAuditor;
pub use engine::{QuantumAnalyticsSnapshot, QuantumMarketEngine};
pub use hamiltonian::MarketHamiltonian;
pub use phase::{MarketQuantumPhase, PhaseClassifier};

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
}
