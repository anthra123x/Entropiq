use crate::types::{Quote, Side, Trade};

const VPIN_BUCKETS_COUNT: usize = 20;

/// Real-time microstructural state tracker.
/// Pre-allocated, zero-allocation during live tick updates.
#[derive(Debug, Clone)]
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
    pub last_event_time_ns: u64,
    pub decay_halflife_sec: f64,

    // Genuine Volume-Bucket VPIN implementation (Easley, López de Prado, O'Hara 2012)
    pub bucket_volume_target: f64,
    pub current_bucket_buy: f64,
    pub current_bucket_sell: f64,
    pub bucket_imbalances: [f64; VPIN_BUCKETS_COUNT],
    pub bucket_count: usize,
    pub bucket_head: usize,
}

impl Default for MarketTracker {
    fn default() -> Self {
        Self::new(0.01)
    }
}

impl MarketTracker {
    pub fn new(tick_size: f64) -> Self {
        let valid_tick = if tick_size > 0.0 { tick_size } else { 0.01 };
        // Calibrate bucket volume target based on asset class / tick size
        let bucket_target = if valid_tick >= 0.50 {
            // Bitcoin (BTC/USD): ~0.50 BTC per bucket (~$40,000 USD)
            0.50
        } else if valid_tick >= 0.05 {
            // Ethereum (ETH/USD): ~5.0 ETH per bucket (~$12,000 USD)
            5.0
        } else if valid_tick >= 0.01 {
            // Solana (SOL/USD) or stocks: ~50.0 units
            50.0
        } else {
            100.0
        };

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
            vpin: 0.25, // Neutral initial baseline (never saturates to 1.0)
            tick_size: valid_tick,
            last_event_time_ns: 0,
            decay_halflife_sec: 5.0,
            bucket_volume_target: bucket_target,
            current_bucket_buy: 0.0,
            current_bucket_sell: 0.0,
            bucket_imbalances: [0.0; VPIN_BUCKETS_COUNT],
            bucket_count: 0,
            bucket_head: 0,
        }
    }

    /// Applies continuous exponential time-decay to rolling volume and accumulated OFI.
    #[inline(always)]
    pub fn apply_time_decay(&mut self, current_time_ns: u64) {
        if self.last_event_time_ns > 0 && current_time_ns > self.last_event_time_ns {
            let dt_sec = (current_time_ns - self.last_event_time_ns) as f64 * 1e-9;
            if dt_sec >= 0.001 {
                let decay = (-dt_sec / self.decay_halflife_sec.max(0.1)).exp();
                self.window_buy_volume *= decay;
                self.window_sell_volume *= decay;
                self.accumulated_ofi *= decay;
            }
        }
        self.last_event_time_ns = current_time_ns;
    }

    /// Ingests a new quote and updates Order Flow Imbalance (OFI).
    pub fn update_quote(&mut self, new_quote: Quote) {
        self.apply_time_decay(new_quote.timestamp_ns);

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

        self.current_quote = Some(new_quote);
    }

    /// Ingests an executed trade print with real volume bucket partitioning.
    pub fn update_trade(&mut self, trade: Trade) {
        self.apply_time_decay(trade.timestamp_ns);

        let mut remaining_size = trade.size;
        let is_buy = trade.taker_side == Side::Buy;

        // Feed trade volume into volume buckets
        while remaining_size > 1e-9 {
            let current_filled = self.current_bucket_buy + self.current_bucket_sell;
            let space = (self.bucket_volume_target - current_filled).max(0.0);
            let fill = remaining_size.min(space);

            if is_buy {
                self.current_bucket_buy += fill;
                self.window_buy_volume += fill;
            } else {
                self.current_bucket_sell += fill;
                self.window_sell_volume += fill;
            }
            remaining_size -= fill;

            if (self.current_bucket_buy + self.current_bucket_sell) >= self.bucket_volume_target - 1e-9 {
                // Finalize bucket: compute |V_B - V_S|
                let bucket_imbalance = (self.current_bucket_buy - self.current_bucket_sell).abs();
                self.bucket_imbalances[self.bucket_head] = bucket_imbalance;
                self.bucket_head = (self.bucket_head + 1) % VPIN_BUCKETS_COUNT;
                if self.bucket_count < VPIN_BUCKETS_COUNT {
                    self.bucket_count += 1;
                }
                self.current_bucket_buy = 0.0;
                self.current_bucket_sell = 0.0;
            }
        }

        self.window_trade_count += 1;
        self.last_trade = Some(trade);
        self.recalculate_vpin();
    }

    /// Recalculates VPIN across rolling volume buckets.
    #[inline]
    fn recalculate_vpin(&mut self) {
        if self.bucket_count >= 5 {
            let mut sum_imbalance = 0.0;
            for i in 0..self.bucket_count {
                sum_imbalance += self.bucket_imbalances[i];
            }
            let denom = self.bucket_count as f64 * self.bucket_volume_target;
            if denom > 1e-9 {
                self.vpin = (sum_imbalance / denom).clamp(0.0, 1.0);
            }
        } else {
            // Bayesian prior: smooth transition from baseline 0.25
            let current_fill = self.current_bucket_buy + self.current_bucket_sell;
            if current_fill > 1e-6 {
                let partial_imbalance = (self.current_bucket_buy - self.current_bucket_sell).abs();
                let partial_vpin = partial_imbalance / current_fill;
                let weight = (current_fill / self.bucket_volume_target).min(1.0);
                self.vpin = (0.25 * (1.0 - weight) + (partial_vpin * 0.5) * weight).clamp(0.0, 1.0);
            } else {
                self.vpin = 0.25;
            }
        }
    }

    /// Resets rolling trade window counters (e.g., at each epoch/second).
    pub fn reset_window(&mut self) {
        self.window_buy_volume = 0.0;
        self.window_sell_volume = 0.0;
        self.window_trade_count = 0;
        self.accumulated_ofi = 0.0;
        self.last_ofi = 0.0;
        self.ofi_acceleration = 0.0;
        self.current_bucket_buy = 0.0;
        self.current_bucket_sell = 0.0;
        self.bucket_count = 0;
        self.bucket_head = 0;
        self.bucket_imbalances = [0.0; VPIN_BUCKETS_COUNT];
        self.vpin = 0.25;
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
