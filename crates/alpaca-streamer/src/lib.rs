pub mod client;
pub mod model;

pub use client::{AlpacaDataFeed, AlpacaStreamer};
pub use model::{AlpacaQuote, AlpacaStreamMessage, AlpacaTrade};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_alpaca_quote() {
        let json_data = r#"[
            {
                "T": "q",
                "S": "AAPL",
                "bx": "V",
                "bp": 182.50,
                "bs": 100,
                "ax": "V",
                "ap": 182.55,
                "as": 200,
                "c": ["R"],
                "t": "2026-10-24T20:00:00.000000000Z"
            }
        ]"#;

        let messages: Vec<AlpacaStreamMessage> = serde_json::from_str(json_data).expect("Valid JSON");
        assert_eq!(messages.len(), 1);

        if let AlpacaStreamMessage::Quote(q) = &messages[0] {
            assert_eq!(q.symbol, "AAPL");
            assert_eq!(q.bid_price, 182.50);
            assert_eq!(q.ask_price, 182.55);

            let micro_q = q.to_microstructure_quote(1_000_000_000);
            assert_eq!(micro_q.bid_price, 182.50);
            assert_eq!(micro_q.ask_price, 182.55);
            assert_eq!(micro_q.spread(), 0.05000000000001137);
        } else {
            panic!("Expected Quote message");
        }
    }

    #[test]
    fn test_deserialize_alpaca_trade() {
        let json_data = r#"[
            {
                "T": "t",
                "S": "SPY",
                "i": 9999,
                "x": "V",
                "p": 500.25,
                "s": 50,
                "c": ["@"],
                "z": "C",
                "t": "2026-10-24T20:00:00.000000000Z"
            }
        ]"#;

        let messages: Vec<AlpacaStreamMessage> = serde_json::from_str(json_data).expect("Valid JSON");
        assert_eq!(messages.len(), 1);

        if let AlpacaStreamMessage::Trade(t) = &messages[0] {
            assert_eq!(t.symbol, "SPY");
            assert_eq!(t.price, 500.25);
            assert_eq!(t.size, 50.0);
        } else {
            panic!("Expected Trade message");
        }
    }
}
