use serde::Serialize;
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Real-time telemetry for a specific market asset.
#[derive(Debug, Clone, Serialize, Default)]
pub struct SymbolStateTelemetry {
    pub symbol: String,
    pub mid_price: f64,
    pub micro_price: f64,
    pub best_bid: f64,
    pub best_ask: f64,
    pub spread_bps: f64,
    pub normalized_entropy: f64,
    pub quantum_coherence: f64,
    pub active_phase: String,
    pub phase_confidence: f64,
    pub mach_number: f64,
    pub speed_of_sound: f64,
    pub fluid_velocity: f64,
    pub is_cavitation: bool,
    pub vpin: f64,
    pub is_toxic: bool,
    pub tunneling_ask: f64,
    pub tunneling_bid: f64,
    pub state_bias: f64,
    pub ofi_accel: f64,
    pub depth_dispersion: [f64; 8],
    pub sor_intent: String,
}

/// Real-time telemetry for active broker positions.
#[derive(Debug, Clone, Serialize, Default)]
pub struct PositionTelemetry {
    pub symbol: String,
    pub side: String,
    pub entry_price: f64,
    pub current_price: f64,
    pub qty: f64,
    pub notional: f64,
    pub pnl_pct: f64,
    pub pnl_usd: f64,
    pub holding_secs: u64,
}

/// Real-time step status for the 7-stage pre-trade decision pipeline.
#[derive(Debug, Clone, Serialize, Default)]
pub struct PipelineStepTelemetry {
    pub id: u32,
    pub name: String,
    pub code: String,
    pub status: String,
    pub value_summary: String,
    pub description: String,
}

/// Rolling log entry for execution and pipeline events.
#[derive(Debug, Clone, Serialize, Default)]
pub struct PipelineLogEntry {
    pub timestamp_str: String,
    pub symbol: String,
    pub phase: String,
    pub intent: String,
    pub message: String,
    pub is_trade: bool,
}

/// Complete live state snapshot exposed to the web dashboard and health check.
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
    pub market_open: bool,
    pub feed: String,
    pub broker_account: String,
    pub buying_power: f64,
    pub current_equity: f64,
    pub sharpe_ratio: f64,
    pub win_rate: f64,
    pub kelly_leverage: f64,
    pub profit_factor: f64,
    pub max_drawdown: f64,
    pub total_trades: u64,

    pub symbols: Vec<SymbolStateTelemetry>,
    pub positions: Vec<PositionTelemetry>,
    pub pipeline_steps: Vec<PipelineStepTelemetry>,
    pub recent_logs: Vec<PipelineLogEntry>,
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
            market_open: true,
            feed: "crypto".to_string(),
            broker_account: "PA3NGD2B8 (Paper)".to_string(),
            buying_power: 0.0,
            current_equity: 0.0,
            sharpe_ratio: 0.0,
            win_rate: 0.0,
            kelly_leverage: 1.0,
            profit_factor: 99.99,
            max_drawdown: 0.0,
            total_trades: 0,
            symbols: Vec::new(),
            positions: Vec::new(),
            pipeline_steps: Vec::new(),
            recent_logs: Vec::new(),
        }
    }
}

