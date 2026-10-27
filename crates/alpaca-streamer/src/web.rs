use serde::Serialize;
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Live state snapshot exposed to the web dashboard and health check.
#[derive(Debug, Clone, Serialize)]
pub struct WebDashboardState {
    pub uptime_secs: u64,
    pub websocket_connected: bool,
    pub alpaca_ticks_received: u64,
    pub alpaca_ticks_dropped: u64,
    pub total_processed_ticks: u64,
    pub throughput_ticks_per_sec: f64,
    pub rss_mb: f64,
    pub avg_latency_ns: f64,
    pub min_latency_ns: u64,
    pub p50_latency_ns: u64,
    pub p99_latency_ns: u64,
    pub entropy: f64,
    pub coherence: f64,
    pub mach_number: f64,
    pub market_phase: String,
    pub phase_confidence: f64,
    pub vpin: f64,
    pub toxic_flow: bool,
    pub lead_lag_spy_bias: f64,
    pub broker_account: String,
    pub buying_power: f64,
    pub current_equity: f64,
    pub sharpe_ratio: f64,
    pub win_rate: f64,
    pub kelly_leverage: f64,
    pub profit_factor: f64,
    pub max_drawdown: f64,
    pub total_trades: u64,
}

impl Default for WebDashboardState {
    fn default() -> Self {
        Self {
            uptime_secs: 0,
            websocket_connected: false,
            alpaca_ticks_received: 0,
            alpaca_ticks_dropped: 0,
            total_processed_ticks: 0,
            throughput_ticks_per_sec: 0.0,
            rss_mb: 0.0,
            avg_latency_ns: 0.0,
            min_latency_ns: 0,
            p50_latency_ns: 0,
            p99_latency_ns: 0,
            entropy: 1.0,
            coherence: 0.0,
            mach_number: 0.0,
            market_phase: "Inicializando".to_string(),
            phase_confidence: 0.0,
            vpin: 0.0,
            toxic_flow: false,
            lead_lag_spy_bias: 0.0,
            broker_account: "Sincronizando Alpaca...".to_string(),
            buying_power: 100_000.0,
            current_equity: 100_000.0,
            sharpe_ratio: 0.0,
            win_rate: 0.0,
            kelly_leverage: 1.0,
            profit_factor: 99.99,
            max_drawdown: 0.0,
            total_trades: 0,
        }
    }
}

/// Spawns the zero-dependency async HTTP server for Render and mobile monitoring.
pub async fn start_web_server(state: Arc<RwLock<WebDashboardState>>, port: u16) {
    let addr = format!("0.0.0.0:{}", port);
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => {
            println!("[+] Micro-Servidor Web HFT activo en http://0.0.0.0:{} (Listo para Render & Móvil)", port);
            l
        }
        Err(e) => {
            eprintln!("[!] Fallo al enlazar servidor web en {}: {:?}", addr, e);
            return;
        }
    };

    loop {
        if let Ok((mut socket, _)) = listener.accept().await {
            let state_clone = state.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                if let Ok(n) = socket.read(&mut buf).await {
                    if n == 0 {
                        return;
                    }
                    let req = String::from_utf8_lossy(&buf[..n]);

                    let (status_line, content_type, body) = if req.starts_with("GET /health") {
                        (
                            "HTTP/1.1 200 OK\r\n",
                            "application/json",
                            r#"{"status":"ok","engine":"entropiq"}"#.to_string(),
                        )
                    } else if req.starts_with("GET /api/status") {
                        let data = state_clone.read().unwrap().clone();
                        let json = serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string());
                        ("HTTP/1.1 200 OK\r\n", "application/json", json)
                    } else {
                        let data = state_clone.read().unwrap().clone();
                        let html = render_dashboard_html(&data);
                        ("HTTP/1.1 200 OK\r\n", "text/html; charset=utf-8", html)
                    };

                    let response = format!(
                        "{}Content-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n{}",
                        status_line,
                        content_type,
                        body.len(),
                        body
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }
            });
        }
    }
}

