/// Institutional-grade quantitative auditor implementing E[R], Sharpe, Calmar, and Drawdown.
#[derive(Debug, Clone, Default)]
pub struct PortfolioAuditor {
    pub total_trades: u64,
    pub winning_trades: u64,
    pub losing_trades: u64,
    pub sum_winning_returns: f64,
    pub sum_losing_returns: f64,

    pub peak_equity: f64,
    pub max_drawdown: f64,
    pub current_equity: f64,

    pub returns_history: Vec<f64>,
}

impl PortfolioAuditor {
    pub fn new(initial_capital: f64) -> Self {
        Self {
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            sum_winning_returns: 0.0,
            sum_losing_returns: 0.0,
            peak_equity: initial_capital,
            max_drawdown: 0.0,
            current_equity: initial_capital,
            returns_history: Vec::with_capacity(50_000),
        }
    }

    /// Records an executed trade outcome.
    pub fn record_trade(&mut self, pnl: f64, return_fraction: f64) {
        self.total_trades += 1;
        self.current_equity += pnl;

        if self.current_equity > self.peak_equity {
            self.peak_equity = self.current_equity;
        }

        let dd = (self.peak_equity - self.current_equity) / self.peak_equity;
        if dd > self.max_drawdown {
            self.max_drawdown = dd;
        }

        if return_fraction >= 0.0 {
            self.winning_trades += 1;
            self.sum_winning_returns += return_fraction;
        } else {
            self.losing_trades += 1;
            self.sum_losing_returns += return_fraction.abs();
        }

        if self.returns_history.len() < 10_000 {
            self.returns_history.push(return_fraction);
        } else {
            let idx = (self.total_trades as usize) % 10_000;
            self.returns_history[idx] = return_fraction;
        }
    }

    /// Win Rate in range [0.0, 1.0]
    pub fn win_rate(&self) -> f64 {
        if self.total_trades == 0 { 0.0 } else { self.winning_trades as f64 / self.total_trades as f64 }
    }

    /// Loss Rate in range [0.0, 1.0]
    pub fn loss_rate(&self) -> f64 {
        if self.total_trades == 0 { 0.0 } else { self.losing_trades as f64 / self.total_trades as f64 }
    }

    /// Average net return on winning trades: R_bar_ganador_neto
    pub fn avg_win_return(&self) -> f64 {
        if self.winning_trades == 0 { 0.0 } else { self.sum_winning_returns / self.winning_trades as f64 }
    }

    /// Average net return on losing trades: R_bar_perdedor_neto
    pub fn avg_loss_return(&self) -> f64 {
        if self.losing_trades == 0 { 0.0 } else { self.sum_losing_returns / self.losing_trades as f64 }
    }

    /// Mathematical Expectancy: E[R] = (WinRate * R_win) - (LossRate * R_loss)
    pub fn mathematical_expectancy(&self) -> f64 {
        (self.win_rate() * self.avg_win_return()) - (self.loss_rate() * self.avg_loss_return())
    }

    /// Profit Factor: gross profits / gross losses
    pub fn profit_factor(&self) -> f64 {
        if self.sum_losing_returns <= 1e-9 {
            99.99
        } else {
            self.sum_winning_returns / self.sum_losing_returns
        }
    }

    /// Annualized Sharpe Ratio: (mean / std) * sqrt(N_periods_per_year)
    pub fn sharpe_ratio(&self, annualization_factor: f64) -> f64 {
        if self.returns_history.len() < 2 {
            return 0.0;
        }
        let n = self.returns_history.len() as f64;
        let mean = self.returns_history.iter().sum::<f64>() / n;

        let var = self.returns_history.iter().map(|&r| (r - mean) * (r - mean)).sum::<f64>() / (n - 1.0);
        let std = var.sqrt();

        if std < 1e-9 { 0.0 } else { (mean / std) * annualization_factor.sqrt() }
    }

    /// Calmar Ratio: CAGR / Max Drawdown
    pub fn calmar_ratio(&self, cagr: f64) -> f64 {
        if self.max_drawdown <= 1e-6 { 99.99 } else { cagr / self.max_drawdown }
    }

    /// Dynamic Fractional Kelly Leverage: f* = ((b * p - q) / b) * Coherence
    /// Maximizes capital geometric growth rate while scaling down in low-coherence states.
    pub fn kelly_leverage(&self, coherence: f64) -> f64 {
        let p = self.win_rate();
        if self.total_trades < 5 || p <= 0.50 {
            return 1.0;
        }
        let avg_win = self.avg_win_return().max(1e-5);
        let avg_loss = self.avg_loss_return().max(1e-5);
        let b = avg_win / avg_loss;
        let q = 1.0 - p;
        let f_star = (p * b - q) / b;
        (f_star.max(0.1) * 0.5 * (0.5 + 0.5 * coherence)).clamp(0.2, 3.0)
    }
}
