pub mod book_l2;
pub mod fluid;
pub mod state;
pub mod types;

pub use book_l2::{L2Book, L2Level};
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

    #[test]
    fn test_market_tracker_time_decay() {
        let mut tracker = MarketTracker::new(0.01);
        tracker.decay_halflife_sec = 1.0; // 1 second half-life for test

        // Trade 1 at t = 0
        tracker.update_trade(Trade::new(1_000_000_000, 100.0, 100.0, Side::Buy));
        assert_eq!(tracker.window_buy_volume, 100.0);

        // Trade 2 at t = 1.0s (1_000_000_000 ns later)
        // Previous volume should decay by e^(-1.0) ~= 0.367879, so total ~= 100 * 0.367879 + 50 = 86.78
        tracker.update_trade(Trade::new(2_000_000_000, 100.0, 50.0, Side::Buy));
        assert!(tracker.window_buy_volume < 100.0 + 50.0);
        assert!((tracker.window_buy_volume - (100.0 * (-1.0_f64).exp() + 50.0)).abs() < 1e-6);
    }

    #[test]
    fn test_l2_book_deep_ofi_and_barriers() {
        let mut book1 = L2Book::<5>::new(1_000_000_000);
        book1.set_bid(0, 100.00, 50.0, 5);
        book1.set_bid(1, 99.99, 100.0, 10);
        book1.set_ask(0, 100.02, 40.0, 4);
        book1.set_ask(1, 100.03, 80.0, 8);

        assert!((book1.spread() - 0.02).abs() < 1e-6);
        assert!((book1.mid_price() - 100.01).abs() < 1e-6);
        assert_eq!(book1.total_bid_depth(), 150.0);
        assert_eq!(book1.total_ask_depth(), 120.0);
        assert!(book1.depth_imbalance() > 0.0); // More bids than asks

        // Book 2: Buyer stacks 100 more at best bid
        let mut book2 = book1;
        book2.timestamp_ns = 1_000_100_000;
        book2.set_bid(0, 100.00, 150.0, 15);

        let ofi = book2.deep_ofi(&book1);
        assert!(ofi > 0.0); // Positive OFI from bid buildup

        let barrier_ask = book2.cumulative_barrier_height(Side::Buy, 5);
        assert!(barrier_ask > 0.0);
    }
}

