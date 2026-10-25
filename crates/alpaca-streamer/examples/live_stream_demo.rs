use alpaca_streamer::{AlpacaDataFeed, AlpacaStreamer};
use std::collections::HashMap;
use std::env;
use std::fs;

fn load_env_file() -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(content) = fs::read_to_string(".env") {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    map
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("==========================================================================================");
    println!("              ALPACAMARKETS REAL-TIME QUANTUM MICROSTRUCTURE STREAMER                     ");
    println!("==========================================================================================");

    let env_file = load_env_file();

    let oauth_token = env::var("ALPACA_ACCESS_TOKEN")
        .ok()
        .or_else(|| env_file.get("ALPACA_ACCESS_TOKEN").cloned())
        .unwrap_or_default();

    let api_key = env::var("ALPACA_API_KEY")
        .ok()
        .or_else(|| env_file.get("ALPACA_API_KEY").cloned())
        .unwrap_or_default();

    let api_secret = env::var("ALPACA_API_SECRET")
        .ok()
        .or_else(|| env_file.get("ALPACA_API_SECRET").cloned())
        .unwrap_or_default();

    let mut streamer = if !oauth_token.is_empty() {
        println!("[+] Autenticando mediante Alpaca OAuth Access Token...");
        AlpacaStreamer::with_oauth_token(oauth_token, AlpacaDataFeed::Iex, 0.01)
    } else if !api_key.is_empty() && !api_secret.is_empty() {
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
