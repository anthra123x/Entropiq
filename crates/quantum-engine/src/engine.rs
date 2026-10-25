use quantum_core::{CMatrix, Complex64, DensityMatrix, LindbladSolver, QuantumDepthWalk, QuantumPotential};
use market_microstructure::{FluidMetrics, LiquidityFluidModel, MarketTracker, Quote, Side, Trade};
use crate::hamiltonian::MarketHamiltonian;
use crate::phase::{MarketQuantumPhase, PhaseClassifier};

/// Output physics metrics generated at sub-microsecond latency.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuantumAnalyticsSnapshot {
    pub timestamp_ns: u64,
    pub mid_price: f64,
    pub micro_price: f64,
    pub von_neumann_entropy: f64,
    pub normalized_entropy: f64,
    pub entropy_production_rate: f64,
    pub ofi_acceleration: f64,
    pub quantum_coherence: f64,
    pub tunneling_prob_ask: f64,
    pub tunneling_prob_bid: f64,
    pub state_bias: f64,
    pub fluid_metrics: FluidMetrics,
    pub depth_probabilities: [f64; 8],
    pub depth_variance: f64,
    pub active_phase: MarketQuantumPhase,
    pub phase_confidence: f64,
}

/// Unified Quantitative Physics Analytics Engine.
pub struct QuantumMarketEngine {
    pub tracker: MarketTracker,
    pub density_matrix: DensityMatrix<2>,
    pub depth_psi: [Complex64; 8],
    pub prev_normalized_entropy: f64,
    pub last_timestamp_ns: u64,
}

impl QuantumMarketEngine {
    pub fn new(tick_size: f64) -> Self {
        // Initialize at maximally mixed thermal state: 0.5 * I
        let mut init_mat = CMatrix::<2>::zeros();
        init_mat.set(0, 0, Complex64::from_real(0.5));
        init_mat.set(1, 1, Complex64::from_real(0.5));
        let dm = DensityMatrix::<2>::from_matrix_normalized(init_mat).unwrap();

        // Initialize 8-level depth walk localized across the spread (sites 3 and 4)
        let inv_sqrt2 = 1.0 / (2.0_f64).sqrt();
        let mut depth_psi = [Complex64::ZERO; 8];
        depth_psi[3] = Complex64::from_real(inv_sqrt2); // Best Bid
        depth_psi[4] = Complex64::from_real(inv_sqrt2); // Best Ask

        Self {
            tracker: MarketTracker::new(tick_size),
            density_matrix: dm,
            depth_psi,
            prev_normalized_entropy: 1.0,
            last_timestamp_ns: 0,
        }
    }

    #[inline(always)]
    pub fn on_quote(&mut self, quote: Quote) -> Option<QuantumAnalyticsSnapshot> {
        let dt_sec = if self.last_timestamp_ns > 0 {
            ((quote.timestamp_ns.saturating_sub(self.last_timestamp_ns)) as f64) * 1e-9
        } else {
            0.001
        };
        self.last_timestamp_ns = quote.timestamp_ns;

        self.tracker.update_quote(quote);

        // 1. Continuous Lindblad evolution under Hamiltonian and resting imbalance relaxation
        let total_size = (quote.bid_size + quote.ask_size).max(1e-6);
        let p_bid = (quote.bid_size / total_size).clamp(0.001, 0.999);
        let p_ask = 1.0 - p_bid;

        // Quote imbalance acts as a continuous directional pumping field
        let gamma_q_buy = (p_bid - 0.5).max(0.0) * 4.0;
        let gamma_q_sell = (p_ask - 0.5).max(0.0) * 4.0;

        let h = MarketHamiltonian::build(&self.tracker);
        let jumps = LindbladSolver::market_jump_operators_2x2(1.0, gamma_q_buy, gamma_q_sell);

        if let Ok(updated_rho) = LindbladSolver::step_rk4(&self.density_matrix, &h, &jumps, dt_sec.min(0.1).max(0.0001)) {
            self.density_matrix = updated_rho;
        }

        // 2. Continuous-Time Quantum Walk (CTQW) on 8-level depth lattice
        // Extrapolate depth profile from top of book & accumulated OFI
        let bid_b3 = (quote.bid_size * 1.5).max(1.0);
        let bid_b2 = (quote.bid_size * 1.2).max(1.0);
        let bid_b1 = (quote.bid_size * 1.0).max(1.0);
        let bid_b0 = quote.bid_size.max(0.1);

        let ask_a0 = quote.ask_size.max(0.1);
        let ask_a1 = (quote.ask_size * 1.0).max(1.0);
        let ask_a2 = (quote.ask_size * 1.2).max(1.0);
        let ask_a3 = (quote.ask_size * 1.5).max(1.0);

        let spread_ticks = (quote.spread() / self.tracker.tick_size).max(1.0);
        let h_walk = QuantumDepthWalk::build_lattice_hamiltonian(
            [bid_b3, bid_b2, bid_b1, bid_b0],
            [ask_a0, ask_a1, ask_a2, ask_a3],
            5.0, // Hopping mobility
            spread_ticks,
        );

        self.depth_psi = QuantumDepthWalk::propagate(&h_walk, &self.depth_psi, dt_sec.min(0.01).max(0.0001));

        self.compute_snapshot(dt_sec)
    }

