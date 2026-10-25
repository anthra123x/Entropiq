pub mod complex;
pub mod density;
pub mod lindblad;
pub mod matrix;
pub mod potential;
pub mod quantum_walk;

pub use complex::Complex64;
pub use density::DensityMatrix;
pub use lindblad::LindbladSolver;
pub use matrix::CMatrix;
pub use potential::QuantumPotential;
pub use quantum_walk::QuantumDepthWalk;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complex_operations() {
        let a = Complex64::new(3.0, 4.0);
        let b = Complex64::new(1.0, -2.0);

        let sum = a + b;
        assert_eq!(sum, Complex64::new(4.0, 2.0));

        let norm = a.norm();
        assert!((norm - 5.0).abs() < 1e-12);

        let prod = a * b;
        assert_eq!(prod, Complex64::new(11.0, -2.0));
    }

    #[test]
    fn test_density_matrix_and_von_neumann_entropy() {
        // Pure state: |psi> = [1, 0]^T
        let psi = [Complex64::ONE, Complex64::ZERO];
        let rho_pure = DensityMatrix::<2>::from_pure_state(&psi);

        assert!((rho_pure.purity() - 1.0).abs() < 1e-10);
        assert!(rho_pure.von_neumann_entropy() < 1e-10);
        assert!(rho_pure.normalized_entropy() < 1e-10);

        // Maximally mixed state: 0.5 * I
        let mut mat = CMatrix::<2>::zeros();
        mat.set(0, 0, Complex64::from_real(0.5));
        mat.set(1, 1, Complex64::from_real(0.5));
        let rho_mixed = DensityMatrix::<2>::from_matrix_normalized(mat).unwrap();

        assert!((rho_mixed.purity() - 0.5).abs() < 1e-10);
        assert!((rho_mixed.normalized_entropy() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_lindblad_thermal_decoherence() {
        // Start from a pure superposition state |+> = (|0> + |1>)/sqrt(2)
        let inv_sqrt2 = 1.0 / (2.0_f64).sqrt();
        let psi = [Complex64::from_real(inv_sqrt2), Complex64::from_real(inv_sqrt2)];
        let rho_initial = DensityMatrix::<2>::from_pure_state(&psi);

        assert!(rho_initial.normalized_entropy() < 1e-10);
        assert!((rho_initial.quantum_coherence() - 1.0).abs() < 1e-10);

        // Subject to thermal dephasing (market noise)
        let h = CMatrix::<2>::zeros();
        let jumps = LindbladSolver::market_jump_operators_2x2(2.0, 0.0, 0.0);

        // Integrate multiple RK4 steps
        let mut rho = rho_initial;
        for _ in 0..50 {
            rho = LindbladSolver::step_rk4(&rho, &h, &jumps, 0.05).unwrap();
        }

        // Dephasing should destroy coherence and increase entropy towards 1.0
        assert!(rho.quantum_coherence() < 0.2);
        assert!(rho.normalized_entropy() > 0.85);
    }

    #[test]
    fn test_quantum_depth_walk_propagation() {
        let bid_depths = [100.0, 80.0, 50.0, 20.0];
        let ask_depths = [20.0, 50.0, 80.0, 100.0];
        let h_walk = QuantumDepthWalk::build_lattice_hamiltonian(bid_depths, ask_depths, 10.0, 1.0);

        // Initial shock concentrated at Best-Ask (site 4)
        let mut psi0 = [Complex64::ZERO; 8];
        psi0[4] = Complex64::ONE;

        // Propagate for dt = 0.05
        let psi_t = QuantumDepthWalk::propagate(&h_walk, &psi0, 0.05);
        let probs = QuantumDepthWalk::depth_probabilities(&psi_t);

        // Total probability must be conserved to 1.0
        let total_prob: f64 = probs.iter().sum();
        assert!((total_prob - 1.0).abs() < 1e-6);

        // Probability must have dispersed from site 4 to neighboring Ask levels (site 5)
        assert!(probs[5] > 0.01);
    }
}
