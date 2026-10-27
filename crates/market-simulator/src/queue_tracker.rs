use market_microstructure::{Side, Trade};

/// Order execution lifecycle status in the FIFO queue
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueOrderStatus {
    Resting,
    PartiallyFilled,
    Filled,
    Cancelled,
}

/// High-Fidelity FIFO Queue Position Tracker for realistic backtesting without fill lookahead bias.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QueueOrderTracker {
    pub limit_price: f64,
    pub side: Side,
    pub original_size: f64,
    pub remaining_size: f64,
    pub qty_ahead: f64,
    pub filled_size: f64,
    pub avg_fill_price: f64,
    pub status: QueueOrderStatus,
}

impl QueueOrderTracker {
    /// Creates a new limit order placed at `limit_price` with `initial_queue_ahead` size in front of us.
    pub fn new(limit_price: f64, side: Side, size: f64, initial_queue_ahead: f64) -> Self {
        Self {
            limit_price,
            side,
            original_size: size,
            remaining_size: size,
            qty_ahead: initial_queue_ahead.max(0.0),
            filled_size: 0.0,
            avg_fill_price: 0.0,
            status: QueueOrderStatus::Resting,
        }
    }

    /// Returns true if the order is completely filled
    #[inline(always)]
    pub fn is_filled(&self) -> bool {
        self.status == QueueOrderStatus::Filled
    }

    /// Models order cancellations ahead of us in the queue.
    /// In real markets, a large percentage of resting depth cancels rather than executes.
    #[inline(always)]
    pub fn on_cancellation_ahead(&mut self, cancel_volume: f64) {
        if self.qty_ahead > 0.0 {
            self.qty_ahead = (self.qty_ahead - cancel_volume.max(0.0)).max(0.0);
        }
    }

    /// Evaluates an incoming executed trade against our resting queue position.
    pub fn on_trade(&mut self, trade: &Trade) -> f64 {
        if self.status == QueueOrderStatus::Filled || self.status == QueueOrderStatus::Cancelled {
            return 0.0;
        }

        let mut fill_this_event = 0.0;

        match self.side {
            Side::Buy => {
                // Buy limit order fills when taker Sells
                if trade.taker_side == Side::Sell {
                    if (trade.price - self.limit_price).abs() < 1e-6 {
                        // Trade at our exact limit price: first consume queue ahead
                        if self.qty_ahead > 0.0 {
                            if trade.size <= self.qty_ahead {
                                self.qty_ahead -= trade.size;
                            } else {
                                let excess = trade.size - self.qty_ahead;
                                self.qty_ahead = 0.0;
                                fill_this_event = excess.min(self.remaining_size);
                            }
                        } else {
                            fill_this_event = trade.size.min(self.remaining_size);
                        }
                    } else if trade.price < self.limit_price {
                        // Traded THROUGH our bid: all queue ahead is exhausted, we fill immediately
                        self.qty_ahead = 0.0;
                        fill_this_event = trade.size.min(self.remaining_size);
                    }
                }
            }
            Side::Sell => {
                // Sell limit order fills when taker Buys
                if trade.taker_side == Side::Buy {
                    if (trade.price - self.limit_price).abs() < 1e-6 {
                        if self.qty_ahead > 0.0 {
                            if trade.size <= self.qty_ahead {
                                self.qty_ahead -= trade.size;
                            } else {
                                let excess = trade.size - self.qty_ahead;
                                self.qty_ahead = 0.0;
                                fill_this_event = excess.min(self.remaining_size);
                            }
                        } else {
                            fill_this_event = trade.size.min(self.remaining_size);
                        }
                    } else if trade.price > self.limit_price {
                        // Traded THROUGH our ask: fill immediately
                        self.qty_ahead = 0.0;
                        fill_this_event = trade.size.min(self.remaining_size);
                    }
                }
            }
        }

        if fill_this_event > 0.0 {
            let prev_total_val = self.filled_size * self.avg_fill_price;
            let new_fill_val = fill_this_event * self.limit_price;
            self.filled_size += fill_this_event;
            self.remaining_size -= fill_this_event;
            self.avg_fill_price = (prev_total_val + new_fill_val) / self.filled_size;

            if self.remaining_size <= 1e-9 {
                self.remaining_size = 0.0;
                self.status = QueueOrderStatus::Filled;
            } else {
                self.status = QueueOrderStatus::PartiallyFilled;
            }
        }

        fill_this_event
    }

    /// Manually cancel remainder of resting order
    pub fn cancel(&mut self) {
        if self.status != QueueOrderStatus::Filled {
            self.status = QueueOrderStatus::Cancelled;
        }
    }
}
