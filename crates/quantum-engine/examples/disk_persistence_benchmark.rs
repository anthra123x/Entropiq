use market_simulator::{
    BinaryDiskReader, BinaryDiskRecorder, BinaryQuoteRecord, MarketRegime, MicrostructureSimulator,
};
use quantum_engine::QuantumMarketEngine;
use std::time::Instant;

fn main() {
    println!("==========================================================================================");
    println!("             DISCO BINARIO BLINDADO: BENCHMARK DE PERSISTENCIA Y REPLAY                   ");
    println!("==========================================================================================");

    let total_ticks = 1_000_000;
    let file_path = std::env::temp_dir().join("quantum_market_feed_1M.bin");

    // -------------------------------------------------------------------------
    // ETAPA 1: ESCRITURA EN DISCO (PERSISTENCIA BINARIA BUFFERED)
    // -------------------------------------------------------------------------
    println!("Paso 1: Grabando {} ticks a disco ({:.2} MB en {:?})...",
        total_ticks,
        (total_ticks * BinaryQuoteRecord::SIZE) as f64 / (1024.0 * 1024.0),
        file_path
    );

    let mut sim = MicrostructureSimulator::new(100.0, 0.01, 12345);
    let mut recorder = BinaryDiskRecorder::create(&file_path).expect("Error creando archivo binario");

    let write_start = Instant::now();
    for i in 0..total_ticks {
        let regime = match (i / 250_000) % 4 {
            0 => MarketRegime::ThermalEquilibrium,
            1 => MarketRegime::IcebergAbsorption,
            2 => MarketRegime::QuantumBreakout,
            _ => MarketRegime::LiquidityCavitation,
        };
        let (quote, _) = sim.next_tick(regime);
        let rec = BinaryQuoteRecord::from_quote(&quote);
        recorder.write_quote(&rec).expect("Error escribiendo tick");
    }
    let total_written = recorder.finish().expect("Error finalizando archivo");
    let write_elapsed = write_start.elapsed();

    let write_mb_s = 30.52 / write_elapsed.as_secs_f64();
    let write_ticks_s = (total_ticks as f64) / write_elapsed.as_secs_f64();
    let write_latency_ns = write_elapsed.as_nanos() as f64 / total_ticks as f64;

    println!("\nI. Resultados de Escritura en Disco:");
    println!("  Total Ticks Grabados:      {}", total_written);
    println!("  Tiempo de Escritura Total: {:.4?}", write_elapsed);
    println!("  Velocidad de Escritura:    {:.2} MB/s", write_mb_s);
    println!("  Throughput de Grabación:   {:.2} Millones de Ticks / Segundo", write_ticks_s / 1_000_000.0);
    println!("  Latencia Media por Write:  {:.2} ns ({:.4} µs)", write_latency_ns, write_latency_ns / 1000.0);

    // -------------------------------------------------------------------------
    // ETAPA 2: LECTURA DIRECTA DE DISCO & PROCESAMIENTO CUÁNTICO
    // -------------------------------------------------------------------------
    println!("\nPaso 2: Abriendo archivo desde disco y ejecutando análisis cuántico en tiempo real...");

    let mut reader = BinaryDiskReader::open(&file_path).expect("Error abriendo lector de disco");
    let mut engine = QuantumMarketEngine::new(0.01);

    let read_start = Instant::now();
    let mut processed = 0u64;

    while let Some(quote) = reader.read_next_quote().expect("Error leyendo tick de disco") {
        let _ = engine.on_quote(quote);
        processed += 1;
    }
    let read_elapsed = read_start.elapsed();

    let read_ticks_s = (processed as f64) / read_elapsed.as_secs_f64();
    let read_latency_ns = read_elapsed.as_nanos() as f64 / processed as f64;

    println!("\nII. Resultados de Lectura de Disco + Cómputo Cuántico:");
    println!("  Ticks Leídos y Procesados: {}", processed);
    println!("  Tiempo Total Read + Quant: {:.4?}", read_elapsed);
    println!("  Throughput de Lectura I/O: {:.2} Millones de Ticks / Segundo", read_ticks_s / 1_000_000.0);
    println!("  Latencia Total (I/O + RK4):{:.2} ns ({:.4} µs)", read_latency_ns, read_latency_ns / 1000.0);

    println!("==========================================================================================");

    // Limpieza de archivo temporal
    let _ = std::fs::remove_file(file_path);
}
