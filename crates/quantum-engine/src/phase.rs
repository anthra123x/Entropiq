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
    /// Classifies the instantaneous market quantum phase with strict econophysical consistency.
    pub fn classify(snapshot: &QuantumAnalyticsSnapshot) -> (MarketQuantumPhase, f64) {
        // 1. Phase 3: Liquidity Cavitation (Supersonic Plasma Shock / Void Collapse)
        // Occurs when fluid velocity exceeds acoustic speed in liquidity (Mach >= 1.0),
        // during acute cavitation risk, or during severe toxic spread blowouts.
        let is_supersonic = snapshot.fluid_metrics.mach_number >= 1.0;
        let is_toxic_shock = snapshot.is_toxic_flow && snapshot.vpin >= 0.75;
        if is_supersonic || snapshot.fluid_metrics.is_cavitation_risk || is_toxic_shock {
            let conf = (snapshot.fluid_metrics.mach_number / 2.0).clamp(0.75, 1.0);
            return (MarketQuantumPhase::LiquidityCavitation, conf);
        }

        // 2. Phase 2: Superconducting Liquidity Flux (Quantum Breakout)
        // Physical prerequisite: Macroscopic quantum coherence (BCS analog), low entropy,
        // subsonic laminar regime, zero toxic flow, and directional tunneling penetration.
        let max_tunnel = snapshot.tunneling_prob_ask.max(snapshot.tunneling_prob_bid);
        let ofi_aligned = if snapshot.tunneling_prob_ask > snapshot.tunneling_prob_bid {
            snapshot.ofi_acceleration >= -1e-6
        } else {
            snapshot.ofi_acceleration <= 1e-6
        };

        let has_macroscopic_coherence = snapshot.quantum_coherence >= 0.25;
        let is_low_entropy = snapshot.normalized_entropy <= 0.82 && snapshot.entropy_production_rate <= 0.50;
        let is_laminar_subsonic = snapshot.fluid_metrics.mach_number < 0.95;
        let is_pure_flow = !snapshot.is_toxic_flow && snapshot.vpin < 0.65;

        if max_tunnel > 0.70
            && has_macroscopic_coherence
            && is_low_entropy
            && is_laminar_subsonic
            && is_pure_flow
            && ofi_aligned
            && snapshot.fluid_metrics.flow_velocity > 0.002
        {
            let conf = (max_tunnel * 0.6 + snapshot.quantum_coherence * 0.4).clamp(0.70, 0.99);
            return (MarketQuantumPhase::SuperconductingBreakout, conf);
        }

        // 3. Phase 1: Coherent Condensate (Bose-Einstein Stealth Iceberg Accumulation)
        // Characterized by orderly laminar flow (Mach << 1.0), entropy collapse,
        // moderate-to-high coherence, and absence of toxic predatory flow.
        if snapshot.quantum_coherence >= 0.18
            && snapshot.normalized_entropy < 0.78
            && snapshot.fluid_metrics.mach_number < 0.60
            && snapshot.vpin < 0.60
            && !snapshot.is_toxic_flow
        {
            let conf = (1.0 - snapshot.normalized_entropy).clamp(0.55, 0.95);
            return (MarketQuantumPhase::CoherentCondensate, conf);
        }

        // 4. Phase 0: Thermal Paramagnetic Noise (Default Brownian Thermal Equilibrium)
        // High entropy, decohered mixed state (rho ~ 0.5*I), isotropic random walk.
        let conf = snapshot.normalized_entropy.clamp(0.50, 1.0);
        (MarketQuantumPhase::ThermalNoise, conf)
    }
}
