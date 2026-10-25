use crate::types::{Quote, Side, Trade};

/// Real-time microstructural state tracker.
/// Pre-allocated, zero-allocation during live tick updates.
#[derive(Debug, Clone, Default)]
pub struct MarketTracker {
    pub current_quote: Option<Quote>,
    pub previous_quote: Option<Quote>,
    pub last_trade: Option<Trade>,

    // Rolling metrics over current micro-window
    pub window_buy_volume: f64,
    pub window_sell_volume: f64,
    pub window_trade_count: u64,
    pub accumulated_ofi: f64,
    pub last_ofi: f64,
    pub ofi_acceleration: f64,
    pub vpin: f64,
    pub tick_size: f64,
}

impl MarketTracker {
    pub fn new(tick_size: f64) -> Self {
        Self {
            current_quote: None,
            previous_quote: None,
            last_trade: None,
            window_buy_volume: 0.0,
            window_sell_volume: 0.0,
            window_trade_count: 0,
            accumulated_ofi: 0.0,
            last_ofi: 0.0,
            ofi_acceleration: 0.0,
            vpin: 0.0,
            tick_size: if tick_size > 0.0 { tick_size } else { 0.01 },
        }
    }

    /// Ingests a new quote and updates Order Flow Imbalance (OFI).
    pub fn update_quote(&mut self, new_quote: Quote) {
        if let Some(prev) = self.current_quote {
            // Cont-Kukanov-Stoikov OFI formulation:
            // e_n = I_{p_b(n) >= p_b(n-1)} * q_b(n) - I_{p_b(n) <= p_b(n-1)} * q_b(n-1)
            //     - I_{p_a(n) <= p_a(n-1)} * q_a(n) + I_{p_a(n) >= p_a(n-1)} * q_a(n-1)
            let mut ofi = 0.0;

            if new_quote.bid_price > prev.bid_price {
                ofi += new_quote.bid_size;
            } else if (new_quote.bid_price - prev.bid_price).abs() < 1e-9 {
                ofi += new_quote.bid_size - prev.bid_size;
            } else {
                ofi -= prev.bid_size;
            }

            if new_quote.ask_price < prev.ask_price {
                ofi -= new_quote.ask_size;
            } else if (new_quote.ask_price - prev.ask_price).abs() < 1e-9 {
                ofi -= new_quote.ask_size - prev.ask_size;
            } else {
                ofi += prev.ask_size;
            }

            self.ofi_acceleration = ofi - self.last_ofi;
            self.last_ofi = ofi;
            self.accumulated_ofi += ofi;
            self.previous_quote = Some(prev);
        }

        let total_depth = new_quote.bid_size + new_quote.ask_size;
        if total_depth > 1e-6 && self.window_trade_count == 0 {
            self.vpin = (new_quote.bid_size - new_quote.ask_size).abs() / total_depth;
        }

        self.current_quote = Some(new_quote);
    }

    /// Ingests an executed trade print.
    pub fn update_trade(&mut self, trade: Trade) {
        match trade.taker_side {
            Side::Buy => self.window_buy_volume += trade.size,
            Side::Sell => self.window_sell_volume += trade.size,
        }
        self.window_trade_count += 1;
        let total_vol = self.window_buy_volume + self.window_sell_volume;
        if total_vol > 1e-6 {
            self.vpin = (self.window_buy_volume - self.window_sell_volume).abs() / total_vol;
        }
        self.last_trade = Some(trade);
    }

    /// Resets rolling trade window counters (e.g., at each epoch/second).
    pub fn reset_window(&mut self) {
        self.window_buy_volume = 0.0;
        self.window_sell_volume = 0.0;
        self.window_trade_count = 0;
        self.accumulated_ofi = 0.0;
        self.last_ofi = 0.0;
        self.ofi_acceleration = 0.0;
        self.vpin = 0.0;
    }

    /// Kinetic flow energy E_k = 0.5 * Mass * Velocity^2
    /// Velocity = Price drift / dt, Mass = traded volume.
    pub fn kinetic_energy(&self) -> f64 {
        let quote = match self.current_quote {
            Some(q) => q,
            None => return 0.0,
        };

        let prev_quote = match self.previous_quote {
            Some(pq) => pq,
            None => return 0.0,
        };

        let dt_sec = ((quote.timestamp_ns.saturating_sub(prev_quote.timestamp_ns)) as f64) * 1e-9;
        let dt = dt_sec.max(1e-4);

        let delta_price = quote.micro_price() - prev_quote.micro_price();
        let velocity = delta_price / dt;

        let total_volume = (self.window_buy_volume + self.window_sell_volume).max(1.0);
        0.5 * total_volume * velocity * velocity
    }

    /// Barrier potential for a given target direction.
    /// Ask wall resists upward movement; Bid wall resists downward movement.
    pub fn barrier_potential(&self, target_side: Side) -> (f64, f64) {
        let quote = match self.current_quote {
            Some(q) => q,
            None => return (0.0, 0.0),
        };

        let spread_ticks = (quote.spread() / self.tick_size).max(1.0);

        match target_side {
            Side::Buy => {
                // Trying to cross the Ask
                let barrier_height = quote.ask_size * (1.0 + quote.spread());
                (barrier_height, spread_ticks)
            }
            Side::Sell => {
                // Trying to cross the Bid
                let barrier_height = quote.bid_size * (1.0 + quote.spread());
                (barrier_height, spread_ticks)
            }
        }
    }
}
