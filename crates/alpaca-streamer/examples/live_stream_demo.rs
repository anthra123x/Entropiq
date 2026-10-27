use alpaca_streamer::{AlpacaDataFeed, AlpacaStreamer, EnvConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("==========================================================================================");
    println!("              ALPACAMARKETS REAL-TIME QUANTUM MICROSTRUCTURE STREAMER                     ");
    println!("==========================================================================================");

    let cfg = EnvConfig::load();
    cfg.log_auth_mode();

    let mut streamer = if let Some(oauth_token) = cfg.oauth_token {
        println!("[+] Autenticando mediante Alpaca OAuth Access Token...");
        AlpacaStreamer::with_oauth_token(oauth_token, AlpacaDataFeed::Iex, 0.01)
    } else if let (Some(api_key), Some(api_secret)) = (cfg.api_key, cfg.api_secret) {
        println!("[+] Autenticando mediante Alpaca API Key / Secret...");
        AlpacaStreamer::with_key_secret(api_key, api_secret, AlpacaDataFeed::Iex, 0.01)
    } else {
        println!("\n[!] No se encontraron credenciales válidas en .env o variables de entorno.");
        println!("    Configura ALPACA_ACCESS_TOKEN o ALPACA_API_KEY / ALPACA_API_SECRET en .env");
        return Ok(());
    };

    let symbols = vec!["SPY", "AAPL", "NVDA"];
    println!("[+] Conectando a Alpaca y suscribiéndose a cotizaciones y transacciones de {:?}...", symbols);

    let mut tick_counter = 0u64;

    streamer
        .stream_symbol(symbols, |snap| {
            tick_counter += 1;
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
        })
        .await?;

    Ok(())
}
