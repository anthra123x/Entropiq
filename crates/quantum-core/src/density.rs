use crate::complex::Complex64;
use crate::matrix::CMatrix;

/// Quantum Density Matrix representing pure or mixed market quantum states.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DensityMatrix<const N: usize> {
    matrix: CMatrix<N>,
}

impl<const N: usize> DensityMatrix<N> {
    /// Constructs a density matrix from a raw CMatrix, normalizing trace to 1.0.
    pub fn from_matrix_normalized(mut mat: CMatrix<N>) -> Result<Self, &'static str> {
        let tr = mat.trace().re;
        if tr <= 1e-15 {
            return Err("Trace must be strictly positive to form a valid density matrix");
        }
        let inv_tr = Complex64::from_real(1.0 / tr);
        for r in 0..N {
            for c in 0..N {
                mat.set(r, c, mat.get(r, c) * inv_tr);
            }
            // Ensure diagonal is purely real and non-negative
            let diag_re = mat.get(r, r).re.max(0.0);
            mat.set(r, r, Complex64::from_real(diag_re));
        }

        // Positivity preservation: ensure |rho_01|^2 <= rho_00 * rho_11
        if N == 2 {
            let p0 = mat.get(0, 0).re;
            let p1 = mat.get(1, 1).re;
            let max_off = (p0 * p1).sqrt();
            let curr_off = mat.get(0, 1).norm();
            if curr_off > max_off && curr_off > 1e-15 {
                let scale = max_off / curr_off;
                mat.set(0, 1, mat.get(0, 1) * Complex64::from_real(scale));
                mat.set(1, 0, mat.get(1, 0) * Complex64::from_real(scale));
            }
        }

        Ok(Self { matrix: mat })
    }

    /// Creates a pure state density matrix rho = |psi><psi|.
    /// The input state vector is automatically normalized.
    pub fn from_pure_state(psi: &[Complex64; N]) -> Self {
        let mut norm_sq = 0.0;
        for c in psi {
            norm_sq += c.norm_sq();
        }
        let norm = if norm_sq > 0.0 { norm_sq.sqrt() } else { 1.0 };

        let mut mat = CMatrix::zeros();
        for r in 0..N {
            let bra = psi[r].conjugate();
            for c in 0..N {
                let ket = psi[c];
                mat.set(c, r, (ket * bra) / Complex64::from_real(norm * norm));
            }
        }
        Self { matrix: mat }
    }

    #[inline(always)]
    pub fn matrix(&self) -> &CMatrix<N> {
        &self.matrix
    }

    /// Computes quantum purity: gamma = Tr(rho^2).
    /// Pure state: gamma = 1.0. Maximally mixed state: gamma = 1.0 / N.
    pub fn purity(&self) -> f64 {
        let rho_sq = self.matrix.matmul(&self.matrix);
        rho_sq.trace().re
    }
}

impl DensityMatrix<2> {
    /// Computes the exact analytical von Neumann entropy for 2-level state:
    /// S(rho) = -sum(lambda_i * ln(lambda_i)).
    /// Result is in nats (bounded between 0.0 and ln(2) ~= 0.693147).
    pub fn von_neumann_entropy(&self) -> f64 {
        let eigenvalues = self.matrix.hermitian_eigenvalues_2x2();
        let mut entropy = 0.0;

        for &lambda in &eigenvalues {
            // Numerical tolerance: clamp small values to 0.0
            if lambda > 1e-12 {
                entropy -= lambda * lambda.ln();
            }
        }
        entropy.max(0.0)
    }

    /// Normalized von Neumann entropy in range [0.0, 1.0]:
    /// 0.0 = Pure state (unidirectional informed order flow)
    /// 1.0 = Maximally mixed state (complete thermal market noise / 50-50 equilibrium)
    pub fn normalized_entropy(&self) -> f64 {
        const LN_2: f64 = 0.6931471805599453;
        (self.von_neumann_entropy() / LN_2).clamp(0.0, 1.0)
    }

    /// Quantum Coherence: measure of off-diagonal quantum superposition (interference).
    /// In trading: measure of order book latent cross-intent between bid and ask before collapse.
    pub fn quantum_coherence(&self) -> f64 {
        // l1-norm of off-diagonal elements
        2.0 * self.matrix.get(0, 1).norm()
    }
}
