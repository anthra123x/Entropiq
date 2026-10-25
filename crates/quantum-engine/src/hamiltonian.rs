use quantum_core::{CMatrix, Complex64};
use market_microstructure::{MarketTracker, Side};

/// Generates the Market 2x2 Hermitian Hamiltonian matrix H from microstructure.
/// H = [ V_bid                 Omega * e^(i*phi) ]
///     [ Omega * e^(-i*phi)    V_ask             ]
pub struct MarketHamiltonian;

impl MarketHamiltonian {
    /// Builds standard localized 2x2 Market Hamiltonian.
    pub fn build(tracker: &MarketTracker) -> CMatrix<2> {
        Self::build_with_cross_asset(tracker, 0.0)
    }

    /// Builds 2x2 Market Hamiltonian with Cross-Asset Lead-Lag Entanglement (e.g., SPY index bias).
    /// External bias polarizes the diagonal barrier potential via Zeeman splitting analog.
    pub fn build_with_cross_asset(tracker: &MarketTracker, external_index_bias: f64) -> CMatrix<2> {
        let (v_ask, _) = tracker.barrier_potential(Side::Buy);
        let (v_bid, _) = tracker.barrier_potential(Side::Sell);

        // Kinetic impulse coupling (Rabi frequency analog Omega)
        let kinetic_energy = tracker.kinetic_energy();
        let omega = kinetic_energy.sqrt();

        // Phase angle phi from accumulated OFI (order flow imbalance)
        let ofi_clamped = tracker.accumulated_ofi.clamp(-1000.0, 1000.0);
        let phi = (ofi_clamped / 1000.0) * std::f64::consts::PI;

        // Energy scale normalization to maintain bounded spectral radius <= 1.0
        let total_scale = (v_ask + v_bid + omega).max(1.0);
        let b_ext = external_index_bias.clamp(-1.0, 1.0) * 0.20;

        // External market index drift modulates effective potential barriers
        let norm_v_bid = (v_bid / total_scale - b_ext).max(0.01);
        let norm_v_ask = (v_ask / total_scale + b_ext).max(0.01);
        let norm_omega = omega / total_scale;

        let coupling = Complex64::new(norm_omega * phi.cos(), norm_omega * phi.sin());

        let mut h = CMatrix::zeros();
        h.set(0, 0, Complex64::from_real(norm_v_bid));
        h.set(1, 1, Complex64::from_real(norm_v_ask));
        h.set(0, 1, coupling);
        h.set(1, 0, coupling.conjugate());

        h
    }
}
