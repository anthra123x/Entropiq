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
            market_open: false,
            feed: "crypto".to_string(),
            broker_account: "Sincronizando Alpaca...".to_string(),
            buying_power: 100_000.0,
            current_equity: 100_000.0,
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

/// Generates the futuristic Neural Network Quantum Pipeline Dashboard.
pub fn render_dashboard_html(initial_state: &WebDashboardState) -> String {
    let initial_json = serde_json::to_string(initial_state).unwrap_or_else(|_| "{}".to_string());

    format!(r##"<!DOCTYPE html>
<html lang="es">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Entropiq — Quantum Neural Microstructure Platform</title>
  <style>
    :root {{
      --bg: #070b13;
      --card-bg: rgba(14, 21, 37, 0.75);
      --card-border: rgba(255, 255, 255, 0.08);
      --cyan: #00f0ff;
      --cyan-glow: rgba(0, 240, 255, 0.35);
      --emerald: #10b981;
      --emerald-glow: rgba(16, 185, 129, 0.35);
      --purple: #8b5cf6;
      --purple-glow: rgba(139, 92, 246, 0.35);
      --amber: #f59e0b;
      --amber-glow: rgba(245, 158, 11, 0.35);
      --rose: #f43f5e;
      --text: #f8fafc;
      --muted: #94a3b8;
      --subtle: #475569;
    }}

    * {{ box-sizing: border-box; margin: 0; padding: 0; font-family: ui-monospace, SFMono-Regular, "Geist Mono", "Roboto Mono", Menlo, monospace; }}
    body {{
      background: var(--bg);
      background-image: 
        radial-gradient(circle at 18% 25%, rgba(139, 92, 246, 0.12) 0%, transparent 45%),
        radial-gradient(circle at 75% 75%, rgba(0, 240, 255, 0.08) 0%, transparent 45%);
      color: var(--text);
      min-height: 100vh;
      overflow-x: hidden;
      padding: 16px;
    }}

    /* Top Navigation Bar */
    .top-bar {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 14px 22px;
      background: rgba(14, 21, 37, 0.85);
      backdrop-filter: blur(16px);
      border: 1px solid var(--card-border);
      border-radius: 16px;
      margin-bottom: 16px;
      box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
    }}
    .brand-wrap {{ display: flex; align-items: center; gap: 14px; }}
    .core-orb-mini {{
      width: 28px; height: 28px; border-radius: 50%;
      background: radial-gradient(circle, var(--cyan) 20%, var(--purple) 80%);
      box-shadow: 0 0 16px var(--cyan-glow);
      animation: pulseMini 2s infinite ease-in-out;
    }}
    @keyframes pulseMini {{
      0%, 100% {{ transform: scale(1); filter: brightness(1); }}
      50% {{ transform: scale(1.12); filter: brightness(1.3); }}
    }}
    .brand-title {{ font-size: 16px; font-weight: 800; letter-spacing: 0.12em; color: #fff; display: flex; align-items: center; gap: 8px; }}
    .brand-title span {{ color: var(--cyan); }}
    .brand-sub {{ font-size: 11px; color: var(--muted); letter-spacing: 0.05em; }}

    .nav-stats {{ display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }}
    .badge {{
      display: inline-flex; align-items: center; gap: 6px;
      font-size: 11px; font-weight: 700; padding: 5px 12px;
      border-radius: 20px; text-transform: uppercase; letter-spacing: 0.06em;
    }}
    .badge.online {{ background: rgba(16, 185, 129, 0.15); color: var(--emerald); border: 1px solid rgba(16, 185, 129, 0.4); }}
    .badge.offline {{ background: rgba(244, 63, 94, 0.15); color: var(--rose); border: 1px solid rgba(244, 63, 94, 0.4); }}
    .badge.info {{ background: rgba(0, 240, 255, 0.12); color: var(--cyan); border: 1px solid rgba(0, 240, 255, 0.3); }}
    .badge.amber {{ background: rgba(245, 158, 11, 0.15); color: var(--amber); border: 1px solid rgba(245, 158, 11, 0.4); }}

    /* Layout Split */
    .dashboard-grid {{
      display: grid;
      grid-template-columns: 1fr 440px;
      gap: 16px;
      min-height: calc(100vh - 110px);
    }}
    @media (max-width: 1180px) {{
      .dashboard-grid {{ grid-template-columns: 1fr; }}
    }}

    /* Left Stage: Neural Network Graph */
    .neural-stage {{
      background: var(--card-bg);
      backdrop-filter: blur(20px);
      border: 1px solid var(--card-border);
      border-radius: 20px;
      padding: 20px;
      display: flex;
      flex-direction: column;
      position: relative;
      box-shadow: 0 12px 40px rgba(0,0,0,0.5);
    }}
    .stage-header {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 12px;
    }}
    .asset-tabs {{ display: flex; gap: 8px; }}
    .asset-tab {{
      background: rgba(255, 255, 255, 0.04);
      border: 1px solid rgba(255, 255, 255, 0.08);
      color: var(--muted);
      padding: 6px 14px;
      border-radius: 10px;
      font-size: 12px;
      font-weight: 700;
      cursor: pointer;
      transition: all 0.2s;
    }}
    .asset-tab:hover {{ color: #fff; border-color: var(--cyan); }}
    .asset-tab.active {{
      background: rgba(0, 240, 255, 0.15);
      border-color: var(--cyan);
      color: #fff;
      box-shadow: 0 0 12px var(--cyan-glow);
    }}

    /* SVG Canvas Neural Container */
    .canvas-wrap {{
      flex: 1;
      width: 100%;
      min-height: 480px;
      display: flex;
      align-items: center;
      justify-content: center;
      position: relative;
    }}
    svg.neural-svg {{
      width: 100%;
      height: 100%;
      min-height: 480px;
      filter: drop-shadow(0 0 10px rgba(0,0,0,0.4));
    }}

    /* Synaptic wire animations */
    .synapse-wire {{
      fill: none;
      stroke: rgba(255, 255, 255, 0.08);
      stroke-width: 1.5;
    }}
    .synapse-pulse {{
      fill: none;
      stroke: var(--cyan);
      stroke-width: 2.5;
      stroke-linecap: round;
      stroke-dasharray: 10 180;
      animation: pulseRun 2.5s linear infinite;
    }}
    @keyframes pulseRun {{
      from {{ stroke-dashoffset: 190; }}
      to {{ stroke-dashoffset: 0; }}
    }}

    /* Node Cards inside SVG */
    .svg-node {{ cursor: pointer; transition: all 0.3s; }}
    .svg-node:hover rect {{ stroke: var(--cyan); filter: drop-shadow(0 0 8px var(--cyan-glow)); }}

    /* Bottom Dispersion Chart */
    .dispersion-wrap {{
      margin-top: 14px;
      background: rgba(7, 11, 19, 0.85);
      border: 1px solid rgba(255, 255, 255, 0.06);
      border-radius: 12px;
      padding: 12px 16px;
    }}
    .dispersion-title {{ font-size: 11px; text-transform: uppercase; color: var(--muted); margin-bottom: 8px; display: flex; justify-content: space-between; }}
    .dispersion-bars {{ display: flex; gap: 6px; height: 36px; align-items: flex-end; }}
    .disp-bar-col {{ flex: 1; display: flex; flex-direction: column; align-items: center; gap: 3px; }}
    .disp-bar {{ width: 100%; background: var(--purple); border-radius: 3px 3px 0 0; transition: height 0.3s ease; min-height: 3px; }}
    .disp-bar.mid {{ background: var(--cyan); box-shadow: 0 0 8px var(--cyan-glow); }}
    .disp-bar-label {{ font-size: 9px; color: var(--subtle); }}

    /* Right Inspection Drawer */
    .telemetry-drawer {{
      background: var(--card-bg);
      backdrop-filter: blur(20px);
      border: 1px solid var(--card-border);
      border-radius: 20px;
      padding: 20px;
      display: flex;
      flex-direction: column;
      gap: 16px;
      box-shadow: 0 12px 40px rgba(0,0,0,0.5);
      overflow-y: auto;
      max-height: calc(100vh - 110px);
    }}
    .drawer-header {{
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
      border-bottom: 1px solid rgba(255, 255, 255, 0.08);
      padding-bottom: 14px;
    }}
    .symbol-hero {{ font-size: 20px; font-weight: 800; color: #fff; display: flex; align-items: baseline; gap: 8px; }}
    .symbol-price {{ font-size: 18px; font-weight: 700; color: var(--cyan); }}
    .symbol-micro {{ font-size: 11px; color: var(--muted); }}

    /* Pipeline Step Stepper */
    .section-title {{ font-size: 12px; font-weight: 800; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); display: flex; align-items: center; justify-content: space-between; }}
    .pipeline-stepper {{ display: flex; flex-direction: column; gap: 8px; }}
    .step-item {{
      background: rgba(7, 11, 19, 0.7);
      border: 1px solid rgba(255, 255, 255, 0.05);
      border-radius: 10px;
      padding: 10px 14px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      transition: all 0.2s;
    }}
    .step-item:hover {{ border-color: rgba(255, 255, 255, 0.15); }}
    .step-left {{ display: flex; align-items: center; gap: 10px; }}
    .step-num {{
      width: 22px; height: 22px; border-radius: 50%;
      background: rgba(255, 255, 255, 0.06);
      font-size: 11px; font-weight: 800;
      display: flex; align-items: center; justify-content: center;
      color: var(--muted);
    }}
    .step-num.pass {{ background: rgba(16, 185, 129, 0.2); color: var(--emerald); }}
    .step-num.active {{ background: rgba(0, 240, 255, 0.2); color: var(--cyan); box-shadow: 0 0 8px var(--cyan-glow); }}
    .step-num.blocked {{ background: rgba(244, 63, 94, 0.2); color: var(--rose); }}
    .step-name {{ font-size: 12px; font-weight: 700; color: #fff; }}
    .step-desc {{ font-size: 10px; color: var(--muted); }}
    .step-val {{ font-size: 11px; font-weight: 700; font-variant-numeric: tabular-nums; }}

    /* Live Broker Card */
    .broker-grid {{
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 8px;
    }}
    .mini-stat {{
      background: rgba(7, 11, 19, 0.6);
      border: 1px solid rgba(255,255,255,0.04);
      border-radius: 8px;
      padding: 8px 10px;
    }}
    .mini-label {{ font-size: 10px; color: var(--muted); text-transform: uppercase; }}
    .mini-val {{ font-size: 13px; font-weight: 700; color: #fff; margin-top: 2px; }}

    /* Positions Table */
    .positions-wrap {{
      background: rgba(7, 11, 19, 0.6);
      border: 1px solid rgba(255,255,255,0.05);
      border-radius: 10px;
      padding: 10px;
      font-size: 11px;
    }}
    .pos-item {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 6px 0;
      border-bottom: 1px solid rgba(255,255,255,0.04);
    }}
    .pos-item:last-child {{ border-bottom: none; }}
    .empty-state {{ color: var(--subtle); font-size: 11px; text-align: center; padding: 12px; }}

    /* Event Stream Terminal */
    .terminal-box {{
      background: #04060a;
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 10px;
      padding: 10px 12px;
      font-size: 10px;
      max-height: 140px;
      overflow-y: auto;
      line-height: 1.4;
    }}
    .term-line {{ margin-bottom: 4px; display: flex; gap: 8px; }}
    .term-time {{ color: var(--subtle); }}
    .term-sym {{ color: var(--cyan); font-weight: 700; }}
    .term-msg {{ color: #cbd5e1; }}
    .term-trade {{ color: var(--emerald); font-weight: 700; }}

    /* Floating control */
    .pause-btn {{
      background: transparent;
      border: 1px solid rgba(255,255,255,0.1);
      color: var(--muted);
      border-radius: 8px;
      padding: 4px 10px;
      font-size: 11px;
      cursor: pointer;
    }}
    .pause-btn:hover {{ border-color: var(--cyan); color: #fff; }}
    .pause-btn.active {{ background: rgba(0, 240, 255, 0.15); color: var(--cyan); border-color: var(--cyan); }}
  </style>
</head>
<body>
  <!-- Top Navigation Bar -->
  <header class="top-bar">
    <div class="brand-wrap">
      <div class="core-orb-mini"></div>
      <div>
        <div class="brand-title">ENTROPIQ <span>QUANTUM NEURAL</span></div>
        <div class="brand-sub">HFT Open Quantum Systems &bull; Lead-Lag Macro Microstructure</div>
      </div>
    </div>

    <div class="nav-stats">
      <span id="badge-ws" class="badge online">● STREAMER CONECTADO</span>
      <span id="badge-market" class="badge info">● CRIPTO 24/7/365</span>
      <span id="badge-phase" class="badge amber">FASE: ThermalNoise</span>
      <button id="btn-pause" class="pause-btn" onclick="togglePause()">PAUSAR FLUJO</button>
    </div>
  </header>

  <!-- Main Split Layout -->
  <div class="dashboard-grid">
    <!-- Left Stage: Interactive Neural Graph -->
    <div class="neural-stage">
      <div class="stage-header">
        <div class="asset-tabs" id="asset-tabs-bar">
          <!-- Dynamic Asset Tabs -->
        </div>
        <div style="font-size: 11px; color: var(--muted);">
          Throughput: <strong id="val-throughput" style="color: var(--cyan);">0</strong> ticks/s &bull;
          Memoria: <strong id="val-rss" style="color: #fff;">0.00</strong> MB
        </div>
      </div>

      <!-- Neural Network Canvas / SVG Graph -->
      <div class="canvas-wrap">
        <svg class="neural-svg" viewBox="0 0 840 500" preserveAspectRatio="xMidYMid meet">
          <defs>
            <radialGradient id="core-glow" cx="50%" cy="50%" r="50%">
              <stop offset="0%" stop-color="#00f0ff" stop-opacity="0.9" />
              <stop offset="45%" stop-color="#8b5cf6" stop-opacity="0.6" />
              <stop offset="80%" stop-color="#090d16" stop-opacity="0.9" />
              <stop offset="100%" stop-color="#090d16" stop-opacity="0" />
            </radialGradient>
            <linearGradient id="cyan-wire" x1="0%" y1="0%" x2="100%" y2="0%">
              <stop offset="0%" stop-color="#8b5cf6" stop-opacity="0.8" />
              <stop offset="100%" stop-color="#00f0ff" stop-opacity="0.9" />
            </linearGradient>
            <filter id="glow-filter" x="-20%" y="-20%" width="140%" height="140%">
              <feGaussianBlur stdDeviation="4" result="blur" />
              <feComposite in="SourceGraphic" in2="blur" operator="over" />
            </filter>
          </defs>

          <!-- Synaptic Connecting Curves -->
          <!-- 1. Core -> L1/L2 Book Depth -->
          <path class="synapse-wire" d="M 195 250 C 330 250, 390 55, 520 55" />
          <path class="synapse-pulse" d="M 195 250 C 330 250, 390 55, 520 55" />

          <!-- 2. Core -> Quantum Lindblad Matrix -->
          <path class="synapse-wire" d="M 195 250 C 330 250, 390 120, 520 120" />
          <path class="synapse-pulse" style="animation-delay: -0.4s;" d="M 195 250 C 330 250, 390 120, 520 120" />

          <!-- 3. Core -> Fluid Acoustics & Mach Shock -->
          <path class="synapse-wire" d="M 195 250 C 330 250, 390 185, 520 185" />
          <path class="synapse-pulse" style="animation-delay: -0.8s;" d="M 195 250 C 330 250, 390 185, 520 185" />

          <!-- 4. Core -> Toxic Flow Shield (VPIN) -->
          <path class="synapse-wire" d="M 195 250 C 330 250, 390 250, 520 250" />
          <path class="synapse-pulse" style="animation-delay: -1.2s;" d="M 195 250 C 330 250, 390 250, 520 250" />

          <!-- 5. Core -> 6-Stage Pre-Trade Pipeline -->
          <path class="synapse-wire" d="M 195 250 C 330 250, 390 315, 520 315" />
          <path class="synapse-pulse" style="animation-delay: -1.6s;" d="M 195 250 C 330 250, 390 315, 520 315" />

          <!-- 6. Core -> Smart Order Router (SOR) & Kelly -->
          <path class="synapse-wire" d="M 195 250 C 330 250, 390 380, 520 380" />
          <path class="synapse-pulse" style="animation-delay: -2.0s;" d="M 195 250 C 330 250, 390 380, 520 380" />

          <!-- 7. Core -> Alpaca Broker Execution -->
          <path class="synapse-wire" d="M 195 250 C 330 250, 390 445, 520 445" />
          <path class="synapse-pulse" style="animation-delay: -2.4s;" d="M 195 250 C 330 250, 390 445, 520 445" />

          <!-- Central Quantum Core Orb -->
          <g transform="translate(195, 250)">
            <!-- Outer Pulsing Glow Rings -->
            <circle r="88" fill="none" stroke="rgba(139, 92, 246, 0.15)" stroke-width="1.5" stroke-dasharray="3 6" />
            <circle r="72" fill="none" stroke="rgba(0, 240, 255, 0.25)" stroke-width="2" />
            <circle r="56" fill="url(#core-glow)" filter="url(#glow-filter)" />
            <circle r="38" fill="#0d1424" stroke="var(--cyan)" stroke-width="2" />
            <!-- Core Labels -->
            <text text-anchor="middle" y="-4" fill="#ffffff" font-size="10" font-weight="800" letter-spacing="1">QUANTUM</text>
            <text text-anchor="middle" y="9" fill="var(--cyan)" font-size="9" font-weight="700">CORE RK4</text>
            <text id="svg-core-phase" text-anchor="middle" y="24" fill="var(--amber)" font-size="8" font-weight="700">THERMAL</text>
          </g>

          <!-- 7 Pipeline Neural Nodes on Right Column -->
          <!-- Node 1: Ingestion & L2 Book Depth -->
          <g class="svg-node" transform="translate(520, 30)" onclick="selectStage(1)">
            <rect width="280" height="48" rx="8" fill="rgba(14, 21, 37, 0.9)" stroke="rgba(255,255,255,0.08)" stroke-width="1" />
            <circle cx="20" cy="24" r="6" fill="var(--cyan)" />
            <text x="36" y="20" fill="#fff" font-size="11" font-weight="700">01 &bull; L1/L2 NBBO &amp; SPREAD</text>
            <text id="node-1-val" x="36" y="36" fill="var(--muted)" font-size="9">Spread: -- bps &bull; Profundidad: OK</text>
            <rect x="220" y="14" width="50" height="20" rx="10" fill="rgba(16,185,129,0.15)" stroke="rgba(16,185,129,0.4)" />
            <text id="node-1-status" x="245" y="27" text-anchor="middle" fill="var(--emerald)" font-size="8" font-weight="800">LAMINAR</text>
          </g>

          <!-- Node 2: Lindblad Density Matrix ρ -->
          <g class="svg-node" transform="translate(520, 95)" onclick="selectStage(2)">
            <rect width="280" height="48" rx="8" fill="rgba(14, 21, 37, 0.9)" stroke="rgba(255,255,255,0.08)" stroke-width="1" />
            <circle cx="20" cy="24" r="6" fill="var(--purple)" />
            <text x="36" y="20" fill="#fff" font-size="11" font-weight="700">02 &bull; MATRIZ DENSIDAD ρ (RK4)</text>
            <text id="node-2-val" x="36" y="36" fill="var(--muted)" font-size="9">S(norm): -- &bull; Coherencia C: --</text>
            <rect x="220" y="14" width="50" height="20" rx="10" fill="rgba(139,92,246,0.15)" stroke="rgba(139,92,246,0.4)" />
            <text id="node-2-status" x="245" y="27" text-anchor="middle" fill="var(--purple)" font-size="8" font-weight="800">COHERENTE</text>
          </g>

          <!-- Node 3: Fluid Acoustics & Mach Shock -->
          <g class="svg-node" transform="translate(520, 160)" onclick="selectStage(3)">
            <rect width="280" height="48" rx="8" fill="rgba(14, 21, 37, 0.9)" stroke="rgba(255,255,255,0.08)" stroke-width="1" />
            <circle cx="20" cy="24" r="6" fill="var(--cyan)" />
            <text x="36" y="20" fill="#fff" font-size="11" font-weight="700">03 &bull; HIDRODINÁMICA &amp; MACH</text>
            <text id="node-3-val" x="36" y="36" fill="var(--muted)" font-size="9">Mach: 0.00 &bull; Cavitación: No</text>
            <rect x="220" y="14" width="50" height="20" rx="10" fill="rgba(0,240,255,0.15)" stroke="rgba(0,240,255,0.4)" />
            <text id="node-3-status" x="245" y="27" text-anchor="middle" fill="var(--cyan)" font-size="8" font-weight="800">SUBSÓNICO</text>
          </g>

          <!-- Node 4: Toxic Flow Shield (VPIN) -->
          <g class="svg-node" transform="translate(520, 225)" onclick="selectStage(4)">
            <rect width="280" height="48" rx="8" fill="rgba(14, 21, 37, 0.9)" stroke="rgba(255,255,255,0.08)" stroke-width="1" />
            <circle cx="20" cy="24" r="6" fill="var(--emerald)" />
            <text x="36" y="20" fill="#fff" font-size="11" font-weight="700">04 &bull; ESCUDO TOXICIDAD (VPIN)</text>
            <text id="node-4-val" x="36" y="36" fill="var(--muted)" font-size="9">VPIN: 0.000 &bull; Umbral: &lt; 0.70</text>
            <rect x="220" y="14" width="50" height="20" rx="10" fill="rgba(16,185,129,0.15)" stroke="rgba(16,185,129,0.4)" />
            <text id="node-4-status" x="245" y="27" text-anchor="middle" fill="var(--emerald)" font-size="8" font-weight="800">PERMISIVO</text>
          </g>

          <!-- Node 5: 6-Stage Pre-Trade Pipeline -->
          <g class="svg-node" transform="translate(520, 290)" onclick="selectStage(5)">
            <rect width="280" height="48" rx="8" fill="rgba(14, 21, 37, 0.9)" stroke="rgba(255,255,255,0.08)" stroke-width="1" />
            <circle cx="20" cy="24" r="6" fill="var(--amber)" />
            <text x="36" y="20" fill="#fff" font-size="11" font-weight="700">05 &bull; PIPELINE PRE-TRADE</text>
            <text id="node-5-val" x="36" y="36" fill="var(--muted)" font-size="9">Retorno &gt; Coste &bull; Delay WS &lt; Max</text>
            <rect x="220" y="14" width="50" height="20" rx="10" fill="rgba(245,158,11,0.15)" stroke="rgba(245,158,11,0.4)" />
            <text id="node-5-status" x="245" y="27" text-anchor="middle" fill="var(--amber)" font-size="8" font-weight="800">FILTRANDO</text>
          </g>

          <!-- Node 6: SOR Router & Half-Kelly Sizing -->
          <g class="svg-node" transform="translate(520, 355)" onclick="selectStage(6)">
            <rect width="280" height="48" rx="8" fill="rgba(14, 21, 37, 0.9)" stroke="rgba(255,255,255,0.08)" stroke-width="1" />
            <circle cx="20" cy="24" r="6" fill="var(--purple)" />
            <text x="36" y="20" fill="#fff" font-size="11" font-weight="700">06 &bull; ROUTER SOR &amp; HALF-KELLY</text>
            <text id="node-6-val" x="36" y="36" fill="var(--muted)" font-size="9">Intent: NeutralHold &bull; Kelly: 1.0x</text>
            <rect x="220" y="14" width="50" height="20" rx="10" fill="rgba(139,92,246,0.15)" stroke="rgba(139,92,246,0.4)" />
            <text id="node-6-status" x="245" y="27" text-anchor="middle" fill="var(--purple)" font-size="8" font-weight="800">DECISIÓN</text>
          </g>

          <!-- Node 7: Alpaca Broker Execution -->
          <g class="svg-node" transform="translate(520, 420)" onclick="selectStage(7)">
            <rect width="280" height="48" rx="8" fill="rgba(14, 21, 37, 0.9)" stroke="rgba(255,255,255,0.08)" stroke-width="1" />
            <circle cx="20" cy="24" r="6" fill="var(--cyan)" />
            <text x="36" y="20" fill="#fff" font-size="11" font-weight="700">07 &bull; EJECUCIÓN BROKER ALPACA</text>
            <text id="node-7-val" x="36" y="36" fill="var(--muted)" font-size="9">Marketable Limit GTC &bull; Slippage Cap</text>
            <rect x="220" y="14" width="50" height="20" rx="10" fill="rgba(0,240,255,0.15)" stroke="rgba(0,240,255,0.4)" />
            <text id="node-7-status" x="245" y="27" text-anchor="middle" fill="var(--cyan)" font-size="8" font-weight="800">LISTO</text>
          </g>
        </svg>
      </div>

      <!-- Wavepacket CTQW Depth Dispersion Chart -->
      <div class="dispersion-wrap">
        <div class="dispersion-title">
          <span>Dispersión de Paquete de Ondas L2 (Caminata Cuántica CTQW 8 Niveles)</span>
          <span id="disp-variance" style="color: var(--cyan);">Var: 0.00</span>
        </div>
        <div class="dispersion-bars" id="dispersion-bars-row">
          <!-- 8 animated probability bars -->
        </div>
      </div>
    </div>

    <!-- Right Drawer: Deep Telemetry & Step-by-Step Pipeline Inspector -->
    <div class="telemetry-drawer">
      <!-- Active Symbol Header Card -->
      <div class="drawer-header">
        <div>
          <div class="symbol-hero">
            <span id="card-symbol">BTC/USD</span>
            <span class="symbol-price" id="card-mid-price">$0.00</span>
          </div>
          <div class="symbol-micro">
            Micro-Price: <strong id="card-micro-price">$0.00</strong> &bull;
            Spread: <strong id="card-spread-bps" style="color: var(--cyan);">0.0</strong> bps
          </div>
        </div>
        <div style="text-align: right;">
          <div id="card-phase-badge" class="badge amber">THERMAL NOISE</div>
          <div style="font-size: 10px; color: var(--muted); margin-top: 4px;">Conf: <span id="card-phase-conf">0.0%</span></div>
        </div>
      </div>

      <!-- Step-by-Step Decision Pipeline Stepper -->
      <div class="section-title">
        <span>Evaluación Paso a Paso en Tiempo Real</span>
        <span style="font-size: 10px; color: var(--cyan);">6 GATES PRE-TRADE</span>
      </div>

      <div class="pipeline-stepper" id="pipeline-stepper-list">
        <!-- Step 1: Spread Check -->
        <div class="step-item">
          <div class="step-left">
            <div id="step-1-badge" class="step-num pass">1</div>
            <div>
              <div class="step-name">Límite de Spread Bid/Ask</div>
              <div class="step-desc" id="step-1-desc">Spread en bps vs límite dinámico</div>
            </div>
          </div>
          <div class="step-val" id="step-1-val" style="color: var(--emerald);">PASS</div>
        </div>

        <!-- Step 2: Friction Coverage -->
        <div class="step-item">
          <div class="step-left">
            <div id="step-2-badge" class="step-num pass">2</div>
            <div>
              <div class="step-name">Margen de Retorno vs Fricción</div>
              <div class="step-desc" id="step-2-desc">Target TP 45 bps - Costes &gt;= 10 bps</div>
            </div>
          </div>
          <div class="step-val" id="step-2-val" style="color: var(--emerald);">PASS</div>
        </div>

        <!-- Step 3: WS Latency Ceiling -->
        <div class="step-item">
          <div class="step-left">
            <div id="step-3-badge" class="step-num pass">3</div>
            <div>
              <div class="step-name">Techo de Latencia de Datos WS</div>
              <div class="step-desc" id="step-3-desc">Edad del paquete vs límite máx</div>
            </div>
          </div>
          <div class="step-val" id="step-3-val" style="color: var(--emerald);">PASS</div>
        </div>

        <!-- Step 4: Daily Drawdown Breaker -->
        <div class="step-item">
          <div class="step-left">
            <div id="step-4-badge" class="step-num pass">4</div>
            <div>
              <div class="step-name">Circuit Breaker Drawdown Diario</div>
              <div class="step-desc" id="step-4-desc">Pérdida intradiaria &lt; 2.0% equity</div>
            </div>
          </div>
          <div class="step-val" id="step-4-val" style="color: var(--emerald);">PASS</div>
        </div>

        <!-- Step 5: Anti-whipsaw & Macro Bias -->
        <div class="step-item">
          <div class="step-left">
            <div id="step-5-badge" class="step-num pass">5</div>
            <div>
              <div class="step-name">Ventana Anti-Whipsaw &amp; Sesgo Macro</div>
              <div class="step-desc" id="step-5-desc">Confirmación acumulada y sesgo Lead-Lag</div>
            </div>
          </div>
          <div class="step-val" id="step-5-val" style="color: var(--cyan);">ALINEADO</div>
        </div>

        <!-- Step 6: SOR Router & Kelly Sizing -->
        <div class="step-item">
          <div class="step-left">
            <div id="step-6-badge" class="step-num active">6</div>
            <div>
              <div class="step-name">Smart Order Router (SOR) &amp; Kelly</div>
              <div class="step-desc" id="step-6-desc">Clasificación de intención y apalancamiento</div>
            </div>
          </div>
          <div class="step-val" id="step-6-val" style="color: var(--purple);">NEUTRAL</div>
        </div>

        <!-- Step 7: Broker Execution Ready -->
        <div class="step-item">
          <div class="step-left">
            <div id="step-7-badge" class="step-num active">7</div>
            <div>
              <div class="step-name">Ejecución en Broker (Alpaca REST)</div>
              <div class="step-desc" id="step-7-desc">Limit GTC con techo de slippage</div>
            </div>
          </div>
          <div class="step-val" id="step-7-val" style="color: var(--cyan);">STANDBY</div>
        </div>
      </div>

      <!-- Broker Account & Risk Metrics -->
      <div class="section-title">
        <span>Auditoría de Portafolio &amp; Broker Alpaca</span>
        <span id="card-broker-account" style="color: var(--cyan); font-size: 11px;">PA3NGD2B8</span>
      </div>

      <div class="broker-grid">
        <div class="mini-stat">
          <div class="mini-label">Capital (Equity)</div>
          <div class="mini-val" id="card-equity" style="color: var(--emerald);">$0.00</div>
        </div>
        <div class="mini-stat">
          <div class="mini-label">Poder de Compra</div>
          <div class="mini-val" id="card-bp">$0.00</div>
        </div>
        <div class="mini-stat">
          <div class="mini-label">Sharpe Ratio</div>
          <div class="mini-val" id="card-sharpe" style="color: var(--cyan);">0.00</div>
        </div>
        <div class="mini-stat">
          <div class="mini-label">Tasa Acierto (WinRate)</div>
          <div class="mini-val" id="card-winrate">0.0%</div>
        </div>
        <div class="mini-stat">
          <div class="mini-label">Max Drawdown</div>
          <div class="mini-val" id="card-maxdd">0.00%</div>
        </div>
        <div class="mini-stat">
          <div class="mini-label">Apalancamiento Kelly</div>
          <div class="mini-val" id="card-kelly" style="color: var(--purple);">1.00x</div>
        </div>
      </div>

      <!-- Active Broker Positions -->
      <div class="section-title">
        <span>Posiciones Abiertas en Broker</span>
        <span id="pos-count" style="font-size: 11px; color: var(--muted);">0 ACTIVAS</span>
      </div>
      <div class="positions-wrap" id="positions-container">
        <div class="empty-state">No hay posiciones abiertas activas en el broker.</div>
      </div>

      <!-- Real-Time Event Stream Terminal -->
      <div class="section-title">
        <span>Consola de Eventos &amp; Auditoría en Vivo</span>
        <span style="font-size: 10px; color: var(--emerald);">● STREAMING</span>
      </div>
      <div class="terminal-box" id="terminal-event-log">
        <!-- Real-time Rolling Log Lines -->
      </div>
    </div>
  </div>

  <script>
    let isPaused = false;
    let selectedSymbolIndex = 0;
    let cachedState = {initial_json};

    function togglePause() {{
      isPaused = !isPaused;
      const btn = document.getElementById("btn-pause");
      if (isPaused) {{
        btn.classList.add("active");
        btn.innerText = "REANUDAR FLUJO";
      }} else {{
        btn.classList.remove("active");
        btn.innerText = "PAUSAR FLUJO";
      }}
    }}

    function selectSymbol(idx) {{
      selectedSymbolIndex = idx;
      renderCurrentSymbol();
    }}

    function selectStage(stageNum) {{
      const stepper = document.getElementById("pipeline-stepper-list");
      if (stepper && stepper.children[stageNum - 1]) {{
        const item = stepper.children[stageNum - 1];
        item.style.transform = "scale(1.02)";
        item.style.borderColor = "var(--cyan)";
        setTimeout(() => {{
          item.style.transform = "none";
          item.style.borderColor = "rgba(255, 255, 255, 0.05)";
        }}, 600);
      }}
    }}

    function renderCurrentSymbol() {{
      const s = cachedState;
      if (!s.symbols || s.symbols.length === 0) return;
      const sym = s.symbols[selectedSymbolIndex] || s.symbols[0];

      // Update Header Asset Tabs
      const tabsBar = document.getElementById("asset-tabs-bar");
      if (tabsBar) {{
        tabsBar.innerHTML = s.symbols.map((symObj, idx) => `
          <button class="asset-tab ${{idx === selectedSymbolIndex ? 'active' : ''}}" onclick="selectSymbol(${{idx}})">
            ${{symObj.symbol}} <span style="font-size: 10px; opacity: 0.8;">$${{symObj.mid_price.toFixed(2)}}</span>
          </button>
        `).join("");
      }}

      // Update Selected Symbol Details
      document.getElementById("card-symbol").innerText = sym.symbol;
      document.getElementById("card-mid-price").innerText = `$${{sym.mid_price.toFixed(2)}}`;
      document.getElementById("card-micro-price").innerText = `$${{sym.micro_price.toFixed(2)}}`;
      document.getElementById("card-spread-bps").innerText = sym.spread_bps.toFixed(1);

      // Phase Badge
      const phaseBadge = document.getElementById("card-phase-badge");
      phaseBadge.innerText = sym.active_phase;
      document.getElementById("card-phase-conf").innerText = `${{(sym.phase_confidence * 100).toFixed(1)}}%`;
      document.getElementById("svg-core-phase").innerText = sym.active_phase.toUpperCase();

      // Update Node 1 (NBBO & Spread)
      document.getElementById("node-1-val").innerText = `Spread: ${{sym.spread_bps.toFixed(1)}} bps &bull; Bid: $${{sym.best_bid.toFixed(2)}} Ask: $${{sym.best_ask.toFixed(2)}}`;
      const n1Status = document.getElementById("node-1-status");
      n1Status.innerText = sym.spread_bps <= 15.0 ? "LAMINAR" : "SPREAD ALTO";
      n1Status.style.color = sym.spread_bps <= 15.0 ? "var(--emerald)" : "var(--amber)";

      // Update Node 2 (Density Matrix)
      document.getElementById("node-2-val").innerText = `S(norm): ${{sym.normalized_entropy.toFixed(3)}} &bull; Coher: ${{sym.quantum_coherence.toFixed(3)}} &bull; Bias: ${{sym.state_bias.toFixed(2)}}`;
      const n2Status = document.getElementById("node-2-status");
      n2Status.innerText = sym.quantum_coherence >= 0.25 ? "COHERENTE" : "DECOHERIDO";
      n2Status.style.color = sym.quantum_coherence >= 0.25 ? "var(--purple)" : "var(--muted)";

      // Update Node 3 (Fluidics & Mach)
      document.getElementById("node-3-val").innerText = `Mach: ${{sym.mach_number.toFixed(2)}} &bull; cs: ${{sym.speed_of_sound.toFixed(1)}} &bull; Cavit: ${{sym.is_cavitation ? 'ALERTA' : 'No'}}`;
      const n3Status = document.getElementById("node-3-status");
      n3Status.innerText = sym.mach_number >= 1.5 ? "CHOQUE CAVIT" : "SUBSÓNICO";
      n3Status.style.color = sym.mach_number >= 1.5 ? "var(--rose)" : "var(--cyan)";

      // Update Node 4 (VPIN Toxic Shield)
      document.getElementById("node-4-val").innerText = `VPIN: ${{sym.vpin.toFixed(3)}} &bull; OFI: ${{sym.ofi_accel.toFixed(1)}}`;
      const n4Status = document.getElementById("node-4-status");
      n4Status.innerText = sym.is_toxic ? "ESCUDO BLOQUEO" : "PERMISIVO";
      n4Status.style.color = sym.is_toxic ? "var(--rose)" : "var(--emerald)";

      // Update Node 5 (Pre-Trade Gate)
      const passSpread = sym.spread_bps <= 15.0;
      const passToxic = !sym.is_toxic;
      document.getElementById("node-5-val").innerText = `Fricción: OK &bull; Macro Bias: ${{s.lead_lag_spy_bias >= 0 ? '+' : ''}}${{s.lead_lag_spy_bias.toFixed(2)}}`;
      const n5Status = document.getElementById("node-5-status");
      n5Status.innerText = (passSpread && passToxic) ? "GATE ABIERTO" : "FILTRANDO";
      n5Status.style.color = (passSpread && passToxic) ? "var(--emerald)" : "var(--amber)";

      // Update Node 6 (SOR Router)
      document.getElementById("node-6-val").innerText = `${{sym.sor_intent}} &bull; Kelly: ${{s.kelly_leverage.toFixed(2)}}x`;
      const n6Status = document.getElementById("node-6-status");
      if (sym.sor_intent.includes("AggressiveTaker")) {{
        n6Status.innerText = "TAKER ACTIVO"; n6Status.style.color = "var(--emerald)";
      }} else if (sym.sor_intent.includes("EmergencyCancel")) {{
        n6Status.innerText = "RETIRADA"; n6Status.style.color = "var(--rose)";
      }} else {{
        n6Status.innerText = "HOLD"; n6Status.style.color = "var(--purple)";
      }}

      // Update Step Stepper List
      document.getElementById("step-1-val").innerText = passSpread ? `${{sym.spread_bps.toFixed(1)}} bps (PASS)` : `${{sym.spread_bps.toFixed(1)}} bps (RECHAZO)`;
      document.getElementById("step-1-val").style.color = passSpread ? "var(--emerald)" : "var(--rose)";

      document.getElementById("step-4-val").innerText = `${{(s.max_drawdown * 100).toFixed(2)}}% (PASS)`;
      document.getElementById("step-5-val").innerText = `Bias: ${{s.lead_lag_spy_bias >= 0 ? '+' : ''}}${{s.lead_lag_spy_bias.toFixed(2)}}`;
      document.getElementById("step-6-val").innerText = sym.sor_intent;

      // Update CTQW 8-level dispersion bars
      const dispRow = document.getElementById("dispersion-bars-row");
      if (dispRow && sym.depth_dispersion) {{
        dispRow.innerHTML = sym.depth_dispersion.map((prob, idx) => {{
          const isMid = idx === 3 || idx === 4;
          const h = Math.max(4, Math.min(34, prob * 60));
          return `
            <div class="disp-bar-col">
              <div class="disp-bar ${{isMid ? 'mid' : ''}}" style="height: ${{h}}px;"></div>
              <span class="disp-bar-label">${{idx < 4 ? 'B' + (3 - idx) : 'A' + (idx - 4)}}</span>
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
        wsBadge.className = "badge offline";
        wsBadge.innerText = "○ WS DESCONECTADO";
      }}

      const marketBadge = document.getElementById("badge-market");
      if (s.market_open) {{
        marketBadge.className = "badge info";
        marketBadge.innerText = s.feed === "crypto" ? "● CRIPTO 24/7/365" : "● WALL STREET ABIERTO";
      }} else {{
        marketBadge.className = "badge offline";
        marketBadge.innerText = "○ MERCADO CERRADO";
      }}

      document.getElementById("badge-phase").innerText = `FASE: ${{s.market_phase}}`;
      document.getElementById("val-throughput").innerText = Math.round(s.throughput_ticks_per_sec);
      document.getElementById("val-rss").innerText = s.rss_mb.toFixed(2);

      // Broker Stats
      document.getElementById("card-broker-account").innerText = s.broker_account;
      document.getElementById("card-equity").innerText = `$${{s.current_equity.toLocaleString(undefined, {{minimumFractionDigits: 2, maximumFractionDigits: 2}})}} USD`;
      document.getElementById("card-bp").innerText = `$${{s.buying_power.toLocaleString(undefined, {{minimumFractionDigits: 2, maximumFractionDigits: 2}})}} USD`;
      document.getElementById("card-sharpe").innerText = s.sharpe_ratio.toFixed(2);
      document.getElementById("card-winrate").innerText = `${{(s.win_rate * 100).toFixed(1)}}%`;
      document.getElementById("card-maxdd").innerText = `${{(s.max_drawdown * 100).toFixed(2)}}%`;
      document.getElementById("card-kelly").innerText = `${{s.kelly_leverage.toFixed(2)}}x`;

      // Positions Container
      const posContainer = document.getElementById("positions-container");
      const posCountBadge = document.getElementById("pos-count");
      if (s.positions && s.positions.length > 0) {{
        posCountBadge.innerText = `${{s.positions.length}} ACTIVAS`;
        posCountBadge.style.color = "var(--emerald)";
        posContainer.innerHTML = s.positions.map(p => `
          <div class="pos-item">
            <div>
              <strong style="color: #fff;">${{p.symbol}}</strong>
              <span style="font-size: 10px; color: ${{p.side === 'LONG' ? 'var(--emerald)' : 'var(--rose)'}};">${{p.side}}</span>
              <div style="font-size: 10px; color: var(--muted);">Qty: ${{p.qty}} &bull; Entrada: $${{p.entry_price.toFixed(2)}}</div>
            </div>
            <div style="text-align: right;">
              <div style="font-weight: 700; color: ${{p.pnl_pct >= 0 ? 'var(--emerald)' : 'var(--rose)'}};">
                ${{p.pnl_pct >= 0 ? '+' : ''}}${{p.pnl_pct.toFixed(2)}}%
              </div>
              <div style="font-size: 10px; color: var(--muted);">${{p.pnl_usd >= 0 ? '+' : ''}}$${{p.pnl_usd.toFixed(2)}} USD</div>
            </div>
          </div>
        `).join("");
      }} else {{
        posCountBadge.innerText = "0 ACTIVAS";
        posCountBadge.style.color = "var(--muted)";
        posContainer.innerHTML = `<div class="empty-state">No hay posiciones abiertas activas en el broker.</div>`;
      }}

      // Event Logs Terminal
      const termBox = document.getElementById("terminal-event-log");
      if (s.recent_logs && s.recent_logs.length > 0) {{
        termBox.innerHTML = s.recent_logs.map(l => `
          <div class="term-line">
            <span class="term-time">[${{l.timestamp_str}}]</span>
            <span class="term-sym">${{l.symbol}}</span>
            <span class="term-msg ${{l.is_trade ? 'term-trade' : ''}}">${{l.message}}</span>
          </div>
        `).join("");
      }}

      renderCurrentSymbol();
    }}

    // Polling Loop (Smooth asynchronous fetch every 750ms without full page reloads)
    async function pollStatus() {{
      if (isPaused) return;
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

    // Initial render
    updateDashboard(cachedState);
    setInterval(pollStatus, 750);
  </script>
</body>
</html>"##)
}