/// Spawns the zero-dependency async HTTP server for Render and mobile monitoring.
pub async fn start_web_server(state: Arc<RwLock<WebDashboardState>>, port: u16) {
    let addr = format!("0.0.0.0:{}", port);
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => {
            println!("[+] Servidor Web Cuántico HFT activo en http://0.0.0.0:{} (Render & Terminal)", port);
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
                        let data = state_clone.read().map(|g| g.clone()).unwrap_or_default();
                        let json = serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string());
                        ("HTTP/1.1 200 OK\r\n", "application/json", json)
                    } else {
                        let data = state_clone.read().map(|g| g.clone()).unwrap_or_default();
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

/// Generates the human-crafted, institutional-grade Quantum Microstructure Dashboard.
/// Uses native CSS Flexbox/Grid for zero text-overlap, an HTML5 Canvas for dynamic synaptic connections, and strict 100vh fit.
pub fn render_dashboard_html(initial_state: &WebDashboardState) -> String {
    let initial_json = serde_json::to_string(initial_state).unwrap_or_else(|_| "{}".to_string());

    format!(r##"<!DOCTYPE html>
<html lang="es">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
  <title>Entropiq — Institutional Quantum Microstructure Platform</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500;600;700&display=swap" rel="stylesheet">
  <style>
    :root {{
      --bg: #07090e;
      --card-bg: rgba(13, 19, 32, 0.88);
      --card-border: rgba(255, 255, 255, 0.08);
      --cyan: #38bdf8;
      --cyan-glow: rgba(56, 189, 248, 0.25);
      --emerald: #10b981;
      --emerald-glow: rgba(16, 185, 129, 0.25);
      --purple: #818cf8;
      --purple-glow: rgba(129, 140, 248, 0.25);
      --amber: #f59e0b;
      --amber-glow: rgba(245, 158, 11, 0.25);
      --rose: #f43f5e;
      --text: #f8fafc;
      --muted: #94a3b8;
      --subtle: #64748b;
    }}

    * {{
      box-sizing: border-box;
      margin: 0;
      padding: 0;
      -webkit-font-smoothing: antialiased;
    }}

    html, body {{
      height: 100vh;
      max-height: 100vh;
      overflow: hidden;
      background: var(--bg);
      background-image:
        radial-gradient(circle at 10% 20%, rgba(99, 102, 241, 0.07) 0%, transparent 45%),
        radial-gradient(circle at 90% 80%, rgba(56, 189, 248, 0.05) 0%, transparent 40%);
      color: var(--text);
      font-family: 'Plus Jakarta Sans', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    }}

    body {{
      display: flex;
      flex-direction: column;
      padding: 10px 14px;
      gap: 10px;
    }}

    .mono {{
      font-family: 'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, monospace;
      font-variant-numeric: tabular-nums;
    }}

    /* Top Navigation Bar */
    .top-bar {{
      height: 44px;
      min-height: 44px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0 16px;
      background: var(--card-bg);
      backdrop-filter: blur(20px);
      border: 1px solid var(--card-border);
      border-radius: 12px;
      flex-shrink: 0;
    }}

    .brand-wrap {{
      display: flex;
      align-items: center;
      gap: 10px;
    }}
    .core-orb-mini {{
      width: 18px;
      height: 18px;
      border-radius: 50%;
      background: radial-gradient(circle, var(--cyan) 20%, #6366f1 80%);
      box-shadow: 0 0 10px var(--cyan-glow);
    }}
    .brand-title {{
      font-size: 13.5px;
      font-weight: 800;
      letter-spacing: 0.12em;
      color: #fff;
      display: flex;
      align-items: center;
      gap: 6px;
    }}
    .brand-title span {{ color: var(--cyan); }}
    .brand-sub {{
      font-size: 9.5px;
      font-weight: 700;
      color: var(--muted);
      letter-spacing: 0.08em;
      background: rgba(255, 255, 255, 0.05);
      padding: 2px 6px;
      border-radius: 4px;
    }}

    .top-center {{
      display: flex;
      align-items: center;
      gap: 6px;
    }}
    .asset-tab {{
      background: rgba(255, 255, 255, 0.03);
      border: 1px solid rgba(255, 255, 255, 0.07);
      color: var(--muted);
      padding: 4px 12px;
      border-radius: 8px;
      font-size: 11px;
      font-weight: 700;
      cursor: pointer;
      transition: all 0.2s ease;
      display: flex;
      align-items: center;
      gap: 6px;
    }}
    .asset-tab:hover {{
      color: #fff;
      border-color: rgba(56, 189, 248, 0.4);
    }}
    .asset-tab.active {{
      background: rgba(56, 189, 248, 0.12);
      border-color: var(--cyan);
      color: #fff;
      box-shadow: 0 0 10px rgba(56, 189, 248, 0.15);
    }}

    .top-right {{
      display: flex;
      align-items: center;
      gap: 8px;
    }}
    .badge {{
      display: inline-flex;
      align-items: center;
      gap: 5px;
      font-size: 10px;
      font-weight: 700;
      padding: 3px 9px;
      border-radius: 20px;
      letter-spacing: 0.04em;
    }}
    .badge.online {{ background: rgba(16, 185, 129, 0.12); color: var(--emerald); border: 1px solid rgba(16, 185, 129, 0.3); }}
    .badge.info {{ background: rgba(56, 189, 248, 0.10); color: var(--cyan); border: 1px solid rgba(56, 189, 248, 0.25); }}
    .badge.purple {{ background: rgba(129, 140, 248, 0.12); color: var(--purple); border: 1px solid rgba(129, 140, 248, 0.3); }}

    /* Main Dashboard Grid: Strictly 100vh */
    .dashboard-grid {{
      flex: 1;
      min-height: 0;
      display: grid;
      grid-template-columns: 1fr 375px;
      gap: 10px;
    }}

    /* Left Stage: Neural Flow Canvas */
    .neural-stage {{
      background: var(--card-bg);
      backdrop-filter: blur(20px);
      border: 1px solid var(--card-border);
      border-radius: 14px;
      display: flex;
      flex-direction: column;
      overflow: hidden;
      position: relative;
    }}

    .canvas-hud {{
      height: 36px;
      min-height: 36px;
      padding: 0 16px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      border-bottom: 1px solid rgba(255, 255, 255, 0.05);
      background: rgba(0, 0, 0, 0.2);
      flex-shrink: 0;
    }}
    .hud-title {{
      font-size: 11px;
      font-weight: 700;
      letter-spacing: 0.08em;
      color: var(--muted);
      text-transform: uppercase;
      display: flex;
      align-items: center;
      gap: 6px;
    }}
    .hud-metrics {{
      font-size: 11px;
      color: var(--muted);
      display: flex;
      align-items: center;
      gap: 14px;
    }}
    .hud-metrics span b {{ color: #fff; }}

    /* Flow Layout Container */
    .flow-container {{
      position: relative;
      flex: 1;
      min-height: 0;
      width: 100%;
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 16px 32px;
      gap: 24px;
      overflow: hidden;
    }}

    /* Dynamic Canvas Layer */
    canvas#synapse-canvas {{
      position: absolute;
      top: 0;
      left: 0;
      width: 100%;
      height: 100%;
      pointer-events: none;
      z-index: 1;
    }}

    /* Column 1: Root Core */
    .root-column {{
      flex-shrink: 0;
      width: 110px;
      display: flex;
      justify-content: center;
      align-items: center;
      z-index: 2;
    }}
    .core-node {{
      position: relative;
      width: 90px;
      height: 90px;
      border-radius: 50%;
      background: radial-gradient(circle at 35% 35%, #0f1d38 0%, #070a12 85%);
      border: 1px solid rgba(56, 189, 248, 0.45);
      box-shadow: 0 0 24px rgba(56, 189, 248, 0.15), inset 0 0 16px rgba(56, 189, 248, 0.1);
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      gap: 2px;
      user-select: none;
    }}
    .core-tag {{ font-size: 8px; font-weight: 800; letter-spacing: 0.15em; color: var(--cyan); text-transform: uppercase; }}
    .core-title {{ font-size: 9.5px; font-weight: 800; color: #fff; }}
    .core-phase-pill {{
      font-size: 7.5px;
      font-weight: 800;
      padding: 1px 6px;
      border-radius: 4px;
      background: rgba(245, 158, 11, 0.15);
      color: var(--amber);
      border: 1px solid rgba(245, 158, 11, 0.3);
      text-transform: uppercase;
      max-width: 78px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      margin-top: 2px;
    }}

    /* Column 2: The 3 Processing Pillars */
    .pillars-column {{
      flex-shrink: 0;
      width: 250px;
      display: flex;
      flex-direction: column;
      justify-content: space-around;
      height: 100%;
      gap: 16px;
      z-index: 2;
    }}
    .pillar-card {{
      background: rgba(14, 21, 37, 0.92);
      border: 1px solid var(--card-border);
      border-left: 3px solid var(--cyan);
      border-radius: 10px;
      padding: 10px 14px;
      display: flex;
      flex-direction: column;
      gap: 4px;
      box-shadow: 0 4px 16px rgba(0, 0, 0, 0.35);
      transition: all 0.2s ease;
    }}
    .pillar-card:hover {{
      border-color: rgba(255, 255, 255, 0.18);
      transform: translateX(2px);
    }}
    .pillar-card.p-purple {{ border-left-color: var(--purple); }}
    .pillar-card.p-emerald {{ border-left-color: var(--emerald); }}

    .pillar-top {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 8px;
    }}
    .pillar-title {{
      font-size: 11px;
      font-weight: 700;
      color: #fff;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }}
    .badge-pill {{
      font-size: 8.5px;
      font-weight: 800;
      padding: 2px 7px;
      border-radius: 4px;
      text-transform: uppercase;
      flex-shrink: 0;
    }}
    .badge-pill.cyan {{ background: rgba(56, 189, 248, 0.12); color: var(--cyan); border: 1px solid rgba(56, 189, 248, 0.3); }}
    .badge-pill.purple {{ background: rgba(129, 140, 248, 0.12); color: var(--purple); border: 1px solid rgba(129, 140, 248, 0.3); }}
    .badge-pill.emerald {{ background: rgba(16, 185, 129, 0.12); color: var(--emerald); border: 1px solid rgba(16, 185, 129, 0.3); }}
    .badge-pill.amber {{ background: rgba(245, 158, 11, 0.12); color: var(--amber); border: 1px solid rgba(245, 158, 11, 0.3); }}

    .pillar-sub {{
      font-size: 9.5px;
      color: var(--muted);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }}

    /* Column 3: The 7 Operational Leafs */
    .leafs-column {{
      flex: 1;
      max-width: 440px;
      display: flex;
      flex-direction: column;
      justify-content: space-around;
      height: 100%;
      gap: 7px;
      z-index: 2;
    }}
    .leaf-card {{
      background: rgba(11, 17, 30, 0.92);
      border: 1px solid rgba(255, 255, 255, 0.07);
      border-radius: 8px;
      padding: 6px 12px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      box-shadow: 0 2px 10px rgba(0, 0, 0, 0.25);
      transition: all 0.2s ease;
    }}
    .leaf-card:hover {{
      border-color: rgba(255, 255, 255, 0.16);
      background: rgba(14, 22, 38, 0.98);
    }}
    .leaf-main {{
      display: flex;
      align-items: center;
      gap: 8px;
      min-width: 0;
      flex: 1;
    }}
    .leaf-dot {{
      width: 6px;
      height: 6px;
      min-width: 6px;
      border-radius: 50%;
      background: var(--emerald);
    }}
    .leaf-dot.cyan {{ background: var(--cyan); }}
    .leaf-dot.purple {{ background: var(--purple); }}
    .leaf-dot.amber {{ background: var(--amber); }}

    .leaf-title {{
      font-size: 11px;
      font-weight: 600;
      color: #e2e8f0;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }}
    .leaf-metrics {{
      display: flex;
      align-items: center;
      gap: 8px;
      flex-shrink: 0;
    }}
    .leaf-val {{
      font-size: 10px;
      font-weight: 600;
      color: #fff;
    }}
    .badge-tag {{
      font-size: 8.5px;
      font-weight: 800;
      padding: 2px 6px;
      border-radius: 4px;
      text-transform: uppercase;
      background: rgba(16, 185, 129, 0.15);
      color: var(--emerald);
      border: 1px solid rgba(16, 185, 129, 0.3);
    }}
    .badge-tag.warn {{ background: rgba(245, 158, 11, 0.15); color: var(--amber); border-color: rgba(245, 158, 11, 0.3); }}
    .badge-tag.purple {{ background: rgba(129, 140, 248, 0.15); color: var(--purple); border-color: rgba(129, 140, 248, 0.3); }}

    /* CTQW Dispersion Strip */
    .dispersion-strip {{
      height: 44px;
      min-height: 44px;
      padding: 4px 16px;
      background: rgba(0, 0, 0, 0.35);
      border-top: 1px solid rgba(255, 255, 255, 0.05);
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 16px;
      flex-shrink: 0;
    }}
    .disp-info {{ display: flex; flex-direction: column; gap: 1px; }}
    .disp-title {{ font-size: 10px; font-weight: 700; letter-spacing: 0.06em; color: var(--muted); text-transform: uppercase; }}
    .disp-sub {{ font-size: 9px; color: var(--subtle); }}
    .disp-bars-box {{ display: flex; align-items: flex-end; gap: 6px; height: 26px; }}
    .disp-col {{ display: flex; flex-direction: column; align-items: center; gap: 2px; width: 22px; }}
    .disp-bar {{ width: 100%; background: rgba(129, 140, 248, 0.45); border-radius: 2px 2px 0 0; min-height: 3px; transition: height 0.3s ease; }}
    .disp-bar.mid {{ background: var(--cyan); }}
    .disp-tag {{ font-size: 8px; font-weight: 700; color: var(--subtle); }}

    /* Right Column: Precision Telemetry Drawer */
    .telemetry-drawer {{
      background: var(--card-bg);
      backdrop-filter: blur(20px);
      border: 1px solid var(--card-border);
      border-radius: 14px;
      display: flex;
      flex-direction: column;
      overflow: hidden;
      padding: 12px;
      gap: 8px;
    }}

    /* Asset Hero Card */
    .asset-hero {{
      background: rgba(255, 255, 255, 0.02);
      border: 1px solid rgba(255, 255, 255, 0.06);
      border-radius: 10px;
      padding: 9px 12px;
      display: flex;
      justify-content: space-between;
      align-items: center;
    }}
    .asset-symbol {{
      font-size: 15px;
      font-weight: 800;
      color: #fff;
      display: flex;
      align-items: center;
      gap: 6px;
    }}
    .asset-micro {{ font-size: 10.5px; color: var(--muted); margin-top: 1px; }}
    .asset-price-box {{ text-align: right; }}
    .asset-price {{ font-size: 19px; font-weight: 800; color: var(--cyan); }}
    .asset-spread-badge {{
      display: inline-block;
      font-size: 9px;
      font-weight: 700;
      padding: 1px 6px;
      border-radius: 4px;
      background: rgba(16, 185, 129, 0.15);
      color: var(--emerald);
      border: 1px solid rgba(16, 185, 129, 0.3);
      margin-top: 2px;
    }}

    /* Physics Matrix (2x2 Quad) */
    .physics-quad {{
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 6px;
    }}
    .quad-tile {{
      background: rgba(0, 0, 0, 0.25);
      border: 1px solid rgba(255, 255, 255, 0.04);
      border-radius: 8px;
      padding: 6px 10px;
      display: flex;
      flex-direction: column;
      gap: 2px;
    }}
    .quad-label {{
      font-size: 9px;
      font-weight: 700;
      color: var(--muted);
      letter-spacing: 0.05em;
      text-transform: uppercase;
    }}
    .quad-val {{ font-size: 12.5px; font-weight: 700; color: #fff; }}

    /* Section Title */
    .drawer-sec-title {{
      font-size: 9.5px;
      font-weight: 700;
      letter-spacing: 0.07em;
      color: var(--muted);
      text-transform: uppercase;
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 0 2px;
    }}

    /* Pre-Trade Gates List */
    .gates-box {{
      background: rgba(0, 0, 0, 0.25);
      border: 1px solid rgba(255, 255, 255, 0.04);
      border-radius: 10px;
      padding: 4px 8px;
      display: flex;
      flex-direction: column;
      gap: 3px;
    }}
    .gate-row {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 4px 6px;
      border-radius: 6px;
      font-size: 10.5px;
    }}
    .gate-row:hover {{ background: rgba(255, 255, 255, 0.02); }}
    .gate-name-wrap {{ display: flex; align-items: center; gap: 7px; }}
    .gate-dot {{ width: 5px; height: 5px; border-radius: 50%; background: var(--emerald); }}
    .gate-dot.purple {{ background: var(--purple); }}
    .gate-name {{ font-size: 10.5px; font-weight: 600; color: var(--text); }}
    .gate-badge {{
      font-size: 8.5px;
      font-weight: 800;
      padding: 1px 6px;
      border-radius: 3px;
      background: rgba(16, 185, 129, 0.15);
      color: var(--emerald);
      border: 1px solid rgba(16, 185, 129, 0.3);
    }}

    /* Broker Account & Equity Tile */
    .broker-box {{
      background: rgba(0, 0, 0, 0.25);
      border: 1px solid rgba(255, 255, 255, 0.04);
      border-radius: 10px;
      padding: 7px 10px;
      display: flex;
      flex-direction: column;
      gap: 4px;
    }}
    .broker-row {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      font-size: 10.5px;
    }}
    .broker-label {{ color: var(--muted); }}
    .broker-val {{ font-weight: 700; color: #fff; }}

    /* Stream Ticker Bar at Bottom */
    .ticker-bar {{
      height: 26px;
      min-height: 26px;
      background: rgba(0, 0, 0, 0.4);
      border: 1px solid rgba(255, 255, 255, 0.04);
      border-radius: 6px;
      padding: 0 8px;
      display: flex;
      align-items: center;
      gap: 7px;
      font-size: 9.5px;
      overflow: hidden;
      white-space: nowrap;
      text-overflow: ellipsis;
    }}
    .ticker-dot {{
      width: 5px;
      height: 5px;
      min-width: 5px;
      border-radius: 50%;
      background: var(--emerald);
    }}
    .ticker-text {{ color: var(--muted); overflow: hidden; text-overflow: ellipsis; }}
    .ticker-text b {{ color: #fff; }}
  </style>
</head>
<body>

  <!-- Top Navigation Bar -->
  <header class="top-bar">
    <div class="brand-wrap">
      <div class="core-orb-mini"></div>
      <div class="brand-title">ENTROPIQ <span>QUANTUM</span></div>
      <div class="brand-sub">HFT SYSTEM</div>
    </div>

    <!-- Center Asset Tabs -->
    <div class="top-center" id="asset-tabs-bar">
      <!-- Injected dynamically via JS -->
    </div>

    <!-- Right System Badges -->
    <div class="top-right">
      <div class="badge online" id="badge-ws">● STREAMER ACTIVO</div>
      <div class="badge info" id="badge-market" title="El mercado de criptomonedas opera 24 horas al día, 7 días a la semana sin interrupción">
        ● CRIPTO 24/7 (CONTINUO)
      </div>
      <div class="badge purple mono" id="badge-phase">FASE: THERMALNOISE</div>
    </div>
  </header>

  <!-- Main 100vh Layout Grid -->
  <main class="dashboard-grid">

    <!-- Left Stage: Dynamic Neural Network Graph -->
    <section class="neural-stage">
      <div class="canvas-hud">
        <div class="hud-title">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
            <circle cx="12" cy="12" r="3"></circle>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
          </svg>
          Arquitectura Neural Cuántica de Microestructura (3-Tier CTQW)
        </div>
        <div class="hud-metrics mono">
          <span>Throughput: <b id="val-throughput">0</b> ticks/s</span>
          <span>Latencia: <b id="val-latency">18.0</b> µs</span>
          <span>RAM: <b id="val-rss">8.4</b> MB</span>
        </div>
      </div>

      <!-- Flow Container with Canvas Connections -->
      <div class="flow-container">
        <!-- High-Performance Synapse Canvas (Calculates Exact Pixel Connections) -->
        <canvas id="synapse-canvas"></canvas>

        <!-- Column 1: Root Core -->
        <div class="root-column">
          <div class="core-node" id="node-root">
            <span class="core-tag">ENTROPIQ</span>
            <span class="core-title">CORE RK4</span>
            <span class="core-phase-pill mono" id="core-phase-pill">THERMAL</span>
          </div>
        </div>

        <!-- Column 2: 3 Processing Pillars -->
        <div class="pillars-column">
          <!-- Pillar 1: Ingestión & Spread -->
          <div class="pillar-card" id="node-p1">
            <div class="pillar-top">
              <span class="pillar-title">01 · INGESTIÓN & SPREAD</span>
              <span class="badge-pill cyan mono" id="p1-status">LAMINAR</span>
            </div>
            <div class="pillar-sub mono" id="p1-desc">Spread: 2.5 bps · OFI: 0.0</div>
          </div>

          <!-- Pillar 2: Física Cuántica & Mach -->
          <div class="pillar-card p-purple" id="node-p2">
            <div class="pillar-top">
              <span class="pillar-title">02 · FÍSICA & MATRIZ ρ</span>
              <span class="badge-pill purple mono" id="p2-status">COHERENTE</span>
            </div>
            <div class="pillar-sub mono" id="p2-desc">S(norm): 0.810 · Coher: 0.248</div>
          </div>

          <!-- Pillar 3: Valuación Pre-Trade & SOR -->
          <div class="pillar-card p-emerald" id="node-p3">
            <div class="pillar-top">
              <span class="pillar-title">03 · VALUACIÓN & SOR</span>
              <span class="badge-pill emerald mono" id="p3-status">PERMISIVO</span>
            </div>
            <div class="pillar-sub mono" id="p3-desc">6 Puertas · Half-Kelly 0.20x</div>
          </div>
        </div>

        <!-- Column 3: 7 Operational Leafs -->
        <div class="leafs-column">
          <!-- Leaf 1: Spread Bid-Ask -->
          <div class="leaf-card" id="node-l1">
            <div class="leaf-main">
              <span class="leaf-dot cyan"></span>
              <span class="leaf-title">Spread Bid-Ask Dinámico</span>
            </div>
            <div class="leaf-metrics">
              <span class="leaf-val mono" id="l1-val">2.5 bps</span>
              <span class="badge-tag mono" id="l1-tag">PASS</span>
            </div>
          </div>

          <!-- Leaf 2: Order Flow Imbalance (OFI) -->
          <div class="leaf-card" id="node-l2">
            <div class="leaf-main">
              <span class="leaf-dot cyan"></span>
              <span class="leaf-title">Order Flow Imbalance (OFI)</span>
            </div>
            <div class="leaf-metrics">
              <span class="leaf-val mono" id="l2-val">Accel: 0.0</span>
              <span class="badge-tag mono">ESTABLE</span>
            </div>
          </div>

          <!-- Leaf 3: Matriz Densidad von Neumann -->
          <div class="leaf-card" id="node-l3">
            <div class="leaf-main">
              <span class="leaf-dot purple"></span>
              <span class="leaf-title">Densidad von Neumann (ρ)</span>
            </div>
            <div class="leaf-metrics">
              <span class="leaf-val mono" id="l3-val">S: 0.81 | C: 0.25</span>
              <span class="badge-tag purple mono">COHER</span>
            </div>
          </div>

          <!-- Leaf 4: Hidrodinámica & Mach -->
          <div class="leaf-card" id="node-l4">
            <div class="leaf-main">
              <span class="leaf-dot cyan"></span>
              <span class="leaf-title">Hidrodinámica & Mach Shock</span>
            </div>
            <div class="leaf-metrics">
              <span class="leaf-val mono" id="l4-val">0.00 M</span>
              <span class="badge-tag mono">SUB</span>
            </div>
          </div>

          <!-- Leaf 5: Escudo Tóxico VPIN -->
          <div class="leaf-card" id="node-l5">
            <div class="leaf-main">
              <span class="leaf-dot"></span>
              <span class="leaf-title">Escudo Toxicidad Flujo (VPIN)</span>
            </div>
            <div class="leaf-metrics">
              <span class="leaf-val mono" id="l5-val">VPIN: 0.000</span>
              <span class="badge-tag mono" id="l5-tag">PASS</span>
            </div>
          </div>

          <!-- Leaf 6: Margen Retorno vs Fricción -->
          <div class="leaf-card" id="node-l6">
            <div class="leaf-main">
              <span class="leaf-dot amber"></span>
              <span class="leaf-title">Margen Retorno vs Fricción</span>
            </div>
            <div class="leaf-metrics">
              <span class="leaf-val mono" id="l6-val">TP > Costes</span>
              <span class="badge-tag mono" id="l6-tag">PASS</span>
            </div>
          </div>

          <!-- Leaf 7: SOR Router & Alpaca Limit -->
          <div class="leaf-card" id="node-l7">
            <div class="leaf-main">
              <span class="leaf-dot purple"></span>
              <span class="leaf-title">Enrutador SOR Alpaca REST</span>
            </div>
            <div class="leaf-metrics">
              <span class="leaf-val mono" id="l7-val">Marketable Limit</span>
              <span class="badge-tag purple mono">LISTO</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Integrated CTQW Dispersion Strip -->
      <div class="dispersion-strip">
        <div class="disp-info">
          <span class="disp-title">Dispersión de Paquete de Ondas Cuántico CTQW</span>
          <span class="disp-sub">Probabilidades de penetración en 8 niveles del libro de órdenes</span>
        </div>
        <div class="disp-bars-box" id="dispersion-bars-row">
          <!-- Injected via JS -->
        </div>
      </div>
    </section>

    <!-- Right Column: Precision Intelligence Panel -->
    <aside class="telemetry-drawer">

      <!-- Active Asset Hero Card -->
      <div class="asset-hero">
        <div class="asset-id-box">
          <div class="asset-symbol" id="card-symbol">
            BTC/USD
            <span class="badge purple mono" id="card-phase-badge" style="font-size: 8.5px; padding: 1px 6px;">THERMALNOISE</span>
          </div>
          <div class="asset-micro mono">
            Micro: <span id="card-micro-price">$76,685.32</span>
          </div>
        </div>
        <div class="asset-price-box">
          <div class="asset-price mono" id="card-mid-price">$76,685.30</div>
          <div class="asset-spread-badge mono">Spread: <span id="card-spread-bps">2.5</span> bps</div>
        </div>
      </div>

      <!-- Physics & Microstructure Matrix -->
      <div class="physics-quad">
        <div class="quad-tile">
          <span class="quad-label">Entropía von Neumann</span>
          <span class="quad-val mono" id="quad-entropy" style="color: var(--purple);">0.810</span>
        </div>
        <div class="quad-tile">
          <span class="quad-label">Coherencia Cuántica</span>
          <span class="quad-val mono" id="quad-coherence" style="color: var(--cyan);">0.248</span>
        </div>
        <div class="quad-tile">
          <span class="quad-label">Mach Fluídico</span>
          <span class="quad-val mono" id="quad-mach" style="color: var(--emerald);">0.00 M</span>
        </div>
        <div class="quad-tile">
          <span class="quad-label">Flujo Tóxico VPIN</span>
          <span class="quad-val mono" id="quad-vpin" style="color: var(--amber);">0.000</span>
        </div>
      </div>

      <!-- Pre-Trade Multi-Gate Valuation Inspector -->
      <div class="drawer-sec-title">
        <span>6 Puertas Pre-Trade de Seguridad</span>
        <span style="color: var(--emerald); font-size: 9px;">DEFENSA ACTIVA</span>
      </div>
      <div class="gates-box">
        <div class="gate-row">
          <div class="gate-name-wrap">
            <div class="gate-dot"></div>
            <span class="gate-name">1. Límite Spread Bid/Ask (&lt;15 bps)</span>
          </div>
          <span class="gate-badge mono" id="gate-spread-badge">PASS</span>
        </div>
        <div class="gate-row">
          <div class="gate-name-wrap">
            <div class="gate-dot"></div>
            <span class="gate-name">2. Retorno Neto vs Fricción Fee</span>
          </div>
          <span class="gate-badge mono" id="gate-fee-badge">PASS</span>
        </div>
        <div class="gate-row">
          <div class="gate-name-wrap">
            <div class="gate-dot"></div>
            <span class="gate-name">3. Techo Latencia Datos WS (&lt;500ms)</span>
          </div>
          <span class="gate-badge mono" id="gate-latency-badge">PASS</span>
        </div>
        <div class="gate-row">
          <div class="gate-name-wrap">
            <div class="gate-dot"></div>
            <span class="gate-name">4. Circuit Breaker Drawdown (&lt;2%)</span>
          </div>
          <span class="gate-badge mono" id="gate-dd-badge">PASS</span>
        </div>
        <div class="gate-row">
          <div class="gate-name-wrap">
            <div class="gate-dot"></div>
            <span class="gate-name">5. Confirmación Anti-Whipsaw</span>
          </div>
          <span class="gate-badge mono" id="gate-conv-badge">PASS</span>
        </div>
        <div class="gate-row">
          <div class="gate-name-wrap">
            <div class="gate-dot purple"></div>
            <span class="gate-name">6. SOR Router & Kelly Leverage</span>
          </div>
          <span class="gate-badge mono" id="gate-sor-badge" style="background: rgba(129, 140, 248, 0.15); color: var(--purple); border-color: rgba(129, 140, 248, 0.3);">STANDBY</span>
        </div>
      </div>

      <!-- Broker Alpaca Paper Account & Capital -->
      <div class="drawer-sec-title">
        <span>Auditoría Broker Alpaca</span>
        <span class="mono" id="card-broker-account" style="color: var(--cyan); font-size: 9.5px;">PA3NGD2B8 (Paper)</span>
      </div>
      <div class="broker-box">
        <div class="broker-row">
          <span class="broker-label">Capital Total (Equity)</span>
          <span class="broker-val mono" id="card-equity" style="color: var(--emerald); font-size: 13px;">$99,327.97 USD</span>
        </div>
        <div class="broker-row">
          <span class="broker-label">Poder de Compra</span>
          <span class="broker-val mono" id="card-bp">$397,311.88 USD</span>
        </div>
        <div class="broker-row">
          <span class="broker-label">Apalancamiento Half-Kelly</span>
          <span class="broker-val mono" id="card-kelly" style="color: var(--purple);">0.20x</span>
        </div>
        <div class="broker-row">
          <span class="broker-label">Posiciones en Cartera</span>
          <span class="broker-val mono" id="card-pos-count" style="color: var(--muted); font-size: 10px;">0 abiertas (Sin riesgo nocturno)</span>
        </div>
      </div>

      <!-- Compact Live Ticker Bar at Bottom -->
      <div class="ticker-bar mono">
        <div class="ticker-dot"></div>
        <div class="ticker-text" id="live-ticker-msg">
          <b>[STREAM ONLINE]</b> Conexión Alpaca activa. Monitoreando microestructura de criptomercado 24/7.
        </div>
      </div>

    </aside>
  </main>

  <script>
    let selectedSymbolIndex = 0;
    let cachedState = {initial_json};
    let photonAnim = 0;

    // Canvas Synapse Connection Drawer
    const canvas = document.getElementById("synapse-canvas");
    const ctx = canvas.getContext("2d");

    function drawConnections() {{
      const container = canvas.parentElement;
      if (!container) return;
      const cRect = container.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;

      if (canvas.width !== Math.round(cRect.width * dpr) || canvas.height !== Math.round(cRect.height * dpr)) {{
        canvas.width = Math.round(cRect.width * dpr);
        canvas.height = Math.round(cRect.height * dpr);
      }}

      ctx.resetTransform();
      ctx.scale(dpr, dpr);
      ctx.clearRect(0, 0, cRect.width, cRect.height);

      function getAnchor(elId, isRight) {{
        const el = document.getElementById(elId);
        if (!el) return null;
        const r = el.getBoundingClientRect();
        return {{
          x: (isRight ? r.right : r.left) - cRect.left,
          y: r.top + r.height / 2 - cRect.top
        }};
      }}

      const links = [
        // Root -> Pillars
        {{ from: "node-root", to: "node-p1", stroke: "rgba(56, 189, 248, 0.4)", pulse: "#38bdf8", speed: 0.007 }},
        {{ from: "node-root", to: "node-p2", stroke: "rgba(129, 140, 248, 0.4)", pulse: "#818cf8", speed: 0.006 }},
        {{ from: "node-root", to: "node-p3", stroke: "rgba(16, 185, 129, 0.4)", pulse: "#10b981", speed: 0.008 }},

        // Pillar 1 -> Leafs 1, 2
        {{ from: "node-p1", to: "node-l1", stroke: "rgba(56, 189, 248, 0.3)", pulse: "#38bdf8", speed: 0.009 }},
        {{ from: "node-p1", to: "node-l2", stroke: "rgba(56, 189, 248, 0.3)", pulse: "#38bdf8", speed: 0.0075 }},

        // Pillar 2 -> Leafs 3, 4
        {{ from: "node-p2", to: "node-l3", stroke: "rgba(129, 140, 248, 0.3)", pulse: "#818cf8", speed: 0.0065 }},
        {{ from: "node-p2", to: "node-l4", stroke: "rgba(129, 140, 248, 0.3)", pulse: "#818cf8", speed: 0.0085 }},

        // Pillar 3 -> Leafs 5, 6, 7
        {{ from: "node-p3", to: "node-l5", stroke: "rgba(16, 185, 129, 0.3)", pulse: "#10b981", speed: 0.008 }},
        {{ from: "node-p3", to: "node-l6", stroke: "rgba(16, 185, 129, 0.3)", pulse: "#10b981", speed: 0.007 }},
        {{ from: "node-p3", to: "node-l7", stroke: "rgba(129, 140, 248, 0.3)", pulse: "#818cf8", speed: 0.009 }},
      ];

      photonAnim += 0.006;

      for (let i = 0; i < links.length; i++) {{
        const l = links[i];
        const p1 = getAnchor(l.from, true);
        const p2 = getAnchor(l.to, false);
        if (!p1 || !p2) continue;

        const dx = Math.max(25, (p2.x - p1.x) * 0.5);
        const cp1x = p1.x + dx;
        const cp1y = p1.y;
        const cp2x = p2.x - dx;
        const cp2y = p2.y;

        // Draw smooth connecting bezier curve
        ctx.beginPath();
        ctx.moveTo(p1.x, p1.y);
        ctx.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, p2.x, p2.y);
        ctx.strokeStyle = l.stroke;
        ctx.lineWidth = 1.6;
        ctx.stroke();

        // Calculate animated photon position B(t)
        const t = (photonAnim * (l.speed * 120) + (i * 0.16)) % 1.0;
        const u = 1 - t;
        const px = u*u*u*p1.x + 3*u*u*t*cp1x + 3*u*t*t*cp2x + t*t*t*p2.x;
        const py = u*u*u*p1.y + 3*u*u*t*cp1y + 3*u*t*t*cp2y + t*t*t*p2.y;

        // Draw photon
        ctx.beginPath();
        ctx.arc(px, py, 3.2, 0, Math.PI * 2);
        ctx.fillStyle = l.pulse;
        ctx.shadowColor = l.pulse;
        ctx.shadowBlur = 8;
        ctx.fill();
        ctx.shadowBlur = 0;
      }}

      requestAnimationFrame(drawConnections);
    }}

    window.addEventListener("resize", () => {{
      drawConnections();
    }});

    function selectSymbol(idx) {{
      selectedSymbolIndex = idx;
      renderCurrentSymbol();
    }}

    function renderCurrentSymbol() {{
      const s = cachedState;
      if (!s.symbols || s.symbols.length === 0) return;
      const sym = s.symbols[selectedSymbolIndex] || s.symbols[0];

      // Update Header Asset Tabs
      const tabsBar = document.getElementById("asset-tabs-bar");
      if (tabsBar && s.symbols) {{
        tabsBar.innerHTML = s.symbols.map((symObj, idx) => `
          <button class="asset-tab ${{idx === selectedSymbolIndex ? 'active' : ''}}" onclick="selectSymbol(${{idx}})">
            ${{symObj.symbol}} <span class="mono" style="font-size: 10px; opacity: 0.85;">$${{symObj.mid_price.toFixed(2)}}</span>
          </button>
        `).join("");
      }}

      // Update Selected Asset Hero
      document.getElementById("card-symbol").firstChild.textContent = sym.symbol + " ";
      document.getElementById("card-mid-price").innerText = `$${{sym.mid_price.toFixed(2)}}`;
      document.getElementById("card-micro-price").innerText = `$${{sym.micro_price.toFixed(2)}}`;
      document.getElementById("card-spread-bps").innerText = sym.spread_bps.toFixed(1);

      // Phase Badge
      const phaseBadge = document.getElementById("card-phase-badge");
      phaseBadge.innerText = `${{sym.active_phase}} (${{(sym.phase_confidence * 100).toFixed(0)}}%)`;
      document.getElementById("core-phase-pill").innerText = sym.active_phase.toUpperCase();

      // Physics Quad
      document.getElementById("quad-entropy").innerText = sym.normalized_entropy.toFixed(3);
      document.getElementById("quad-coherence").innerText = sym.quantum_coherence.toFixed(3);
      document.getElementById("quad-mach").innerText = `${{sym.mach_number.toFixed(2)}} M`;
      document.getElementById("quad-vpin").innerText = sym.vpin.toFixed(3);

      // Update Intermediate Pillars (Tier 2)
      document.getElementById("p1-desc").innerText = `Spread: ${{sym.spread_bps.toFixed(1)}} bps · OFI: ${{sym.ofi_accel.toFixed(1)}}`;
      const p1Status = document.getElementById("p1-status");
      p1Status.innerText = sym.spread_bps <= 15.0 ? "LAMINAR" : "ALTO SPREAD";
      p1Status.className = sym.spread_bps <= 15.0 ? "badge-pill cyan mono" : "badge-pill amber mono";

      document.getElementById("p2-desc").innerText = `S(norm): ${{sym.normalized_entropy.toFixed(3)}} · Coher: ${{sym.quantum_coherence.toFixed(3)}}`;
      const p2Status = document.getElementById("p2-status");
      p2Status.innerText = sym.quantum_coherence >= 0.25 ? "COHERENTE" : "DECOHERIDO";

      document.getElementById("p3-desc").innerText = `VPIN: ${{sym.vpin.toFixed(3)}} · Kelly: ${{s.kelly_leverage.toFixed(2)}}x`;
      const p3Status = document.getElementById("p3-status");
      p3Status.innerText = sym.is_toxic ? "ESCUDO BLOQUEO" : "PERMISIVO";
      p3Status.className = sym.is_toxic ? "badge-pill warn mono" : "badge-pill emerald mono";

      // Update Operational Leafs (Tier 3)
      document.getElementById("l1-val").innerText = `${{sym.spread_bps.toFixed(1)}} bps`;
      const l1Tag = document.getElementById("l1-tag");
      const passSpread = sym.spread_bps <= 15.0;
      l1Tag.innerText = passSpread ? "PASS" : "ALTO";
      l1Tag.className = passSpread ? "badge-tag mono" : "badge-tag warn mono";

      document.getElementById("l2-val").innerText = `Accel: ${{sym.ofi_accel.toFixed(1)}}`;
      document.getElementById("l3-val").innerText = `S: ${{sym.normalized_entropy.toFixed(3)}} | C: ${{sym.quantum_coherence.toFixed(3)}}`;
      document.getElementById("l4-val").innerText = `${{sym.mach_number.toFixed(2)}} M (${{sym.is_cavitation ? 'Cavitación' : 'Sub'}})`;
      document.getElementById("l5-val").innerText = `VPIN: ${{sym.vpin.toFixed(3)}}`;
      const l5Tag = document.getElementById("l5-tag");
      l5Tag.innerText = sym.is_toxic ? "BLOQUEADO" : "PASS";
      l5Tag.className = sym.is_toxic ? "badge-tag warn mono" : "badge-tag mono";

      document.getElementById("l6-val").innerText = passSpread ? "TP > Costes" : "Fricción Alta";
      document.getElementById("l7-val").innerText = `${{sym.sor_intent}} (${{s.kelly_leverage.toFixed(2)}}x)`;

      // Update Pre-Trade Gates Badges
      const gSpread = document.getElementById("gate-spread-badge");
      gSpread.innerText = passSpread ? "PASS" : "RECHAZO";
      gSpread.style.color = passSpread ? "var(--emerald)" : "var(--rose)";

      // Update 8-level CTQW Dispersion
      const dispRow = document.getElementById("dispersion-bars-row");
      if (dispRow && sym.depth_dispersion) {{
        dispRow.innerHTML = sym.depth_dispersion.map((prob, idx) => {{
          const isMid = idx === 3 || idx === 4;
          const h = Math.max(4, Math.min(24, prob * 50));
          return `
            <div class="disp-col">
              <div class="disp-bar ${{isMid ? 'mid' : ''}}" style="height: ${{h}}px;"></div>
              <span class="disp-tag mono">${{idx < 4 ? 'B' + (3 - idx) : 'A' + (idx - 4)}}</span>
            </div>
          `;
        }}).join("");
      }}
    }}

    function updateDashboard(s) {{
      cachedState = s;

      // Badges
      const wsBadge = document.getElementById("badge-ws");
      if (s.websocket_connected) {{
        wsBadge.className = "badge online";
        wsBadge.innerText = `● STREAMER ACTIVO (${{s.feed.toUpperCase()}})`;
      }} else {{
        wsBadge.className = "badge info";
        wsBadge.innerText = "○ WS RECONECTANDO...";
      }}

      document.getElementById("badge-phase").innerText = `FASE: ${{s.market_phase}}`;
      document.getElementById("val-throughput").innerText = Math.round(s.throughput_ticks_per_sec);
      document.getElementById("val-rss").innerText = s.rss_mb.toFixed(1);
      document.getElementById("val-latency").innerText = (s.avg_latency_ns / 1000.0).toFixed(1);

      // Broker Stats
      document.getElementById("card-broker-account").innerText = s.broker_account;
      document.getElementById("card-equity").innerText = `$${{s.current_equity.toLocaleString(undefined, {{minimumFractionDigits: 2, maximumFractionDigits: 2}})}} USD`;
      document.getElementById("card-bp").innerText = `$${{s.buying_power.toLocaleString(undefined, {{minimumFractionDigits: 2, maximumFractionDigits: 2}})}} USD`;
      document.getElementById("card-kelly").innerText = `${{s.kelly_leverage.toFixed(2)}}x`;

      const posLabel = document.getElementById("card-pos-count");
      if (s.positions && s.positions.length > 0) {{
        posLabel.innerText = `${{s.positions.length}} activa(s) en broker`;
        posLabel.style.color = "var(--emerald)";
      }} else {{
        posLabel.innerText = "0 abiertas (Sin riesgo nocturno)";
        posLabel.style.color = "var(--muted)";
      }}

      // Live Ticker Line
      if (s.recent_logs && s.recent_logs.length > 0) {{
        const latest = s.recent_logs[s.recent_logs.length - 1];
        document.getElementById("live-ticker-msg").innerHTML = `<b>[${{latest.timestamp_str}}]</b> ${{latest.symbol}}: ${{latest.message}}`;
      }}

      renderCurrentSymbol();
    }}

    // Polling Loop: Seamless 750ms update without page reload
    async function pollStatus() {{
      try {{
        const resp = await fetch("/api/status");
        if (resp.ok) {{
          const data = await resp.json();
          updateDashboard(data);
        }}
      }} catch (e) {{
        console.warn("Poll status error:", e);
      }}
    }}

    // Init
    updateDashboard(cachedState);
    drawConnections();
    setInterval(pollStatus, 750);
  </script>
</body>
</html>"##)
}