/// Generates a modern, responsive, institutional dark-mode terminal dashboard for mobile and desktop.
fn render_dashboard_html(s: &WebDashboardState) -> String {
    let ws_status_badge = if s.websocket_connected {
        r#"<span class="badge online">● CONECTADO</span>"#
    } else {
        r#"<span class="badge offline">○ DESCONECTADO</span>"#
    };

    let toxic_badge = if s.toxic_flow {
        r#"<span class="badge offline">BLOQUEADO</span>"#
    } else {
        r#"<span class="badge online">PERMISIVO</span>"#
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta http-equiv="refresh" content="3">
  <title>Entropiq — Terminal Cuántica</title>
  <style>
    :root {{
      --bg: #090d16;
      --card-bg: #111726;
      --border: #1e293b;
      --cyan: #00f0ff;
      --green: #10b981;
      --red: #ef4444;
      --yellow: #f59e0b;
      --text: #f8fafc;
      --muted: #94a3b8;
    }}
    * {{ box-sizing: border-box; margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, monospace; }}
    body {{ background: var(--bg); color: var(--text); padding: 16px; font-size: 14px; line-height: 1.5; }}
    .container {{ max-width: 900px; margin: 0 auto; }}
    header {{ display: flex; justify-content: space-between; align-items: center; padding: 14px 18px; background: var(--card-bg); border: 1px solid var(--border); border-radius: 12px; margin-bottom: 16px; box-shadow: 0 4px 12px rgba(0,0,0,0.5); }}
    .brand {{ font-size: 18px; font-weight: 700; letter-spacing: 0.05em; color: var(--cyan); }}
    .badge {{ font-size: 11px; font-weight: 700; padding: 4px 10px; border-radius: 20px; text-transform: uppercase; }}
    .badge.online {{ background: rgba(16, 185, 129, 0.2); color: var(--green); border: 1px solid var(--green); }}
    .badge.offline {{ background: rgba(239, 68, 68, 0.2); color: var(--red); border: 1px solid var(--red); }}
    .grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 14px; margin-bottom: 16px; }}
    .card {{ background: var(--card-bg); border: 1px solid var(--border); border-radius: 12px; padding: 16px; }}
    .card-title {{ font-size: 11px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); margin-bottom: 12px; display: flex; justify-content: space-between; }}
    .stat-row {{ display: flex; justify-content: space-between; padding: 6px 0; border-bottom: 1px solid rgba(255,255,255,0.04); }}
    .stat-row:last-child {{ border-bottom: none; }}
    .stat-label {{ color: var(--muted); }}
    .stat-val {{ font-weight: 600; font-variant-numeric: tabular-nums; }}
    .highlight {{ color: var(--cyan); }}
    .gain {{ color: var(--green); }}
    .phase {{ color: var(--yellow); font-weight: 700; }}
    footer {{ text-align: center; color: var(--muted); font-size: 11px; padding: 12px; }}
  </style>
