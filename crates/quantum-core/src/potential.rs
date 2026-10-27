use crate::complex::Complex64;
use crate::matrix::CMatrix;

/// Physics models for quantum tunneling through liquidity walls and unitary evolution.
pub struct QuantumPotential;

impl QuantumPotential {
    /// Normalized Planck constant analog for market microstructural scale
    pub const HBAR_ANALOG: f64 = 1.0;

    /// Computes the quantum tunneling transmission coefficient T in range [0.0, 1.0].
    /// - `energy`: Kinetic energy / flow impulse of incoming aggressive orders.
    /// - `barrier_height`: Depth potential of the liquidity wall (e.g., total resting depth).
    /// - `barrier_width`: Spread distance / price depth to traverse in ticks.
    /// - `effective_mass`: Market inertia / viscosity of the order book.
    pub fn tunneling_probability(
        energy: f64,
        barrier_height: f64,
        barrier_width: f64,
        effective_mass: f64,
    ) -> f64 {
        if energy <= 1e-9 {
            return 0.0;
        }
        if barrier_height <= 1e-9 || barrier_width <= 1e-9 {
            return 1.0;
        }

        let m = effective_mass.max(1e-4);
        let hbar = Self::HBAR_ANALOG;

        if (energy - barrier_height).abs() < 1e-4 {
            // Threshold E ~= V0
            let factor = 1.0 + (m * barrier_height * barrier_width * barrier_width) / (2.0 * hbar * hbar);
            return (1.0 / factor).clamp(0.0, 1.0);
        }

        if energy < barrier_height {
            // Quantum tunneling regime (E < V0)
            let diff = barrier_height - energy;
            let kappa = (2.0 * m * diff).sqrt() / hbar;
            let kappa_a = kappa * barrier_width;

            // Guard against sinh overflow: if kappa_a > 40.0, transmission is physically 0.0
            if kappa_a > 40.0 {
                return 0.0;
            }

            let sinh_val = kappa_a.sinh();
            let denom_factor = (barrier_height * barrier_height) / (4.0 * energy * diff).max(1e-12);
            let denom = 1.0 + denom_factor * sinh_val * sinh_val;
            if denom.is_nan() || denom <= 0.0 {
                0.0
            } else {
                (1.0 / denom).clamp(0.0, 1.0)
            }
        } else {
            // Classical over-barrier transmission with quantum interference (E > V0)
            let diff = energy - barrier_height;
            let k = (2.0 * m * diff).sqrt() / hbar;
            let sin_val = (k * barrier_width).sin();
            let denom_factor = (barrier_height * barrier_height) / (4.0 * energy * diff).max(1e-12);
            let denom = 1.0 + denom_factor * sin_val * sin_val;
            if denom.is_nan() || denom <= 0.0 {
                0.0
            } else {
                (1.0 / denom).clamp(0.0, 1.0)
            }
        }
    }

    /// Exact analytical closed-form unitary propagator U(dt) = exp(-i * H * dt / hbar)
    /// for a 2x2 Hermitian Hamiltonian matrix using Pauli decomposition.
    pub fn unitary_propagator_2x2(hamiltonian: &CMatrix<2>, dt: f64) -> CMatrix<2> {
        let hbar = Self::HBAR_ANALOG;
        let t = dt / hbar;

        let h00 = hamiltonian.data[0][0].re;
        let h11 = hamiltonian.data[1][1].re;
        let h01 = hamiltonian.data[0][1];

        // Pauli decomposition: H = d0 * I + d1 * sigma_x + d2 * sigma_y + d3 * sigma_z
        let d0 = 0.5 * (h00 + h11);
        let d1 = h01.re;
        let d2 = -h01.im;
        let d3 = 0.5 * (h00 - h11);

        let d_norm = (d1 * d1 + d2 * d2 + d3 * d3).sqrt();
        let phase = Complex64::new(0.0, -d0 * t).exp();

        if d_norm < 1e-15 {
            // Pure global phase rotation
            let mut u = CMatrix::zeros();
            u.set(0, 0, phase);
            u.set(1, 1, phase);
            return u;
        }

        let cos_val = (d_norm * t).cos();
        let sin_factor = (d_norm * t).sin() / d_norm;

        // U_unphased = cos(|d|*t)*I - i*sin(|d|*t)/|d| * (d1*sigma_x + d2*sigma_y + d3*sigma_z)
        let i_sin = Complex64::new(0.0, -sin_factor);

        let u00 = (Complex64::from_real(cos_val) + i_sin * Complex64::from_real(d3)) * phase;
        let u01 = (i_sin * Complex64::new(d1, -d2)) * phase;
        let u10 = (i_sin * Complex64::new(d1, d2)) * phase;
        let u11 = (Complex64::from_real(cos_val) - i_sin * Complex64::from_real(d3)) * phase;

        let mut res = CMatrix::zeros();
        res.set(0, 0, u00);
        res.set(0, 1, u01);
        res.set(1, 0, u10);
        res.set(1, 1, u11);
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tunneling_probability_stability() {
        // Zero energy -> 0.0
        assert_eq!(QuantumPotential::tunneling_probability(0.0, 100.0, 1.0, 1.0), 0.0);

        // Huge barrier (where old code would sinh overflow to Infinity/NaN)
        let t_huge = QuantumPotential::tunneling_probability(10.0, 50_000.0, 5.0, 1.0);
        assert_eq!(t_huge, 0.0);
        assert!(!t_huge.is_nan());

        // Threshold E == V0
        let t_thresh = QuantumPotential::tunneling_probability(100.0, 100.0, 1.0, 1.0);
        assert!(t_thresh > 0.0 && t_thresh <= 1.0);
        assert!(!t_thresh.is_nan());

        // Over-barrier E > V0
        let t_over = QuantumPotential::tunneling_probability(150.0, 100.0, 1.0, 1.0);
        assert!(t_over > 0.0 && t_over <= 1.0);
        assert!(!t_over.is_nan());
    }
}

