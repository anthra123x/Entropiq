use crate::types::{Side, Quote};

/// Represents a single price level in the Level-2 order book.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct L2Level {
    pub price: f64,
    pub size: f64,
    pub order_count: u32,
}

impl L2Level {
    pub const EMPTY: Self = Self {
        price: 0.0,
        size: 0.0,
        order_count: 0,
    };

    #[inline(always)]
    pub fn new(price: f64, size: f64, order_count: u32) -> Self {
        Self {
            price,
            size,
            order_count,
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.size <= 1e-9 || self.price <= 0.0
    }
}

/// Zero-heap-allocation Level-2 Order Book with fixed depth capacity.
/// Maintains up to `DEPTH` levels of bids and asks on the stack.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct L2Book<const DEPTH: usize = 10> {
    pub timestamp_ns: u64,
    pub bids: [L2Level; DEPTH],
    pub asks: [L2Level; DEPTH],
    pub bid_count: usize,
    pub ask_count: usize,
}

impl<const DEPTH: usize> L2Book<DEPTH> {
    pub fn new(timestamp_ns: u64) -> Self {
        Self {
            timestamp_ns,
            bids: [L2Level::EMPTY; DEPTH],
            asks: [L2Level::EMPTY; DEPTH],
            bid_count: 0,
            ask_count: 0,
        }
    }

    /// Sets or updates a bid level (0 = Best Bid, 1 = Next Level, etc.)
    #[inline(always)]
    pub fn set_bid(&mut self, level: usize, price: f64, size: f64, order_count: u32) {
        if level < DEPTH {
            self.bids[level] = L2Level::new(price, size, order_count);
            if level >= self.bid_count {
                self.bid_count = level + 1;
            }
        }
    }

    /// Sets or updates an ask level (0 = Best Ask, 1 = Next Level, etc.)
    #[inline(always)]
    pub fn set_ask(&mut self, level: usize, price: f64, size: f64, order_count: u32) {
        if level < DEPTH {
            self.asks[level] = L2Level::new(price, size, order_count);
            if level >= self.ask_count {
                self.ask_count = level + 1;
            }
        }
    }

    /// Best Bid level
    #[inline(always)]
    pub fn best_bid(&self) -> Option<&L2Level> {
        if self.bid_count > 0 && !self.bids[0].is_empty() {
            Some(&self.bids[0])
        } else {
            None
        }
    }

    /// Best Ask level
    #[inline(always)]
    pub fn best_ask(&self) -> Option<&L2Level> {
        if self.ask_count > 0 && !self.asks[0].is_empty() {
            Some(&self.asks[0])
        } else {
            None
        }
    }

    /// Top-of-Book Bid-Ask Spread
    #[inline(always)]
    pub fn spread(&self) -> f64 {
        match (self.best_bid(), self.best_ask()) {
            (Some(b), Some(a)) => (a.price - b.price).max(0.0),
            _ => 0.0,
        }
    }

    /// Classical Mid-Price from top-of-book
    #[inline(always)]
    pub fn mid_price(&self) -> f64 {
        match (self.best_bid(), self.best_ask()) {
            (Some(b), Some(a)) => (a.price + b.price) * 0.5,
            _ => 0.0,
        }
    }

    /// Total resting bid liquidity depth across all valid levels
    #[inline(always)]
    pub fn total_bid_depth(&self) -> f64 {
        let mut total = 0.0;
        for i in 0..self.bid_count {
            total += self.bids[i].size;
        }
        total
    }

    /// Total resting ask liquidity depth across all valid levels
    #[inline(always)]
    pub fn total_ask_depth(&self) -> f64 {
        let mut total = 0.0;
        for i in 0..self.ask_count {
            total += self.asks[i].size;
        }
        total
    }

    /// Multi-level Depth Imbalance in range [-1.0, 1.0]
    #[inline(always)]
    pub fn depth_imbalance(&self) -> f64 {
        let b = self.total_bid_depth();
        let a = self.total_ask_depth();
        let denom = b + a;
        if denom <= 1e-9 {
            0.0
        } else {
            (b - a) / denom
        }
    }

