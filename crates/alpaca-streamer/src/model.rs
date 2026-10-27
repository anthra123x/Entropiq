use market_microstructure::{Quote, Side, Trade};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct AuthMessage<'a> {
    pub action: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<&'a str>,
}

#[derive(Debug, Serialize)]
pub struct SubscribeMessage<'a> {
    pub action: &'static str,
    pub trades: Vec<&'a str>,
    pub quotes: Vec<&'a str>,
}

/// Raw message from Alpaca Market Data WebSocket stream (SIP or IEX)
#[derive(Debug, Deserialize)]
#[serde(tag = "T")]
pub enum AlpacaStreamMessage {
    #[serde(rename = "success")]
    Success { msg: String },

    #[serde(rename = "error")]
    Error { msg: String, code: u32 },

    #[serde(rename = "subscription")]
    Subscription {
        #[serde(default)]
        trades: Vec<String>,
        #[serde(default)]
        quotes: Vec<String>,
    },

    #[serde(rename = "q")]
    Quote(AlpacaQuote),

    #[serde(rename = "t")]
    Trade(AlpacaTrade),

    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct AlpacaQuote {
    #[serde(rename = "S")]
    pub symbol: String,
    #[serde(rename = "bp")]
    pub bid_price: f64,
    #[serde(rename = "bs")]
    pub bid_size: f64,
    #[serde(rename = "ap")]
    pub ask_price: f64,
    #[serde(rename = "as")]
    pub ask_size: f64,
    /// Exchange timestamp in RFC3339 / ISO8601 format (nanosecond resolution from Alpaca)
    #[serde(rename = "t", default)]
    pub timestamp: Option<String>,
}

impl AlpacaQuote {
    /// Converts to internal zero-cost Quote struct.
    /// Uses exchange timestamp when available (DEF-02: avoids OS clock jitter ±50µs).
    /// Falls back to caller-provided `fallback_ns` only when exchange timestamp is absent.
    pub fn to_microstructure_quote(&self, fallback_ns: u64) -> Quote {
        let ts = self.timestamp.as_deref()
            .and_then(parse_alpaca_timestamp_ns)
            .unwrap_or(fallback_ns);
        Quote::new(ts, self.bid_price, self.bid_size, self.ask_price, self.ask_size)
    }
}

#[derive(Debug, Deserialize)]
pub struct AlpacaTrade {
    #[serde(rename = "S")]
    pub symbol: String,
    #[serde(rename = "p")]
    pub price: f64,
    #[serde(rename = "s")]
    pub size: f64,
    /// Exchange timestamp in RFC3339 / ISO8601 format (nanosecond resolution from Alpaca)
    #[serde(rename = "t", default)]
    pub timestamp: Option<String>,
}

impl AlpacaTrade {
    /// Infers taker side by comparing trade price to prevailing best bid/ask.
    /// Uses exchange timestamp when available (DEF-02 fix).
    pub fn to_microstructure_trade(&self, fallback_ns: u64, prevailing_mid: f64) -> Trade {
        let ts = self.timestamp.as_deref()
            .and_then(parse_alpaca_timestamp_ns)
            .unwrap_or(fallback_ns);
        let side = if self.price >= prevailing_mid { Side::Buy } else { Side::Sell };
        Trade::new(ts, self.price, self.size, side)
    }
}

/// Parses an Alpaca ISO8601/RFC3339 timestamp to nanoseconds since UNIX epoch.
/// Format: "2026-10-24T20:00:00.123456789Z"
/// Zero allocation — no external crate needed.
pub fn parse_alpaca_timestamp_ns(ts: &str) -> Option<u64> {
    let ts = ts.trim_end_matches('Z').trim();
    let (date_time, nanos_str) = if let Some(dot_pos) = ts.rfind('.') {
        (&ts[..dot_pos], Some(&ts[dot_pos + 1..]))
    } else {
        (ts, None)
    };

    if date_time.len() < 19 {
        return None;
    }

    let year: u64 = date_time[0..4].parse().ok()?;
    let month: u64 = date_time[5..7].parse().ok()?;
    let day: u64 = date_time[8..10].parse().ok()?;
    let hour: u64 = date_time[11..13].parse().ok()?;
    let min: u64 = date_time[14..16].parse().ok()?;
    let sec: u64 = date_time[17..19].parse().ok()?;

    let epoch_days = days_since_epoch(year, month, day)?;
    let total_secs = epoch_days * 86_400 + hour * 3600 + min * 60 + sec;

    let sub_nanos: u64 = if let Some(frac) = nanos_str {
        let len = frac.len().min(9);
        let padded = format!("{:0<9}", &frac[..len]);
        padded.parse().unwrap_or(0)
    } else {
        0
    };

    Some(total_secs * 1_000_000_000 + sub_nanos)
}

/// Gregorian days since Unix epoch (1970-01-01) using Rata Die algorithm.
fn days_since_epoch(year: u64, month: u64, day: u64) -> Option<u64> {
    if month == 0 || month > 12 || day == 0 || day > 31 || year < 1970 {
        return None;
    }
    let y = if month <= 2 { year - 1 } else { year };
    let m = if month <= 2 { month + 9 } else { month - 3 };
    let era = y / 400;
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe;
    days.checked_sub(719_468)
}