    #[inline(always)]
    pub fn on_trade(&mut self, trade: Trade) {
        self.tracker.update_trade(trade);

        let total_depth = if let Some(q) = self.tracker.current_quote {
            (q.bid_size + q.ask_size).max(1.0)
        } else {
            100.0
        };

        // Immediate dissipation injection into density matrix via jump operator
        let gamma_buy = if trade.taker_side == Side::Buy { (trade.size / total_depth).min(2.0) } else { 0.0 };
        let gamma_sell = if trade.taker_side == Side::Sell { (trade.size / total_depth).min(2.0) } else { 0.0 };

        let jumps = LindbladSolver::market_jump_operators_2x2(0.1, gamma_buy, gamma_sell);
        let h = MarketHamiltonian::build(&self.tracker);

        if let Ok(updated_rho) = LindbladSolver::step_rk4(&self.density_matrix, &h, &jumps, 0.01) {
            self.density_matrix = updated_rho;
        }

        // Perturb CTQW depth wavepacket upon trade impact
        let target_site = if trade.taker_side == Side::Buy { 4 } else { 3 };
        let impact_phase = Complex64::new(0.0, 0.5).exp();
        self.depth_psi[target_site] = self.depth_psi[target_site] * impact_phase;
    }

    /// Computes full quantum physical, hydrodynamic, and phase-transition state snapshot.
    pub fn compute_snapshot(&mut self, dt_sec: f64) -> Option<QuantumAnalyticsSnapshot> {
        let quote = self.tracker.current_quote?;

        // 1. Quantum state observables from Lindblad-evolved density matrix
        let vn_entropy = self.density_matrix.von_neumann_entropy();
        let norm_entropy = self.density_matrix.normalized_entropy();
        let coherence = self.density_matrix.quantum_coherence();

        let entropy_rate = norm_entropy - self.prev_normalized_entropy;
        self.prev_normalized_entropy = norm_entropy;
        let ofi_accel = self.tracker.ofi_acceleration;

        // 2. Quantum Tunneling against Bid & Ask liquidity barriers
        let kinetic_energy = self.tracker.kinetic_energy();
        let (v_ask, width_ask) = self.tracker.barrier_potential(Side::Buy);
        let (v_bid, width_bid) = self.tracker.barrier_potential(Side::Sell);

        let t_ask = QuantumPotential::tunneling_probability(kinetic_energy, v_ask, width_ask, 1.0);
        let t_bid = QuantumPotential::tunneling_probability(kinetic_energy, v_bid, width_bid, 1.0);
        let state_bias = t_ask - t_bid;

        // 3. Fluid dynamics & liquidity cavitation
        let total_depth = quote.bid_size + quote.ask_size;
        let price_delta = if let Some(prev) = self.tracker.previous_quote {
            quote.micro_price() - prev.micro_price()
        } else {
            0.0
        };

        let fluid_metrics = LiquidityFluidModel::compute_metrics(
            total_depth,
            quote.spread(),
            price_delta,
            dt_sec,
        );

        // 4. Depth quantum walk wavepacket distribution & spatial variance
        let depth_probs = QuantumDepthWalk::depth_probabilities(&self.depth_psi);
        let mut mean_x = 0.0;
        for i in 0..8 {
            mean_x += (i as f64) * depth_probs[i];
        }
        let mut variance = 0.0;
        for i in 0..8 {
            let diff = (i as f64) - mean_x;
            variance += diff * diff * depth_probs[i];
        }

        let mut snapshot = QuantumAnalyticsSnapshot {
            timestamp_ns: quote.timestamp_ns,
            mid_price: quote.mid_price(),
            micro_price: quote.micro_price(),
            von_neumann_entropy: vn_entropy,
            normalized_entropy: norm_entropy,
            entropy_production_rate: entropy_rate,
            ofi_acceleration: ofi_accel,
            quantum_coherence: coherence,
            tunneling_prob_ask: t_ask,
            tunneling_prob_bid: t_bid,
            state_bias,
            fluid_metrics,
            depth_probabilities: depth_probs,
            depth_variance: variance,
            active_phase: MarketQuantumPhase::ThermalNoise,
            phase_confidence: 0.0,
        };

        // 5. Econophysical Phase Classification
        let (phase, conf) = PhaseClassifier::classify(&snapshot);
        snapshot.active_phase = phase;
        snapshot.phase_confidence = conf;

        Some(snapshot)
    }
}
