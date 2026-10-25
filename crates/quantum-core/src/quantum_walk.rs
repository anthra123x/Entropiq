use crate::complex::Complex64;
use crate::matrix::CMatrix;

/// Continuous-Time Quantum Walk (CTQW) on an N-level Order Book Depth Lattice.
/// Sites 0..3: Bid levels [Bid-3, Bid-2, Bid-1, Best-Bid]
/// Sites 4..7: Ask levels [Best-Ask, Ask+1, Ask+2, Ask+3]
pub struct QuantumDepthWalk;

impl QuantumDepthWalk {
    pub const SITES: usize = 8;

    /// Builds the Tight-Binding Hamiltonian H_walk for the 8-level depth lattice.
    /// - `bid_depths`: Resting sizes at [Bid-3, Bid-2, Bid-1, Best-Bid]
    /// - `ask_depths`: Resting sizes at [Best-Ask, Ask+1, Ask+2, Ask+3]
    /// - `hopping_rate`: Inter-level capital mobility
    /// - `spread_ticks`: Distance across the spread (attenuates center hopping)
    pub fn build_lattice_hamiltonian(
        bid_depths: [f64; 4],
        ask_depths: [f64; 4],
        hopping_rate: f64,
        spread_ticks: f64,
    ) -> CMatrix<8> {
        let mut h = CMatrix::<8>::zeros();

        // 1. Diagonal: local potential proportional to resting volume
        for i in 0..4 {
            h.set(i, i, Complex64::from_real(bid_depths[i]));
        }
        for i in 0..4 {
            h.set(4 + i, 4 + i, Complex64::from_real(ask_depths[i]));
        }

        // 2. Nearest-neighbor hopping within the same book side
        let hop = Complex64::from_real(-hopping_rate);
        for i in 0..3 {
            // Bid side connections
            h.set(i, i + 1, hop);
            h.set(i + 1, i, hop);
            // Ask side connections
            h.set(4 + i, 4 + i + 1, hop);
            h.set(4 + i + 1, 4 + i, hop);
        }

        // 3. Across-the-spread tunneling between Best-Bid (index 3) and Best-Ask (index 4)
        // Attenuated exponentially by spread distance
        let spread_attenuation = (-spread_ticks.max(1.0) * 0.5).exp();
        let spread_hop = Complex64::from_real(-hopping_rate * spread_attenuation);
        h.set(3, 4, spread_hop);
        h.set(4, 3, spread_hop);

        h
    }

    /// Propagates state vector psi(t+dt) using unitary Taylor expansion up to 4th order.
    /// psi(t+dt) = (I - i*H*dt - (H*dt)^2 / 2! + i*(H*dt)^3 / 3! + (H*dt)^4 / 4!) * psi(t)
    pub fn propagate(
        hamiltonian: &CMatrix<8>,
        psi: &[Complex64; 8],
        dt: f64,
    ) -> [Complex64; 8] {
        // Compute H * psi
        let h_psi = Self::mat_vec_mul(hamiltonian, psi);

        // H^2 * psi
        let h2_psi = Self::mat_vec_mul(hamiltonian, &h_psi);

        // H^3 * psi
        let h3_psi = Self::mat_vec_mul(hamiltonian, &h2_psi);

        // H^4 * psi
        let h4_psi = Self::mat_vec_mul(hamiltonian, &h3_psi);

        let dt2 = dt * dt;
        let dt3 = dt2 * dt;
        let dt4 = dt3 * dt;

        let mut res = [Complex64::ZERO; 8];
        for i in 0..8 {
            // Term 0: I * psi
            let mut val = psi[i];

            // Term 1: -i * dt * H*psi
            let term1 = Complex64::new(h_psi[i].im, -h_psi[i].re) * Complex64::from_real(dt);
            val = val + term1;

            // Term 2: -0.5 * dt^2 * H^2*psi
            let term2 = h2_psi[i] * Complex64::from_real(-0.5 * dt2);
            val = val + term2;

            // Term 3: +i * (dt^3 / 6) * H^3*psi
            let term3 = Complex64::new(-h3_psi[i].im, h3_psi[i].re) * Complex64::from_real(dt3 / 6.0);
            val = val + term3;

            // Term 4: +(dt^4 / 24) * H^4*psi
            let term4 = h4_psi[i] * Complex64::from_real(dt4 / 24.0);
            val = val + term4;

            res[i] = val;
        }

        // Renormalize probability distribution sum |psi_i|^2 = 1.0
        let mut norm_sq = 0.0;
        for c in &res {
            norm_sq += c.norm_sq();
        }
        let norm = if norm_sq > 1e-15 { norm_sq.sqrt() } else { 1.0 };
        for i in 0..8 {
            res[i] = res[i] / Complex64::from_real(norm);
        }

        res
    }

    /// Computes spatial dispersion / wave packet penetration across depth levels.
    /// Returns probability array [P_0, P_1, ..., P_7].
    pub fn depth_probabilities(psi: &[Complex64; 8]) -> [f64; 8] {
        let mut probs = [0.0; 8];
        for i in 0..8 {
            probs[i] = psi[i].norm_sq();
        }
        probs
    }

    #[inline(always)]
    fn mat_vec_mul(mat: &CMatrix<8>, vec: &[Complex64; 8]) -> [Complex64; 8] {
        let mut out = [Complex64::ZERO; 8];
        for r in 0..8 {
            let mut sum = Complex64::ZERO;
            for c in 0..8 {
                sum = sum + mat.data[r][c] * vec[c];
            }
            out[r] = sum;
        }
        out
    }
}
