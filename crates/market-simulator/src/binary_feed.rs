use market_microstructure::{Quote, Side, Trade};

/// Compact 32-byte binary quote record (cache-line optimized: exactly half of a 64-byte L1 cache line).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct BinaryQuoteRecord {
    pub timestamp_ns: u64, // 8 bytes
    pub bid_price: f64,    // 8 bytes
    pub ask_price: f64,    // 8 bytes
    pub bid_size: f32,     // 4 bytes
    pub ask_size: f32,     // 4 bytes
}

impl BinaryQuoteRecord {
    pub const SIZE: usize = 32;

    pub fn from_quote(q: &Quote) -> Self {
        Self {
            timestamp_ns: q.timestamp_ns,
            bid_price: q.bid_price,
            ask_price: q.ask_price,
            bid_size: q.bid_size as f32,
            ask_size: q.ask_size as f32,
        }
    }

    pub fn to_quote(&self) -> Quote {
        Quote::new(
            self.timestamp_ns,
            self.bid_price,
            self.bid_size as f64,
            self.ask_price,
            self.ask_size as f64,
        )
    }

    #[inline(always)]
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0..8].copy_from_slice(&self.timestamp_ns.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.bid_price.to_le_bytes());
        bytes[16..24].copy_from_slice(&self.ask_price.to_le_bytes());
        bytes[24..28].copy_from_slice(&self.bid_size.to_le_bytes());
        bytes[28..32].copy_from_slice(&self.ask_size.to_le_bytes());
        bytes
    }

    #[inline(always)]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < Self::SIZE {
            return Err("Buffer too small for BinaryQuoteRecord");
        }
        let timestamp_ns = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let bid_price = f64::from_le_bytes(bytes[8..16].try_into().unwrap());
        let ask_price = f64::from_le_bytes(bytes[16..24].try_into().unwrap());
        let bid_size = f32::from_le_bytes(bytes[24..28].try_into().unwrap());
        let ask_size = f32::from_le_bytes(bytes[28..32].try_into().unwrap());

        Ok(Self {
            timestamp_ns,
            bid_price,
            ask_price,
            bid_size,
            ask_size,
        })
    }
}

/// Compact 32-byte binary trade record.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct BinaryTradeRecord {
    pub timestamp_ns: u64, // 8 bytes
    pub price: f64,        // 8 bytes
    pub size: f32,         // 4 bytes
    pub side: u8,          // 1 byte: 1 = Buy, 2 = Sell
    pub _pad: [u8; 11],    // 11 bytes padding to reach 32 bytes
}

impl BinaryTradeRecord {
    pub const SIZE: usize = 32;

    pub fn from_trade(t: &Trade) -> Self {
        Self {
            timestamp_ns: t.timestamp_ns,
            price: t.price,
            size: t.size as f32,
            side: match t.taker_side {
                Side::Buy => 1,
                Side::Sell => 2,
            },
            _pad: [0u8; 11],
        }
    }

    pub fn to_trade(&self) -> Trade {
        Trade::new(
            self.timestamp_ns,
            self.price,
            self.size as f64,
            if self.side == 1 { Side::Buy } else { Side::Sell },
        )
    }

    #[inline(always)]
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0..8].copy_from_slice(&self.timestamp_ns.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.price.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.size.to_le_bytes());
        bytes[20] = self.side;
        bytes
    }

    #[inline(always)]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < Self::SIZE {
            return Err("Buffer too small for BinaryTradeRecord");
        }
        let timestamp_ns = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let price = f64::from_le_bytes(bytes[8..16].try_into().unwrap());
        let size = f32::from_le_bytes(bytes[16..20].try_into().unwrap());
        let side = bytes[20];

        Ok(Self {
            timestamp_ns,
            price,
            size,
            side,
            _pad: [0u8; 11],
        })
    }
}

/// Zero-allocation binary buffer replayer.
pub struct BinaryBufferReplayer<'a> {
    buffer: &'a [u8],
    offset: usize,
}

impl<'a> BinaryBufferReplayer<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, offset: 0 }
    }

    /// Reads next quote record in O(1) time without allocations.
    #[inline(always)]
    pub fn next_quote(&mut self) -> Option<Quote> {
        if self.offset + BinaryQuoteRecord::SIZE > self.buffer.len() {
            return None;
        }
        let chunk = &self.buffer[self.offset..self.offset + BinaryQuoteRecord::SIZE];
        self.offset += BinaryQuoteRecord::SIZE;
        BinaryQuoteRecord::from_bytes(chunk).ok().map(|r| r.to_quote())
    }

    pub fn reset(&mut self) {
        self.offset = 0;
    }

    pub fn remaining_records(&self) -> usize {
        (self.buffer.len().saturating_sub(self.offset)) / BinaryQuoteRecord::SIZE
    }
}
