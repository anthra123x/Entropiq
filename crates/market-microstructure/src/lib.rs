pub mod fluid;
pub mod state;
pub mod types;

pub use fluid::{FluidMetrics, LiquidityFluidModel};
pub use state::MarketTracker;
pub use types::{Quote, Side, Trade};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_micro_price_and_imbalance() {
        let q = Quote::new(1_000_000, 100.0, 10.0, 100.5, 30.0);
        assert_eq!(q.spread(), 0.5);
        assert_eq!(q.mid_price(), 100.25);
        assert_eq!(q.micro_price(), 100.125);
        assert_eq!(q.imbalance(), -0.5);
    }

    #[test]
    fn test_market_tracker_ofi() {
        let mut tracker = MarketTracker::new(0.01);
        let q1 = Quote::new(100, 50.0, 100.0, 50.05, 100.0);
        tracker.update_quote(q1);

        let q2 = Quote::new(200, 50.0, 150.0, 50.05, 100.0);
        tracker.update_quote(q2);

        assert_eq!(tracker.accumulated_ofi, 50.0);
    }

    #[test]
    fn test_liquidity_fluid_mach_and_cavitation() {
        // High depth, small spread, slow price drift -> Subsonic laminar flow (Mach < 1.0)
        let metrics_laminar = LiquidityFluidModel::compute_metrics(1000.0, 0.01, 0.001, 0.01);
        assert!(metrics_laminar.mach_number < 1.0);
        assert!(!metrics_laminar.is_cavitation_risk);

        // Thin depth, sudden jump -> Supersonic cavitation shock (Mach >= 1.0)
        let metrics_shock = LiquidityFluidModel::compute_metrics(10.0, 0.05, 0.50, 0.0005);
        assert!(metrics_shock.mach_number >= 1.0);
        assert!(metrics_shock.is_cavitation_risk);
    }
}
