/// Institutional-grade quantitative auditor implementing E[R], Sharpe, Calmar, and Drawdown.
/// DEF-07 fix: Uses Welford's online algorithm for O(1) running mean/variance.
/// Eliminates O(n) iteration on returns_history for every Sharpe ratio call.
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

    // Welford online running statistics — no history buffer needed
    welford_count: u64,
    welford_mean: f64,
    welford_m2: f64,  // sum of squared deviations from mean
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
            welford_count: 0,
            welford_mean: 0.0,
            welford_m2: 0.0,
        }
    }

    /// Records an executed trade outcome.
    pub fn record_trade(&mut self, pnl: f64, return_fraction: f64) {
        self.total_trades += 1;
        self.current_equity += pnl;

        if self.current_equity > self.peak_equity {
            self.peak_equity = self.current_equity;
        }

        let dd = if self.peak_equity > 0.0 {
            (self.peak_equity - self.current_equity) / self.peak_equity
        } else {
            0.0
        };
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

        // Welford online update — O(1)
        self.welford_count += 1;
        let delta = return_fraction - self.welford_mean;
        self.welford_mean += delta / self.welford_count as f64;
        let delta2 = return_fraction - self.welford_mean;
        self.welford_m2 += delta * delta2;
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

    /// Annualized Sharpe Ratio — O(1) via Welford online variance.
    pub fn sharpe_ratio(&self, annualization_factor: f64) -> f64 {
        if self.welford_count < 2 {
            return 0.0;
        }
        let variance = self.welford_m2 / (self.welford_count - 1) as f64;
        let std = variance.sqrt();
        if std < 1e-9 { 0.0 } else { (self.welford_mean / std) * annualization_factor.sqrt() }
    }

    /// Calmar Ratio: CAGR / Max Drawdown
    pub fn calmar_ratio(&self, cagr: f64) -> f64 {
        if self.max_drawdown <= 1e-6 { 99.99 } else { cagr / self.max_drawdown }
    }

    /// Dynamic Fractional Kelly Leverage:
    /// f* = (p * b - q) / b
    /// where:
    ///   p = win probability
    ///   q = loss probability = 1.0 - p
    ///   b = payoff ratio (average win / average loss)
    /// Incorporates Bayesian prior (55% win rate, 1.75 payoff) when sample size < 8,
    /// applies Half-Kelly for institutional ruin-probability minimization,
    /// and scales smoothly by macroscopic quantum coherence.
    pub fn kelly_leverage(&self, coherence: f64) -> f64 {
        let (p, b) = if self.total_trades < 8 {
            // Prior Bayesian parameters: 55% win rate, 1.75 payoff ratio (TP +0.35% / SL -0.20%)
            (0.55, 1.75)
        } else {
            let win_p = self.win_rate().clamp(0.35, 0.85);
            let avg_win = self.avg_win_return().max(0.0005);
            let avg_loss = self.avg_loss_return().max(0.0005);
            let payoff = (avg_win / avg_loss).clamp(0.60, 3.50);
            (win_p, payoff)
        };

        let q = 1.0 - p;
        let f_star = (p * b - q) / b;

        // Half-Kelly institutional rule: bet half of f* to reduce variance by 75%
        let half_kelly = (f_star * 0.5).max(0.15);
        let coherence_multiplier = (0.60 + 0.40 * coherence).clamp(0.60, 1.0);
        (half_kelly * coherence_multiplier).clamp(0.20, 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sharpe_welford_online() {
        let mut auditor = PortfolioAuditor::new(100_000.0);
        // 5 identical returns: mean=0.01, std=0 -> guard returns 0
        for _ in 0..5 {
            auditor.record_trade(10.0, 0.01);
        }
        // All same return: std dev ≈ 0 -> Sharpe guard = 0
        let s = auditor.sharpe_ratio(252.0);
        assert_eq!(s, 0.0);

        // Mixed returns
        let mut auditor2 = PortfolioAuditor::new(100_000.0);
        auditor2.record_trade(100.0, 0.02);
        auditor2.record_trade(-50.0, -0.01);
        auditor2.record_trade(75.0, 0.015);
        let s2 = auditor2.sharpe_ratio(252.0);
        assert!(s2 > 0.0);
    }
}


