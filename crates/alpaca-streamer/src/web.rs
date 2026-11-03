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
    pub status: String, // "PASS", "ACTIVE", "WARNING", "BLOCKED", "IDLE"
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

    // Rich Neural Pipeline and Microstructure Telemetry
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

/// Generates the futuristic, ultra-modern Neural Network Quantum Pipeline Dashboard.
/// Strictly fitted to 100vh without vertical scrolling, with clean typography and zero visual clutter.
pub fn render_dashboard_html(initial_state: &WebDashboardState) -> String {
    let initial_json = serde_json::to_string(initial_state).unwrap_or_else(|_| "{}".to_string());

    format!(r##"<!DOCTYPE html>
<html lang="es">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
  <title>Entropiq — Quantum Neural Microstructure Platform</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500;600;700&display=swap" rel="stylesheet">
  <style>
    :root {{
      --bg: #07090e;
      --card-bg: rgba(13, 18, 30, 0.72);
      --card-border: rgba(255, 255, 255, 0.07);
      --cyan: #00f2fe;
      --cyan-glow: rgba(0, 242, 254, 0.35);
      --emerald: #10b981;
      --emerald-glow: rgba(16, 185, 129, 0.35);
      --purple: #818cf8;
      --purple-glow: rgba(129, 140, 248, 0.35);
      --amber: #fbbf24;
      --amber-glow: rgba(251, 191, 36, 0.35);
      --rose: #f43f5e;
      --text: #f1f5f9;
      --muted: #94a3b8;
      --subtle: #475569;
    }}

    * {{
      box-sizing: border-box;
      margin: 0;
      padding: 0;
      -webkit-font-smoothing: antialiased;
      -moz-osx-font-smoothing: grayscale;
    }}

    html, body {{
      height: 100vh;
      max-height: 100vh;
      overflow: hidden;
      background: var(--bg);
      background-image:
        radial-gradient(circle at 15% 20%, rgba(99, 102, 241, 0.10) 0%, transparent 45%),
        radial-gradient(circle at 85% 80%, rgba(0, 242, 254, 0.08) 0%, transparent 40%);
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
      font-family: 'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
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
    }}

    .brand-wrap {{
      display: flex;
      align-items: center;
      gap: 10px;
    }}
    .core-orb-mini {{
      width: 22px;
      height: 22px;
      border-radius: 50%;
      background: radial-gradient(circle, var(--cyan) 20%, #6366f1 80%);
      box-shadow: 0 0 14px var(--cyan-glow);
      animation: pulseMini 2.5s infinite ease-in-out;
    }}
    @keyframes pulseMini {{
      0%, 100% {{ transform: scale(1); filter: brightness(1); }}
      50% {{ transform: scale(1.1); filter: brightness(1.3); }}
    }}
    .brand-title {{
      font-size: 14px;
      font-weight: 800;
      letter-spacing: 0.1em;
      color: #fff;
      display: flex;
      align-items: center;
      gap: 6px;
    }}
    .brand-title span {{ color: var(--cyan); }}
    .brand-sub {{
      font-size: 10px;
      font-weight: 600;
      color: var(--muted);
      letter-spacing: 0.06em;
      background: rgba(255, 255, 255, 0.05);
      padding: 2px 7px;
      border-radius: 6px;
    }}

    .top-center {{
      display: flex;
      align-items: center;
      gap: 8px;
    }}
    .asset-tab {{
      background: rgba(255, 255, 255, 0.03);
      border: 1px solid rgba(255, 255, 255, 0.06);
      color: var(--muted);
      padding: 5px 12px;
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
      border-color: rgba(0, 242, 254, 0.4);
    }}
    .asset-tab.active {{
      background: rgba(0, 242, 254, 0.12);
      border-color: var(--cyan);
      color: #fff;
      box-shadow: 0 0 10px rgba(0, 242, 254, 0.2);
    }}

    .top-right {{
      display: flex;
      align-items: center;
      gap: 10px;
    }}
    .badge {{
      display: inline-flex;
      align-items: center;
      gap: 5px;
      font-size: 10px;
      font-weight: 700;
      padding: 4px 10px;
      border-radius: 20px;
      letter-spacing: 0.04em;
    }}
    .badge.online {{ background: rgba(16, 185, 129, 0.12); color: var(--emerald); border: 1px solid rgba(16, 185, 129, 0.3); }}
    .badge.info {{ background: rgba(0, 242, 254, 0.10); color: var(--cyan); border: 1px solid rgba(0, 242, 254, 0.25); }}
    .badge.amber {{ background: rgba(251, 191, 36, 0.12); color: var(--amber); border: 1px solid rgba(251, 191, 36, 0.3); }}
    .badge.purple {{ background: rgba(129, 140, 248, 0.12); color: var(--purple); border: 1px solid rgba(129, 140, 248, 0.3); }}

    /* Main Dashboard Grid: Perfectly Fits Viewport */
    .dashboard-grid {{
      flex: 1;
      min-height: 0;
      display: grid;
      grid-template-columns: 1fr 380px;
      gap: 10px;
    }}

    /* Left Stage: The Neural Graph Canvas */
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
      height: 38px;
      min-height: 38px;
      padding: 0 16px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      border-bottom: 1px solid rgba(255, 255, 255, 0.04);
      background: rgba(0, 0, 0, 0.2);
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
    .hud-metrics span b {{
      color: #fff;
    }}

    .canvas-wrap {{
      flex: 1;
      min-height: 0;
      width: 100%;
      position: relative;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 6px;
    }}

    svg.neural-svg {{
      width: 100%;
      height: 100%;
      display: block;
    }}

    /* SVG Neural Elements */
    .synapse-base {{
      fill: none;
      stroke: rgba(255, 255, 255, 0.07);
      stroke-width: 1.5;
    }}
    .synapse-pulse {{
      fill: none;
      stroke: var(--cyan);
      stroke-width: 2.2;
      stroke-linecap: round;
      stroke-dasharray: 8 160;
      animation: synapseRun 2.8s linear infinite;
    }}
    .synapse-pulse-purple {{
      fill: none;
      stroke: var(--purple);
      stroke-width: 2.2;
      stroke-linecap: round;
      stroke-dasharray: 8 160;
      animation: synapseRun 3.2s linear infinite;
    }}
    .synapse-pulse-emerald {{
      fill: none;
      stroke: var(--emerald);
      stroke-width: 2.2;
      stroke-linecap: round;
      stroke-dasharray: 8 160;
      animation: synapseRun 2.5s linear infinite;
    }}
    @keyframes synapseRun {{
      from {{ stroke-dashoffset: 168; }}
      to {{ stroke-dashoffset: 0; }}
    }}

    .svg-node-card {{
      cursor: pointer;
      transition: all 0.2s ease;
    }}
    .svg-node-card:hover rect.node-box {{
      stroke: var(--cyan);
      fill: rgba(14, 24, 45, 0.95);
      filter: drop-shadow(0 0 8px rgba(0, 242, 254, 0.3));
    }}

    /* CTQW Dispersion Strip at Bottom of Stage */
    .dispersion-strip {{
      height: 48px;
      min-height: 48px;
      padding: 6px 16px;
      background: rgba(0, 0, 0, 0.35);
      border-top: 1px solid rgba(255, 255, 255, 0.04);
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 16px;
    }}
    .disp-info {{
      display: flex;
      flex-direction: column;
      gap: 2px;
    }}
    .disp-title {{
      font-size: 10px;
      font-weight: 700;
      letter-spacing: 0.06em;
      color: var(--muted);
      text-transform: uppercase;
    }}
    .disp-sub {{
      font-size: 9px;
      color: var(--subtle);
    }}
    .disp-bars-box {{
      display: flex;
      align-items: flex-end;
      gap: 5px;
      height: 28px;
    }}
    .disp-col {{
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 2px;
      width: 24px;
    }}
    .disp-bar {{
      width: 100%;
      background: rgba(129, 140, 248, 0.5);
      border-radius: 2px 2px 0 0;
      min-height: 3px;
      transition: height 0.3s ease;
    }}
    .disp-bar.mid {{
      background: var(--cyan);
      box-shadow: 0 0 6px var(--cyan-glow);
    }}
    .disp-tag {{
      font-size: 8px;
      font-weight: 700;
      color: var(--subtle);
    }}

    /* Right Panel: Precision Telemetry Drawer */
    .telemetry-drawer {{
      background: var(--card-bg);
      backdrop-filter: blur(20px);
      border: 1px solid var(--card-border);
      border-radius: 14px;
      display: flex;
      flex-direction: column;
      overflow: hidden;
      padding: 12px;
      gap: 9px;
    }}

    /* Active Asset Hero */
    .asset-hero {{
      background: rgba(255, 255, 255, 0.02);
      border: 1px solid rgba(255, 255, 255, 0.05);
      border-radius: 10px;
      padding: 10px 12px;
      display: flex;
      justify-content: space-between;
      align-items: center;
    }}
    .asset-id-box {{
      display: flex;
      flex-direction: column;
    }}
    .asset-symbol {{
      font-size: 16px;
      font-weight: 800;
      color: #fff;
      display: flex;
      align-items: center;
      gap: 6px;
    }}
    .asset-micro {{
      font-size: 11px;
      color: var(--muted);
      margin-top: 1px;
    }}
    .asset-price-box {{
      text-align: right;
    }}
    .asset-price {{
      font-size: 20px;
      font-weight: 800;
      color: var(--cyan);
      text-shadow: 0 0 10px rgba(0, 242, 254, 0.25);
    }}
    .asset-spread-badge {{
      display: inline-block;
      font-size: 9px;
      font-weight: 700;
      padding: 2px 6px;
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
      padding: 7px 10px;
      display: flex;
      flex-direction: column;
      gap: 2px;
    }}
    .quad-label {{
      font-size: 9px;
      font-weight: 700;
      color: var(--muted);
      letter-spacing: 0.06em;
      text-transform: uppercase;
    }}
    .quad-val {{
      font-size: 13px;
      font-weight: 700;
      color: #fff;
    }}

    /* Section Title */
    .drawer-sec-title {{
      font-size: 10px;
      font-weight: 700;
      letter-spacing: 0.07em;
      color: var(--muted);
      text-transform: uppercase;
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 0 2px;
    }}

    /* Pre-Trade Gates Compact List */
    .gates-box {{
      background: rgba(0, 0, 0, 0.25);
      border: 1px solid rgba(255, 255, 255, 0.04);
      border-radius: 10px;
      padding: 4px 8px;
      display: flex;
      flex-direction: column;
      gap: 4px;
    }}
    .gate-row {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 5px 6px;
      border-radius: 6px;
      font-size: 11px;
      transition: background 0.15s;
    }}
    .gate-row:hover {{
      background: rgba(255, 255, 255, 0.03);
    }}
    .gate-name-wrap {{
      display: flex;
      align-items: center;
      gap: 7px;
    }}
    .gate-dot {{
      width: 6px;
      height: 6px;
      border-radius: 50%;
      background: var(--emerald);
      box-shadow: 0 0 6px var(--emerald-glow);
    }}
    .gate-dot.amber {{ background: var(--amber); box-shadow: 0 0 6px var(--amber-glow); }}
    .gate-dot.purple {{ background: var(--purple); box-shadow: 0 0 6px var(--purple-glow); }}
    .gate-name {{
      font-size: 11px;
      font-weight: 600;
      color: var(--text);
    }}
    .gate-badge {{
      font-size: 9px;
      font-weight: 700;
      padding: 2px 7px;
      border-radius: 4px;
      background: rgba(16, 185, 129, 0.15);
      color: var(--emerald);
      border: 1px solid rgba(16, 185, 129, 0.3);
    }}

    /* Broker Account & Equity Tile */
    .broker-box {{
      background: rgba(0, 0, 0, 0.25);
      border: 1px solid rgba(255, 255, 255, 0.04);
      border-radius: 10px;
      padding: 8px 12px;
      display: flex;
      flex-direction: column;
      gap: 5px;
    }}
    .broker-row {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      font-size: 11px;
    }}
    .broker-label {{ color: var(--muted); }}
    .broker-val {{ font-weight: 700; color: #fff; }}

    /* Stream Ticker Bar at Bottom of Drawer */
    .ticker-bar {{
      height: 28px;
      min-height: 28px;
      background: rgba(0, 0, 0, 0.4);
      border: 1px solid rgba(255, 255, 255, 0.04);
      border-radius: 6px;
      padding: 0 10px;
      display: flex;
      align-items: center;
      gap: 8px;
      font-size: 10px;
      overflow: hidden;
      white-space: nowrap;
      text-overflow: ellipsis;
    }}
    .ticker-dot {{
      width: 6px;
      height: 6px;
      min-width: 6px;
      border-radius: 50%;
      background: var(--emerald);
      box-shadow: 0 0 6px var(--emerald-glow);
      animation: pulseMini 2s infinite ease-in-out;
    }}
    .ticker-text {{
      color: var(--muted);
      overflow: hidden;
      text-overflow: ellipsis;
    }}
    .ticker-text b {{
      color: #fff;
    }}
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
      <div class="badge info" id="badge-market" title="El mercado de criptomonedas no cierra jamás: opera 24/7/365 continuo">
        ● CRIPTO 24/7 (CONTINUO)
      </div>
      <div class="badge purple mono" id="badge-phase">FASE: THERMALNOISE</div>
    </div>
  </header>

  <!-- Main 100vh Layout Grid -->
  <main class="dashboard-grid">

    <!-- Left Stage: Interactive SVG Neural Network Canvas -->
    <section class="neural-stage">
      <div class="canvas-hud">
        <div class="hud-title">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
            <circle cx="12" cy="12" r="3"></circle>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
          </svg>
          Arquitectura de Decisión Neural Cuántica (3-Tier CTQW)
        </div>
        <div class="hud-metrics mono">
          <span>Throughput: <b id="val-throughput">0</b> ticks/s</span>
          <span>Latencia: <b id="val-latency">18.0</b> µs</span>
          <span>RAM: <b id="val-rss">8.4</b> MB</span>
        </div>
      </div>

      <div class="canvas-wrap">
        <svg class="neural-svg" viewBox="0 0 940 450" preserveAspectRatio="xMidYMid meet">
          <defs>
            <radialGradient id="grad-core" cx="50%" cy="50%" r="50%">
              <stop offset="0%" stop-color="#00f2fe" stop-opacity="1" />
              <stop offset="40%" stop-color="#3b82f6" stop-opacity="0.8" />
              <stop offset="100%" stop-color="#6366f1" stop-opacity="0.1" />
            </radialGradient>
            <radialGradient id="grad-aura" cx="50%" cy="50%" r="50%">
              <stop offset="0%" stop-color="#00f2fe" stop-opacity="0.3" />
              <stop offset="60%" stop-color="#818cf8" stop-opacity="0.1" />
              <stop offset="100%" stop-color="#07090e" stop-opacity="0" />
            </radialGradient>
            <filter id="glow-filter" x="-20%" y="-20%" width="140%" height="140%">
              <feGaussianBlur stdDeviation="5" result="blur" />
              <feComposite in="SourceGraphic" in2="blur" operator="over" />
            </filter>
          </defs>

          <!-- Synaptic Connecting Wires (Tier 1 -> Tier 2) -->
          <g class="synapse-layer-1">
            <path d="M 125 225 C 200 225, 200 115, 275 115" class="synapse-base" />
            <path d="M 125 225 C 200 225, 200 115, 275 115" class="synapse-pulse" />

            <path d="M 125 225 C 200 225, 200 225, 275 225" class="synapse-base" />
            <path d="M 125 225 C 200 225, 200 225, 275 225" class="synapse-pulse-purple" />

            <path d="M 125 225 C 200 225, 200 335, 275 335" class="synapse-base" />
            <path d="M 125 225 C 200 225, 200 335, 275 335" class="synapse-pulse-emerald" />
          </g>

          <!-- Synaptic Connecting Wires (Tier 2 -> Tier 3 Leafs) -->
          <g class="synapse-layer-2">
            <!-- Hub 1 to Leafs 1, 2 -->
            <path d="M 485 115 C 530 115, 530 85, 575 85" class="synapse-base" />
            <path d="M 485 115 C 530 115, 530 85, 575 85" class="synapse-pulse" />

            <path d="M 485 115 C 530 115, 530 135, 575 135" class="synapse-base" />
            <path d="M 485 115 C 530 115, 530 135, 575 135" class="synapse-pulse" />

            <!-- Hub 2 to Leafs 3, 4 -->
            <path d="M 485 225 C 530 225, 530 200, 575 200" class="synapse-base" />
            <path d="M 485 225 C 530 225, 530 200, 575 200" class="synapse-pulse-purple" />

            <path d="M 485 225 C 530 225, 530 250, 575 250" class="synapse-base" />
            <path d="M 485 225 C 530 225, 530 250, 575 250" class="synapse-pulse-purple" />

            <!-- Hub 3 to Leafs 5, 6, 7 -->
            <path d="M 485 335 C 530 335, 530 310, 575 310" class="synapse-base" />
            <path d="M 485 335 C 530 335, 530 310, 575 310" class="synapse-pulse-emerald" />

            <path d="M 485 335 C 530 335, 530 355, 575 355" class="synapse-base" />
            <path d="M 485 335 C 530 335, 530 355, 575 355" class="synapse-pulse-emerald" />

            <path d="M 485 335 C 530 335, 530 400, 575 400" class="synapse-base" />
            <path d="M 485 335 C 530 335, 530 400, 575 400" class="synapse-pulse-emerald" />
          </g>

          <!-- TIER 1: The Central Quantum Core Sphere -->
          <g transform="translate(85, 225)">
            <!-- Outer Pulsing Halo -->
            <circle cx="0" cy="0" r="68" fill="url(#grad-aura)" />
            <circle cx="0" cy="0" r="54" fill="none" stroke="rgba(0, 242, 254, 0.2)" stroke-width="1.5" stroke-dasharray="4 6">
              <animateTransform attributeName="transform" type="rotate" from="0" to="360" dur="20s" repeatCount="indefinite"/>
            </circle>
            <!-- Core Sphere -->
            <circle cx="0" cy="0" r="42" fill="url(#grad-core)" filter="url(#glow-filter)" />
            <circle cx="0" cy="0" r="42" fill="none" stroke="rgba(255, 255, 255, 0.5)" stroke-width="1" />
            <!-- Text inside core -->
            <text x="0" y="-8" text-anchor="middle" fill="#ffffff" font-size="10" font-weight="800" letter-spacing="1.5">QUANTUM</text>
            <text x="0" y="5" text-anchor="middle" fill="#00f2fe" font-size="8" font-weight="700">CORE RK4</text>
            <rect x="-30" y="12" width="60" height="14" rx="7" fill="rgba(0, 0, 0, 0.45)" stroke="rgba(0, 242, 254, 0.3)" stroke-width="0.8" />
            <text x="0" y="22" id="svg-core-phase" text-anchor="middle" fill="#fbbf24" font-size="7.5" font-weight="800" font-family="'JetBrains Mono', monospace">THERMAL</text>
          </g>

          <!-- TIER 2: Intermediate Processing Clusters -->
          <!-- Hub 1: Microestructura L1/L2 -->
          <g class="svg-node-card" transform="translate(275, 87)">
            <rect class="node-box" width="210" height="56" rx="10" fill="rgba(13, 20, 35, 0.88)" stroke="rgba(0, 242, 254, 0.3)" stroke-width="1.2" />
            <circle cx="18" cy="22" r="5" fill="#00f2fe" filter="url(#glow-filter)" />
            <text x="32" y="22" fill="#ffffff" font-size="11" font-weight="700">01 · INGESTIÓN & SPREAD</text>
            <text x="32" y="38" id="hub-1-desc" fill="#94a3b8" font-size="9" font-family="'JetBrains Mono', monospace">Spread: -- bps &bull; OFI: --</text>
            <rect x="150" y="10" width="50" height="16" rx="4" fill="rgba(0, 242, 254, 0.12)" stroke="rgba(0, 242, 254, 0.3)" stroke-width="0.8" />
            <text x="175" y="21" id="hub-1-status" text-anchor="middle" fill="#00f2fe" font-size="8" font-weight="800">LAMINAR</text>
          </g>

          <!-- Hub 2: Física Cuántica & Fluidos -->
          <g class="svg-node-card" transform="translate(275, 197)">
            <rect class="node-box" width="210" height="56" rx="10" fill="rgba(13, 20, 35, 0.88)" stroke="rgba(129, 140, 248, 0.3)" stroke-width="1.2" />
            <circle cx="18" cy="22" r="5" fill="#818cf8" filter="url(#glow-filter)" />
            <text x="32" y="22" fill="#ffffff" font-size="11" font-weight="700">02 · FÍSICA & MATRIZ ρ</text>
            <text x="32" y="38" id="hub-2-desc" fill="#94a3b8" font-size="9" font-family="'JetBrains Mono', monospace">S(norm): -- &bull; Coher: --</text>
            <rect x="145" y="10" width="55" height="16" rx="4" fill="rgba(129, 140, 248, 0.12)" stroke="rgba(129, 140, 248, 0.3)" stroke-width="0.8" />
            <text x="172" y="21" id="hub-2-status" text-anchor="middle" fill="#818cf8" font-size="8" font-weight="800">COHERENTE</text>
          </g>

          <!-- Hub 3: Valuación Pre-Trade & SOR -->
          <g class="svg-node-card" transform="translate(275, 307)">
            <rect class="node-box" width="210" height="56" rx="10" fill="rgba(13, 20, 35, 0.88)" stroke="rgba(16, 185, 129, 0.3)" stroke-width="1.2" />
            <circle cx="18" cy="22" r="5" fill="#10b981" filter="url(#glow-filter)" />
            <text x="32" y="22" fill="#ffffff" font-size="11" font-weight="700">03 · VALUACIÓN & SOR</text>
            <text x="32" y="38" id="hub-3-desc" fill="#94a3b8" font-size="9" font-family="'JetBrains Mono', monospace">6 Gates &bull; Half-Kelly 1.0x</text>
            <rect x="148" y="10" width="52" height="16" rx="4" fill="rgba(16, 185, 129, 0.12)" stroke="rgba(16, 185, 129, 0.3)" stroke-width="0.8" />
            <text x="174" y="21" id="hub-3-status" text-anchor="middle" fill="#10b981" font-size="8" font-weight="800">PERMISIVO</text>
          </g>

          <!-- TIER 3: Operational Micro-Nodes (Leaf Fan-Out) -->
          <!-- Leaf 1: Spread Bid/Ask -->
          <g class="svg-node-card" transform="translate(575, 68)">
            <rect class="node-box" width="315" height="34" rx="7" fill="rgba(10, 16, 28, 0.85)" stroke="rgba(255, 255, 255, 0.08)" stroke-width="1" />
            <text x="14" y="21" fill="#e2e8f0" font-size="10.5" font-weight="600">Spread Bid-Ask Dinámico</text>
            <text x="210" y="21" id="leaf-1-val" fill="#00f2fe" font-size="9.5" font-family="'JetBrains Mono', monospace">2.5 bps</text>
            <circle cx="295" cy="17" r="3.5" fill="#10b981" />
          </g>

          <!-- Leaf 2: Order Flow Imbalance (OFI) -->
          <g class="svg-node-card" transform="translate(575, 118)">
            <rect class="node-box" width="315" height="34" rx="7" fill="rgba(10, 16, 28, 0.85)" stroke="rgba(255, 255, 255, 0.08)" stroke-width="1" />
            <text x="14" y="21" fill="#e2e8f0" font-size="10.5" font-weight="600">Order Flow Imbalance (OFI)</text>
            <text x="210" y="21" id="leaf-2-val" fill="#94a3b8" font-size="9.5" font-family="'JetBrains Mono', monospace">Accel: 0.0</text>
            <circle cx="295" cy="17" r="3.5" fill="#10b981" />
          </g>

          <!-- Leaf 3: Matriz Densidad von Neumann -->
          <g class="svg-node-card" transform="translate(575, 183)">
            <rect class="node-box" width="315" height="34" rx="7" fill="rgba(10, 16, 28, 0.85)" stroke="rgba(255, 255, 255, 0.08)" stroke-width="1" />
            <text x="14" y="21" fill="#e2e8f0" font-size="10.5" font-weight="600">Operador Densidad von Neumann (ρ)</text>
            <text x="210" y="21" id="leaf-3-val" fill="#818cf8" font-size="9.5" font-family="'JetBrains Mono', monospace">S: 0.81 | C: 0.25</text>
            <circle cx="295" cy="17" r="3.5" fill="#818cf8" />
          </g>

          <!-- Leaf 4: Régimen Fluídico & Mach -->
          <g class="svg-node-card" transform="translate(575, 233)">
            <rect class="node-box" width="315" height="34" rx="7" fill="rgba(10, 16, 28, 0.85)" stroke="rgba(255, 255, 255, 0.08)" stroke-width="1" />
            <text x="14" y="21" fill="#e2e8f0" font-size="10.5" font-weight="600">Hidrodinámica & Mach Shockwave</text>
            <text x="210" y="21" id="leaf-4-val" fill="#00f2fe" font-size="9.5" font-family="'JetBrains Mono', monospace">Mach 0.00 (Sub)</text>
            <circle cx="295" cy="17" r="3.5" fill="#00f2fe" />
          </g>

          <!-- Leaf 5: Escudo Tóxico VPIN -->
          <g class="svg-node-card" transform="translate(575, 293)">
            <rect class="node-box" width="315" height="34" rx="7" fill="rgba(10, 16, 28, 0.85)" stroke="rgba(255, 255, 255, 0.08)" stroke-width="1" />
            <text x="14" y="21" fill="#e2e8f0" font-size="10.5" font-weight="600">Escudo Toxicidad Flujo (VPIN)</text>
            <text x="210" y="21" id="leaf-5-val" fill="#10b981" font-size="9.5" font-family="'JetBrains Mono', monospace">VPIN: 0.000</text>
            <circle cx="295" cy="17" r="3.5" fill="#10b981" />
          </g>

          <!-- Leaf 6: Margen de Retorno vs Fricción -->
          <g class="svg-node-card" transform="translate(575, 338)">
            <rect class="node-box" width="315" height="34" rx="7" fill="rgba(10, 16, 28, 0.85)" stroke="rgba(255, 255, 255, 0.08)" stroke-width="1" />
            <text x="14" y="21" fill="#e2e8f0" font-size="10.5" font-weight="600">Margen Retorno vs Fricción Fee</text>
            <text x="210" y="21" id="leaf-6-val" fill="#fbbf24" font-size="9.5" font-family="'JetBrains Mono', monospace">TP 45bps &gt; Fee</text>
            <circle cx="295" cy="17" r="3.5" fill="#10b981" />
          </g>

          <!-- Leaf 7: SOR Router & Alpaca Execution -->
          <g class="svg-node-card" transform="translate(575, 383)">
            <rect class="node-box" width="315" height="34" rx="7" fill="rgba(10, 16, 28, 0.85)" stroke="rgba(255, 255, 255, 0.08)" stroke-width="1" />
            <text x="14" y="21" fill="#e2e8f0" font-size="10.5" font-weight="600">Enrutador SOR & Límite Alpaca REST</text>
            <text x="210" y="21" id="leaf-7-val" fill="#a78bfa" font-size="9.5" font-family="'JetBrains Mono', monospace">Hold &bull; Kelly: 0.2x</text>
            <circle cx="295" cy="17" r="3.5" fill="#a78bfa" />
          </g>
        </svg>
      </div>

      <!-- Integrated CTQW Dispersion Strip -->
      <div class="dispersion-strip">
        <div class="disp-info">
          <span class="disp-title">Dispersión de Paquete de Ondas Cuántico CTQW</span>
          <span class="disp-sub">Distribución probabilística de penetración en 8 niveles de profundidad del libro</span>
        </div>
        <div class="disp-bars-box" id="dispersion-bars-row">
          <!-- Injected via JS -->
        </div>
      </div>
    </section>

    <!-- Right Column: Precision Intelligence Panel (Fits 100vh) -->
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

      <!-- Compact Live Ticker Event Bar -->
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
      document.getElementById("svg-core-phase").innerText = sym.active_phase.toUpperCase();

      // Physics Quad
      document.getElementById("quad-entropy").innerText = sym.normalized_entropy.toFixed(3);
      document.getElementById("quad-coherence").innerText = sym.quantum_coherence.toFixed(3);
      document.getElementById("quad-mach").innerText = `${{sym.mach_number.toFixed(2)}} M`;
      document.getElementById("quad-vpin").innerText = sym.vpin.toFixed(3);

      // Update SVG Intermediate Hubs
      document.getElementById("hub-1-desc").innerText = `Spread: ${{sym.spread_bps.toFixed(1)}} bps &bull; Profundidad: OK`;
      const h1Status = document.getElementById("hub-1-status");
      h1Status.innerText = sym.spread_bps <= 15.0 ? "LAMINAR" : "ALTO SPREAD";
      h1Status.style.color = sym.spread_bps <= 15.0 ? "var(--cyan)" : "var(--amber)";

      document.getElementById("hub-2-desc").innerText = `S(norm): ${{sym.normalized_entropy.toFixed(3)}} &bull; Coher: ${{sym.quantum_coherence.toFixed(3)}}`;
      const h2Status = document.getElementById("hub-2-status");
      h2Status.innerText = sym.quantum_coherence >= 0.25 ? "COHERENTE" : "DECOHERIDO";
      h2Status.style.color = sym.quantum_coherence >= 0.25 ? "var(--purple)" : "var(--muted)";

      document.getElementById("hub-3-desc").innerText = `VPIN: ${{sym.vpin.toFixed(3)}} &bull; Kelly: ${{s.kelly_leverage.toFixed(2)}}x`;
      const h3Status = document.getElementById("hub-3-status");
      h3Status.innerText = sym.is_toxic ? "ESCUDO BLOQUEO" : "PERMISIVO";
      h3Status.style.color = sym.is_toxic ? "var(--rose)" : "var(--emerald)";

      // Update SVG Micro-Nodes (Leafs)
      document.getElementById("leaf-1-val").innerText = `${{sym.spread_bps.toFixed(1)}} bps (Bid: $${{sym.best_bid.toFixed(2)}})`;
      document.getElementById("leaf-2-val").innerText = `Accel: ${{sym.ofi_accel.toFixed(1)}}`;
      document.getElementById("leaf-3-val").innerText = `S: ${{sym.normalized_entropy.toFixed(3)}} | C: ${{sym.quantum_coherence.toFixed(3)}}`;
      document.getElementById("leaf-4-val").innerText = `Mach ${{sym.mach_number.toFixed(2)}} (${{sym.is_cavitation ? 'Cavitación' : 'Sub'}})`
      document.getElementById("leaf-5-val").innerText = `VPIN: ${{sym.vpin.toFixed(3)}} ${{sym.is_toxic ? '[ALTO]' : '[OK]'}}`;
      document.getElementById("leaf-6-val").innerText = sym.spread_bps <= 15.0 ? "Margen > 10bps (PASS)" : "Fricción Alta";
      document.getElementById("leaf-7-val").innerText = `${{sym.sor_intent}} &bull; Kelly ${{s.kelly_leverage.toFixed(2)}}x`;

      // Update Pre-Trade Gates Badges
      const passSpread = sym.spread_bps <= 15.0;
      const gSpread = document.getElementById("gate-spread-badge");
      gSpread.innerText = passSpread ? "PASS" : "RECHAZO";
      gSpread.style.color = passSpread ? "var(--emerald)" : "var(--rose)";

      // Update 8-level CTQW Dispersion
      const dispRow = document.getElementById("dispersion-bars-row");
      if (dispRow && sym.depth_dispersion) {{
        dispRow.innerHTML = sym.depth_dispersion.map((prob, idx) => {{
          const isMid = idx === 3 || idx === 4;
          const h = Math.max(4, Math.min(26, prob * 55));
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
        wsBadge.className = "badge amber";
        wsBadge.innerText = "○ WS CONECTANDO...";
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
        posLabel.innerText = `${{s.positions.length}} abierta(s) en broker`;
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

    updateDashboard(cachedState);
    setInterval(pollStatus, 750);
  </script>
</body>
</html>"##)
}
