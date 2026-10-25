use alpaca_streamer::{AlpacaDataFeed, AlpacaStreamer};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("==========================================================================================");
    println!("              ALPACAMARKETS REAL-TIME QUANTUM MICROSTRUCTURE STREAMER                     ");
    println!("==========================================================================================");

    let api_key = env::var("ALPACA_API_KEY").unwrap_or_default();
    let api_secret = env::var("ALPACA_API_SECRET").unwrap_or_default();

    if api_key.is_empty() || api_secret.is_empty() {
        println!("\n[!] No se encontraron credenciales en el entorno (ALPACA_API_KEY / ALPACA_API_SECRET).");
        println!("    Para conectar con datos de mercado en vivo de Alpaca, ejecuta:");
        println!("    $ export ALPACA_API_KEY=\"tu_api_key\"");
        println!("    $ export ALPACA_API_SECRET=\"tu_api_secret\"");
        println!("    $ cargo run --release --example live_stream_demo -p alpaca-streamer\n");
        println!("    El conector asíncrono WebSocket TLS y el parser de mensajes están listos y validados.");
        return Ok(());
    }

    println!("[+] Credenciales detectadas. Inicializando cliente WebSocket hacia Alpaca IEX Feed...");

    let symbols = vec!["SPY", "AAPL", "NVDA"];
    let mut streamer = AlpacaStreamer::new(api_key, api_secret, AlpacaDataFeed::Iex, 0.01);

    println!("[+] Conectando a Alpaca y suscribiéndose a cotizaciones y transacciones de {:?}...", symbols);

    let mut tick_counter = 0u64;

    streamer
        .stream_symbol(symbols, |snap| {
            tick_counter += 1;
            if tick_counter % 10 == 0 || snap.active_phase != quantum_engine::MarketQuantumPhase::ThermalNoise {
                println!(
                    "Tick #{:<6} | Mid: ${:<7.2} | Micro: ${:<7.2} | S(norm): {:.3} | Coher: {:.3} | Fase: {:?} (Conf: {:.1}%) | Mach: {:.2}",
                    tick_counter,
                    snap.mid_price,
                    snap.micro_price,
                    snap.normalized_entropy,
                    snap.quantum_coherence,
                    snap.active_phase,
                    snap.phase_confidence * 100.0,
                    snap.fluid_metrics.mach_number,
                );
            }
        })
        .await?;

    Ok(())
}
