use market_microstructure::Side;
use crate::engine::QuantumAnalyticsSnapshot;
use crate::phase::MarketQuantumPhase;

/// Smart Order Router (SOR) execution intent generated from real-time microstructural state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrderIntent {
    /// Cross the spread aggressively with a market or aggressive marketable limit order.
    AggressiveTaker {
        side: Side,
        size_ratio: f64,
        urgency: f64,
    },
    /// Post liquidity inside or at the top of the book to capture spread without paying taker fees.
    PassiveMaker {
        side: Side,
        target_price: f64,
        size_ratio: f64,
    },
    /// Market conditions are neutral; maintain or idle positions.
    NeutralHold,
    /// High adverse selection or fluid cavitation detected: cancel all resting orders immediately.
    EmergencyCancelAll {
        reason: &'static str,
    },
}

/// Smart Order Routing (SOR) engine evaluating quantitative phase transitions and quantum barriers.
pub struct SmartOrderRouter;

impl SmartOrderRouter {
    /// Computes the optimal execution intent for a given quantum analytics snapshot.
    pub fn route_intent(
        snapshot: &QuantumAnalyticsSnapshot,
        best_bid: f64,
        best_ask: f64,
        base_size_multiplier: f64,
    ) -> OrderIntent {
        // 1. Safety Filter: Cavitation shock (Mach >= 1.0 or LiquidityCavitation phase) -> Emergency retreat
        if snapshot.active_phase == MarketQuantumPhase::LiquidityCavitation
            || snapshot.fluid_metrics.is_cavitation_risk
            || snapshot.fluid_metrics.mach_number >= 1.0
        {
            return OrderIntent::EmergencyCancelAll {
                reason: "Fluid cavitation shock (Mach >= 1.0)",
            };
        }

        // 2. Safety Filter: High adverse selection / toxic flow (healthy threshold: 0.70)
        if snapshot.vpin >= 0.70 {
            return OrderIntent::EmergencyCancelAll {
                reason: "Toxic flow detected (VPIN >= 0.70)",
            };
        }

        let spread = (best_ask - best_bid).max(0.01);
        let mid = (best_ask + best_bid) * 0.5;

        // 3. SuperconductingBreakout Regime: Aggressive Taker
        // High tunneling probability through the barrier + strong directional state bias
        if snapshot.active_phase == MarketQuantumPhase::SuperconductingBreakout {
            if snapshot.tunneling_prob_ask >= 0.75 && snapshot.state_bias > 0.15 {
                let urgency = snapshot.tunneling_prob_ask;
                let size = (snapshot.phase_confidence * base_size_multiplier).clamp(0.1, 2.0);
                return OrderIntent::AggressiveTaker {
                    side: Side::Buy,
                    size_ratio: size,
                    urgency,
                };
            } else if snapshot.tunneling_prob_bid >= 0.75 && snapshot.state_bias < -0.15 {
                let urgency = snapshot.tunneling_prob_bid;
                let size = (snapshot.phase_confidence * base_size_multiplier).clamp(0.1, 2.0);
                return OrderIntent::AggressiveTaker {
                    side: Side::Sell,
                    size_ratio: size,
                    urgency,
                };
            }
        }

        // 4. CoherentCondensate Regime: Passive Maker Spread Capture
        // Stealth accumulation detected (entropy collapse) and flow is laminar: post passive limit orders
        if snapshot.active_phase == MarketQuantumPhase::CoherentCondensate && !snapshot.is_toxic_flow {
            if snapshot.state_bias > 0.10 && snapshot.tunneling_prob_ask < 0.60 {
                // Skewed positive, but barrier holds: post at Best Bid or just above
                let target_price = (best_bid + spread * 0.1).min(mid);
                return OrderIntent::PassiveMaker {
                    side: Side::Buy,
                    target_price,
                    size_ratio: base_size_multiplier.clamp(0.1, 1.0),
                };
            } else if snapshot.state_bias < -0.10 && snapshot.tunneling_prob_bid < 0.60 {
                // Skewed negative, but barrier holds: post at Best Ask or just below
                let target_price = (best_ask - spread * 0.1).max(mid);
                return OrderIntent::PassiveMaker {
                    side: Side::Sell,
                    target_price,
                    size_ratio: base_size_multiplier.clamp(0.1, 1.0),
                };
            }
        }

        // 5. Thermal equilibrium or unaligned state -> Hold
        OrderIntent::NeutralHold
    }
}