    /// Depth-Weighted Midprice (L2 Microprice) using exponential distance decay
    pub fn weighted_mid_price(&self) -> f64 {
        let mid = self.mid_price();
        if mid <= 0.0 {
            return 0.0;
        }

        let mut bid_w_sum = 0.0;
        let mut bid_p_sum = 0.0;
        for i in 0..self.bid_count {
            let lvl = &self.bids[i];
            if !lvl.is_empty() {
                let dist = (mid - lvl.price).abs().max(1e-4);
                let w = lvl.size / dist;
                bid_w_sum += w;
                bid_p_sum += lvl.price * w;
            }
        }

        let mut ask_w_sum = 0.0;
        let mut ask_p_sum = 0.0;
        for i in 0..self.ask_count {
            let lvl = &self.asks[i];
            if !lvl.is_empty() {
                let dist = (lvl.price - mid).abs().max(1e-4);
                let w = lvl.size / dist;
                ask_w_sum += w;
                ask_p_sum += lvl.price * w;
            }
        }

        let total_w = bid_w_sum + ask_w_sum;
        if total_w <= 1e-9 {
            mid
        } else {
            // Harmonic microprice weighting: higher weight on opposite side reflects where true equilibrium is
            (bid_p_sum * ask_w_sum + ask_p_sum * bid_w_sum) / (2.0 * bid_w_sum * ask_w_sum).max(1e-9)
        }
    }

    /// Multi-Level Deep Order Flow Imbalance (Deep OFI)
    /// Computes the change in order book depth across all levels k=0..DEPTH-1,
    /// weighted by inverse level distance: w_k = 1.0 / (k + 1).
    pub fn deep_ofi(&self, previous: &Self) -> f64 {
        let mut ofi = 0.0;
        let max_k = self.bid_count.max(self.ask_count).min(DEPTH);

        for k in 0..max_k {
            let weight = 1.0 / ((k + 1) as f64);

            // Bid side delta
            let bid_delta = if k < self.bid_count && k < previous.bid_count {
                let curr = &self.bids[k];
                let prev = &previous.bids[k];
                if curr.price > prev.price {
                    curr.size
                } else if (curr.price - prev.price).abs() < 1e-6 {
                    curr.size - prev.size
                } else {
                    -prev.size
                }
            } else if k < self.bid_count {
                self.bids[k].size
            } else if k < previous.bid_count {
                -previous.bids[k].size
            } else {
                0.0
            };

            // Ask side delta
            let ask_delta = if k < self.ask_count && k < previous.ask_count {
                let curr = &self.asks[k];
                let prev = &previous.asks[k];
                if curr.price < prev.price {
                    curr.size
                } else if (curr.price - prev.price).abs() < 1e-6 {
                    curr.size - prev.size
                } else {
                    -prev.size
                }
            } else if k < self.ask_count {
                self.asks[k].size
            } else if k < previous.ask_count {
                -previous.asks[k].size
            } else {
                0.0
            };

            ofi += weight * (bid_delta - ask_delta);
        }

        ofi
    }

    /// Converts the Top-of-Book (Level 0) into a standard Level-1 Quote
    pub fn to_quote(&self) -> Option<Quote> {
        match (self.best_bid(), self.best_ask()) {
            (Some(b), Some(a)) => Some(Quote::new(
                self.timestamp_ns,
                b.price,
                b.size,
                a.price,
                a.size,
            )),
            _ => None,
        }
    }

    /// Computes the effective potential barrier height for quantum tunneling
    /// integrating resting volume depth weighted by price distance from mid.
    pub fn cumulative_barrier_height(&self, side: Side, max_levels: usize) -> f64 {
        let mid = self.mid_price();
        let mut barrier = 0.0;
        let count = max_levels.min(DEPTH);

        match side {
            Side::Buy => {
                // Crossing into asks
                for i in 0..count.min(self.ask_count) {
                    let lvl = &self.asks[i];
                    if !lvl.is_empty() {
                        let dist = (lvl.price - mid).max(0.01);
                        barrier += lvl.size * dist;
                    }
                }
            }
            Side::Sell => {
                // Crossing into bids
                for i in 0..count.min(self.bid_count) {
                    let lvl = &self.bids[i];
                    if !lvl.is_empty() {
                        let dist = (mid - lvl.price).max(0.01);
                        barrier += lvl.size * dist;
                    }
                }
            }
        }

        barrier.max(0.1)
    }
}
