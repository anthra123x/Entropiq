/// Trading side / market taker intent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

/// Level-1 Top-of-Book Quote (NBBO format directly compatible with Alpaca)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quote {
    pub timestamp_ns: u64,
    pub bid_price: f64,
    pub bid_size: f64,
    pub ask_price: f64,
    pub ask_size: f64,
}

impl Quote {
    #[inline(always)]
    pub fn new(
        timestamp_ns: u64,
        bid_price: f64,
        bid_size: f64,
        ask_price: f64,
        ask_size: f64,
    ) -> Self {
        Self {
            timestamp_ns,
            bid_price,
            bid_size,
            ask_price,
            ask_size,
        }
    }

    /// Bid-Ask Spread
    #[inline(always)]
    pub fn spread(&self) -> f64 {
        (self.ask_price - self.bid_price).max(0.0)
    }

    /// Classical Mid-Price (unweighted average)
    #[inline(always)]
    pub fn mid_price(&self) -> f64 {
        (self.ask_price + self.bid_price) * 0.5
    }

    /// Volume-Weighted Micro-Price:
    /// P_micro = (bid_price * ask_size + ask_price * bid_size) / (bid_size + ask_size)
    #[inline(always)]
    pub fn micro_price(&self) -> f64 {
        let total_size = self.bid_size + self.ask_size;
        if total_size <= 1e-9 {
            self.mid_price()
        } else {
            (self.bid_price * self.ask_size + self.ask_price * self.bid_size) / total_size
        }
    }

    /// Order Book Imbalance in range [-1.0, 1.0]:
    /// +1.0 = 100% Bid (extreme buy pressure), -1.0 = 100% Ask (extreme sell pressure)
    #[inline(always)]
    pub fn imbalance(&self) -> f64 {
        let total_size = self.bid_size + self.ask_size;
        if total_size <= 1e-9 {
            0.0
        } else {
            (self.bid_size - self.ask_size) / total_size
        }
    }
}

/// Executed trade print (Tick level)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trade {
    pub timestamp_ns: u64,
    pub price: f64,
    pub size: f64,
    pub taker_side: Side,
}

impl Trade {
    #[inline(always)]
    pub fn new(timestamp_ns: u64, price: f64, size: f64, taker_side: Side) -> Self {
        Self {
            timestamp_ns,
            price,
            size,
            taker_side,
        }
    }
}
