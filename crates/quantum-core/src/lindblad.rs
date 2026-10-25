use crate::complex::Complex64;
use crate::density::DensityMatrix;
use crate::matrix::CMatrix;

/// Master Equation in Lindblad form for open quantum systems:
/// d(rho)/dt = -i [H, rho] + sum_k ( L_k rho L_k^† - 0.5 {L_k^† L_k, rho} )
pub struct LindbladSolver;

impl LindbladSolver {
    /// Computes the time derivative d(rho)/dt given Hamiltonian H and jump operators L_k.
    pub fn d_rho_dt<const N: usize>(
        rho: &CMatrix<N>,
        hamiltonian: &CMatrix<N>,
        jump_operators: &[CMatrix<N>],
    ) -> CMatrix<N> {
        // 1. Unitary von Neumann component: -i [H, rho] = -i (H*rho - rho*H)
        let h_rho = hamiltonian.matmul(rho);
        let rho_h = rho.matmul(hamiltonian);
        let mut commutator = CMatrix::zeros();
        for r in 0..N {
            for c in 0..N {
                let diff = h_rho.data[r][c] - rho_h.data[r][c];
                // Multiply by -i: (re, im) * (0, -1) = (im, -re)
                commutator.set(r, c, Complex64::new(diff.im, -diff.re));
            }
        }

        // 2. Dissipator sum: sum_k ( L_k rho L_k^† - 0.5 (L_k^† L_k rho + rho L_k^† L_k) )
        let mut dissipator = CMatrix::zeros();

        for l in jump_operators {
            let l_dagger = l.conjugate_transpose();
            let l_rho_ldag = l.matmul(rho).matmul(&l_dagger);

            let ldag_l = l_dagger.matmul(l);
            let ldag_l_rho = ldag_l.matmul(rho);
            let rho_ldag_l = rho.matmul(&ldag_l);

            for r in 0..N {
                for c in 0..N {
                    let anti_comm = (ldag_l_rho.data[r][c] + rho_ldag_l.data[r][c]) * Complex64::from_real(0.5);
                    let term = l_rho_ldag.data[r][c] - anti_comm;
                    dissipator.set(r, c, dissipator.data[r][c] + term);
                }
            }
        }

        commutator.add_mat(&dissipator)
    }

    /// Single integration step using 4th-order Runge-Kutta (RK4).
    /// Preserves trace normalization and Hermiticity.
    pub fn step_rk4<const N: usize>(
        density: &DensityMatrix<N>,
        hamiltonian: &CMatrix<N>,
        jump_operators: &[CMatrix<N>],
        dt: f64,
    ) -> Result<DensityMatrix<N>, &'static str> {
        let rho0 = *density.matrix();

        // k1 = f(rho0)
        let k1 = Self::d_rho_dt(&rho0, hamiltonian, jump_operators);

        // rho1 = rho0 + 0.5 * dt * k1
        let rho1 = rho0.add_mat(&k1.scale(Complex64::from_real(0.5 * dt)));
        let k2 = Self::d_rho_dt(&rho1, hamiltonian, jump_operators);

        // rho2 = rho0 + 0.5 * dt * k2
        let rho2 = rho0.add_mat(&k2.scale(Complex64::from_real(0.5 * dt)));
        let k3 = Self::d_rho_dt(&rho2, hamiltonian, jump_operators);

        // rho3 = rho0 + dt * k3
        let rho3 = rho0.add_mat(&k3.scale(Complex64::from_real(dt)));
        let k4 = Self::d_rho_dt(&rho3, hamiltonian, jump_operators);

        // Update: rho(t+dt) = rho0 + (dt/6) * (k1 + 2*k2 + 2*k3 + k4)
        let mut delta = CMatrix::zeros();
        for r in 0..N {
            for c in 0..N {
                let sum = k1.data[r][c]
                    + k2.data[r][c] * Complex64::from_real(2.0)
                    + k3.data[r][c] * Complex64::from_real(2.0)
                    + k4.data[r][c];
                delta.set(r, c, sum * Complex64::from_real(dt / 6.0));
            }
        }

        let updated_raw = rho0.add_mat(&delta);
        DensityMatrix::from_matrix_normalized(updated_raw)
    }

    /// Factory for typical market microstructure 2x2 Lindblad jump operators:
    /// 1. Thermal dephasing (market noise causing decoherence)
    /// 2. Buy flow jump operator (Ask absorption |Bid><Ask|)
    /// 3. Sell flow jump operator (Bid absorption |Ask><Bid|)
    pub fn market_jump_operators_2x2(
        gamma_dephase: f64,
        gamma_buy: f64,
        gamma_sell: f64,
    ) -> [CMatrix<2>; 3] {
        // Dephasing operator: sqrt(gamma) * sigma_z
        let mut l_dephase = CMatrix::<2>::zeros();
        let amp_dep = gamma_dephase.max(0.0).sqrt();
        l_dephase.set(0, 0, Complex64::from_real(amp_dep));
        l_dephase.set(1, 1, Complex64::from_real(-amp_dep));

        // Buy jump operator: buyer hits ask -> populates bid state
        let mut l_buy = CMatrix::<2>::zeros();
        let amp_buy = gamma_buy.max(0.0).sqrt();
        l_buy.set(0, 1, Complex64::from_real(amp_buy));

        // Sell jump operator: seller hits bid -> populates ask state
        let mut l_sell = CMatrix::<2>::zeros();
        let amp_sell = gamma_sell.max(0.0).sqrt();
        l_sell.set(1, 0, Complex64::from_real(amp_sell));

        [l_dephase, l_buy, l_sell]
    }
}