</head>
<body>
  <div class="container">
    <header>
      <div>
        <div class="brand">ENTROPIQ SUPERVISOR</div>
        <div style="font-size: 11px; color: var(--muted);">Wall Street IEX &bull; Quantum Microstructure</div>
      </div>
      <div>{ws_status_badge}</div>
    </header>

    <div class="grid">
      <!-- Card 1: PnL y Rendimiento Financiero -->
      <div class="card">
        <div class="card-title">Auditoría Cuantitativa</div>
        <div class="stat-row"><span class="stat-label">Cuenta Broker</span><span class="stat-val highlight">{}</span></div>
        <div class="stat-row"><span class="stat-label">Capital (Equity)</span><span class="stat-val gain">${:.2} USD</span></div>
        <div class="stat-row"><span class="stat-label">Poder de Compra</span><span class="stat-val">${:.2} USD</span></div>
        <div class="stat-row"><span class="stat-label">Sharpe Ratio</span><span class="stat-val highlight">{:.2}</span></div>
        <div class="stat-row"><span class="stat-label">Tasa de Acierto (WinRate)</span><span class="stat-val">{:.1}%</span></div>
        <div class="stat-row"><span class="stat-label">Profit Factor</span><span class="stat-val">{:.2}</span></div>
        <div class="stat-row"><span class="stat-label">Max Drawdown</span><span class="stat-val">{:.2}%</span></div>
        <div class="stat-row"><span class="stat-label">Apalancamiento Kelly</span><span class="stat-val highlight">{:.2}x</span></div>
        <div class="stat-row"><span class="stat-label">Operaciones Totales</span><span class="stat-val">{}</span></div>
      </div>

      <!-- Card 2: Estado Físico Cuántico y Microestructura -->
      <div class="card">
        <div class="card-title">Microestructura Cuántica</div>
        <div class="stat-row"><span class="stat-label">Fase de Mercado</span><span class="stat-val phase">{}</span></div>
        <div class="stat-row"><span class="stat-label">Confianza de Fase</span><span class="stat-val">{:.1}%</span></div>
        <div class="stat-row"><span class="stat-label">Entropía de von Neumann</span><span class="stat-val">{:.3}</span></div>
        <div class="stat-row"><span class="stat-label">Coherencia Cuántica</span><span class="stat-val highlight">{:.3}</span></div>
        <div class="stat-row"><span class="stat-label">Mach Fluídico</span><span class="stat-val">{:.2}</span></div>
        <div class="stat-row"><span class="stat-label">VPIN Flujo Tóxico</span><span class="stat-val">{:.3}</span></div>
        <div class="stat-row"><span class="stat-label">Escudo Tóxico</span><span class="stat-val">{}</span></div>
        <div class="stat-row"><span class="stat-label">Sesgo Lead-Lag SPY</span><span class="stat-val">{:.2}</span></div>
      </div>

      <!-- Card 3: Rendimiento Computacional y Latencia -->
      <div class="card">
        <div class="card-title">Métricas de Latencia (HFT)</div>
        <div class="stat-row"><span class="stat-label">Throughput</span><span class="stat-val highlight">{:.0} ticks/s</span></div>
        <div class="stat-row"><span class="stat-label">Ticks Procesados</span><span class="stat-val">{}</span></div>
        <div class="stat-row"><span class="stat-label">Latencia Media</span><span class="stat-val">{:.0} ns ({:.2} µs)</span></div>
        <div class="stat-row"><span class="stat-label">Latencia Mínima</span><span class="stat-val">{} ns</span></div>
        <div class="stat-row"><span class="stat-label">Percentil p50</span><span class="stat-val">{} ns</span></div>
        <div class="stat-row"><span class="stat-label">Percentil p99</span><span class="stat-val">{} ns</span></div>
        <div class="stat-row"><span class="stat-label">Ticks Descartados</span><span class="stat-val">{}</span></div>
        <div class="stat-row"><span class="stat-label">Memoria RAM (RSS)</span><span class="stat-val">{:.2} MB</span></div>
      </div>
    </div>

    <footer>
      Auto-refresco cada 3s &bull; Uptime: {}s &bull; Desplegado en Render Cloud &bull; Entropiq v0.1.0
    </footer>
  </div>
</body>
</html>"#,
        s.broker_account,
        s.current_equity,
        s.buying_power,
        s.sharpe_ratio,
        s.win_rate * 100.0,
        s.profit_factor,
        s.max_drawdown * 100.0,
        s.kelly_leverage,
        s.total_trades,
        s.market_phase,
        s.phase_confidence * 100.0,
        s.entropy,
        s.coherence,
        s.mach_number,
        s.vpin,
        toxic_badge,
        s.lead_lag_spy_bias,
        s.throughput_ticks_per_sec,
        s.total_processed_ticks,
        s.avg_latency_ns,
        s.avg_latency_ns / 1000.0,
        s.min_latency_ns,
        s.p50_latency_ns,
        s.p99_latency_ns,
        s.alpaca_ticks_dropped,
        s.rss_mb,
        s.uptime_secs
    )
}
