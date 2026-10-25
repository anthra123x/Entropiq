use crate::rng::FastRng;
use market_microstructure::{Quote, Side, Trade};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketRegime {
    /// Regime 1: Brownian random noise, balanced bid/ask depth, no directional trend
    ThermalEquilibrium,
    /// Regime 2: Stealth institutional absorption (massive hidden/passive size absorbing flow without price movement)
    IcebergAbsorption,
    /// Regime 3: Aggressive momentum sweep penetrating liquidity walls
    QuantumBreakout,
    /// Regime 4: Flash vacuum / sudden spread blowout and cavitation
    LiquidityCavitation,
}

pub struct MicrostructureSimulator {
    rng: FastRng,
    current_mid: f64,
    current_spread: f64,
    timestamp_ns: u64,
    tick_size: f64,
}

impl MicrostructureSimulator {
    pub fn new(initial_price: f64, tick_size: f64, seed: u64) -> Self {
        Self {
            rng: FastRng::new(seed),
            current_mid: initial_price,
            current_spread: tick_size * 2.0,
            timestamp_ns: 1_700_000_000_000_000_000, // nanosecond epoch
            tick_size,
        }
    }

    /// Generates next tick event (Quote and optional Trade) based on active market regime.
    pub fn next_tick(&mut self, regime: MarketRegime) -> (Quote, Option<Trade>) {
        // Increment timestamp by random microsecond interval [50µs .. 500µs]
        let dt_ns = 50_000 + (self.rng.next_u64() % 450_000);
        self.timestamp_ns += dt_ns;

        let (quote, trade) = match regime {
            MarketRegime::ThermalEquilibrium => {
                // Symmetrical Brownian wiggle
                let drift = self.rng.next_gaussian() * self.tick_size * 0.5;
                self.current_mid = (self.current_mid + drift).max(1.0);
                self.current_spread = self.tick_size * 2.0;

                let half_spread = self.current_spread * 0.5;
                let bid = (self.current_mid - half_spread).max(0.01);
                let ask = bid + self.current_spread;

                let bid_size = 50.0 + self.rng.next_f64() * 20.0;
                let ask_size = 50.0 + self.rng.next_f64() * 20.0;
                let q = Quote::new(self.timestamp_ns, bid, bid_size, ask, ask_size);

                // 25% chance of small balanced trade
                let tr = if self.rng.next_f64() < 0.25 {
                    let side = if self.rng.next_f64() < 0.5 { Side::Buy } else { Side::Sell };
                    let price = if side == Side::Buy { ask } else { bid };
                    Some(Trade::new(self.timestamp_ns, price, 5.0 + self.rng.next_f64() * 5.0, side))
                } else {
                    None
                };

                (q, tr)
            }

            MarketRegime::IcebergAbsorption => {
                // Price is pinned, but bid size is massive and absorbing all sales
                self.current_spread = self.tick_size;
                let bid = self.current_mid;
                let ask = bid + self.current_spread;

                // Hidden iceberg: bid depth artificially massive
                let bid_size = 2000.0 + self.rng.next_f64() * 500.0;
                let ask_size = 30.0 + self.rng.next_f64() * 10.0;
                let q = Quote::new(self.timestamp_ns, bid, bid_size, ask, ask_size);

                // Frequent aggressive sell orders being absorbed at the Bid price
                let tr = Some(Trade::new(
                    self.timestamp_ns,
                    bid,
                    50.0 + self.rng.next_f64() * 50.0,
                    Side::Sell,
                ));

                (q, tr)
            }

            MarketRegime::QuantumBreakout => {
                // High kinetic push: price walks upward rapidly
                let step = self.tick_size * (1.0 + self.rng.next_f64());
                self.current_mid += step;
                self.current_spread = self.tick_size;

                let bid = self.current_mid;
                let ask = bid + self.current_spread;

                // Ask depth is thin, Bid depth stacks behind
                let bid_size = 500.0 + self.rng.next_f64() * 200.0;
                let ask_size = 15.0 + self.rng.next_f64() * 10.0;
                let q = Quote::new(self.timestamp_ns, bid, bid_size, ask, ask_size);

                // Heavy aggressive buy orders sweeping ask
                let tr = Some(Trade::new(
                    self.timestamp_ns,
                    ask,
                    100.0 + self.rng.next_f64() * 100.0,
                    Side::Buy,
                ));

                (q, tr)
            }

            MarketRegime::LiquidityCavitation => {
                // Flash vacuum: spread blows out 10x, liquidity evaporates, price drops abruptly
                self.current_spread = self.tick_size * 15.0;
                self.current_mid -= self.tick_size * 5.0;

                let bid = self.current_mid - self.current_spread * 0.5;
                let ask = bid + self.current_spread;

                let bid_size = 2.0; // Virtually zero resting liquidity
                let ask_size = 5.0;
                let q = Quote::new(self.timestamp_ns, bid, bid_size, ask, ask_size);

                let tr = Some(Trade::new(
                    self.timestamp_ns,
                    bid,
                    50.0,
                    Side::Sell,
                ));

                (q, tr)
            }
        };

        (quote, trade)
    }
}
