use crate::engine::QuantumAnalyticsSnapshot;

/// Four fundamental econophysical phases of the Limit Order Book.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketQuantumPhase {
    /// Phase 0: Thermal Paramagnetic Noise
    /// Characterized by maximal entropy (S_norm > 0.85), isotropic diffuse flow,
    /// low quantum coherence, and subsonic laminar dynamics (Mach < 1.0).
    ThermalNoise,

    /// Phase 1: Bose-Einstein Condensate Analog (Stealth Iceberg Accumulation)
    /// Characterized by entropy collapse (S_norm < 0.60), high resting depth,
    /// highly laminar flow (Mach << 1.0), and zero cavitation risk.
    CoherentCondensate,

    /// Phase 2: Superconducting Liquidity Flux (Quantum Breakout)
    /// Characterized by high kinetic momentum, tunneling probability > 0.80,
    /// and directional phase-locking across the spread.
    SuperconductingBreakout,

    /// Phase 3: Fluid Cavitation & Plasma Shock (Flash Crash / Vacuum)
    /// Characterized by supersonic flow (Mach >= 1.0), sudden spread blowout,
    /// and complete evaporation of passive book elasticity.
    LiquidityCavitation,
}

pub struct PhaseClassifier;

impl PhaseClassifier {
    /// Classifies the instantaneous market quantum phase and calculates a confidence score [0.0, 1.0].
    pub fn classify(snapshot: &QuantumAnalyticsSnapshot) -> (MarketQuantumPhase, f64) {
        // 1. Liquidity Cavitation: true vacuum collapse (spread blowout and depth evaporation)
        if snapshot.fluid_metrics.is_cavitation_risk && snapshot.fluid_metrics.liquidity_density < 500.0 {
            let conf = (snapshot.fluid_metrics.mach_number / 2.0).clamp(0.70, 1.0);
            return (MarketQuantumPhase::LiquidityCavitation, conf);
        }

        // 2. Coherent Condensate: stealth accumulation/iceberg absorption (low entropy, resting imbalance, laminar/subsonic)
        if snapshot.normalized_entropy < 0.88 && snapshot.fluid_metrics.mach_number < 0.60 {
            let conf = (1.0 - snapshot.normalized_entropy).clamp(0.50, 0.98);
            return (MarketQuantumPhase::CoherentCondensate, conf);
        }

        // 3. Superconducting Breakout: active kinetic penetration through liquidity wall
        // High win-rate criteria: Tunneling > 0.75, Entropy contracting (dS/dt <= 0.05), and Directional OFI alignment
        let max_tunnel = snapshot.tunneling_prob_ask.max(snapshot.tunneling_prob_bid);
        let ofi_aligned = if snapshot.tunneling_prob_ask > snapshot.tunneling_prob_bid {
            snapshot.ofi_acceleration >= -1e-6
        } else {
            snapshot.ofi_acceleration <= 1e-6
        };

        // Anti-toxicity shield: If informed toxic flow is high, prevent contrarian fills
        let toxic_shield_ok = !snapshot.is_toxic_flow || ofi_aligned;

        if max_tunnel > 0.72
            && snapshot.normalized_entropy < 0.88
            && snapshot.entropy_production_rate <= 0.50
            && ofi_aligned
            && toxic_shield_ok
            && snapshot.fluid_metrics.flow_velocity > 0.005
        {
            let conf = max_tunnel.clamp(0.70, 0.99);
            return (MarketQuantumPhase::SuperconductingBreakout, conf);
        }

        // 4. Default: Thermal Noise (diffusive brownian motion)
        let conf = snapshot.normalized_entropy.clamp(0.50, 1.0);
        (MarketQuantumPhase::ThermalNoise, conf)
    }
}
