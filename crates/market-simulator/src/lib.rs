pub mod binary_disk;
pub mod binary_feed;
pub mod regimes;
pub mod rng;

pub use binary_disk::{BinaryDiskReader, BinaryDiskRecorder};
pub use binary_feed::{BinaryBufferReplayer, BinaryQuoteRecord, BinaryTradeRecord};
pub use regimes::{MarketRegime, MicrostructureSimulator};
pub use rng::FastRng;

#[cfg(test)]
mod tests {
    use super::*;
    use market_microstructure::Quote;

    #[test]
    fn test_simulator_generates_ticks() {
        let mut sim = MicrostructureSimulator::new(100.0, 0.01, 42);
        let (q, _) = sim.next_tick(MarketRegime::ThermalEquilibrium);
        assert!(q.bid_price > 0.0);
        assert!(q.ask_price > q.bid_price);
        assert!(q.spread() > 0.0);
    }

    #[test]
    fn test_simulator_regimes_differentiation() {
        let mut sim = MicrostructureSimulator::new(100.0, 0.01, 12345);

        // Iceberg regime: Bid size must be massive
        let (q_iceberg, _) = sim.next_tick(MarketRegime::IcebergAbsorption);
        assert!(q_iceberg.bid_size > 1000.0);

        // Cavitation regime: Spread blows out
        let (q_cavit, _) = sim.next_tick(MarketRegime::LiquidityCavitation);
        assert!(q_cavit.spread() > 0.10);
    }

    #[test]
    fn test_binary_records_and_replayer() {
        let q1 = Quote::new(1_000_000, 100.5, 50.0, 100.55, 60.0);
        let q2 = Quote::new(1_001_000, 100.52, 45.0, 100.56, 70.0);

        let mut buffer = Vec::new();
        buffer.extend_from_slice(&BinaryQuoteRecord::from_quote(&q1).to_bytes());
        buffer.extend_from_slice(&BinaryQuoteRecord::from_quote(&q2).to_bytes());

        assert_eq!(buffer.len(), 64); // 2 records * 32 bytes

        let mut replayer = BinaryBufferReplayer::new(&buffer);
        assert_eq!(replayer.remaining_records(), 2);

        let read_q1 = replayer.next_quote().expect("q1 must exist");
        assert_eq!(read_q1.timestamp_ns, q1.timestamp_ns);
        assert_eq!(read_q1.bid_price, q1.bid_price);

        let read_q2 = replayer.next_quote().expect("q2 must exist");
        assert_eq!(read_q2.timestamp_ns, q2.timestamp_ns);
        assert_eq!(read_q2.ask_price, q2.ask_price);

        assert!(replayer.next_quote().is_none());
    }

    #[test]
    fn test_binary_disk_roundtrip() {
        let tmp_file = std::env::temp_dir().join("test_quantum_disk_roundtrip.bin");

        // 1. Write to disk
        let mut recorder = BinaryDiskRecorder::create(&tmp_file).expect("Recorder create");
        let q1 = Quote::new(10_000_000, 150.25, 100.0, 150.28, 120.0);
        let q2 = Quote::new(10_050_000, 150.26, 110.0, 150.29, 115.0);

        recorder.write_quote(&BinaryQuoteRecord::from_quote(&q1)).expect("Write q1");
        recorder.write_quote(&BinaryQuoteRecord::from_quote(&q2)).expect("Write q2");
        let count = recorder.finish().expect("Finish");
        assert_eq!(count, 2);

        // 2. Read back from disk
        let mut reader = BinaryDiskReader::open(&tmp_file).expect("Reader open");
        assert_eq!(reader.total_records(), 2);

        let read_q1 = reader.read_next_quote().expect("read q1").expect("some q1");
        assert_eq!(read_q1.timestamp_ns, q1.timestamp_ns);
        assert_eq!(read_q1.bid_price, q1.bid_price);

        let read_q2 = reader.read_next_quote().expect("read q2").expect("some q2");
        assert_eq!(read_q2.timestamp_ns, q2.timestamp_ns);
        assert_eq!(read_q2.ask_price, q2.ask_price);

        assert!(reader.read_next_quote().expect("EOF").is_none());

        // Cleanup
        let _ = std::fs::remove_file(tmp_file);
    }
}
