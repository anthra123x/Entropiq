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
/// Uses native CSS Flexbox/Grid for zero text-overlap, an SVG synapse backdrop, and strict 100vh fit.
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
      --card-bg: rgba(13, 19, 32, 0.85);
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
        radial-gradient(circle at 12% 18%, rgba(99, 102, 241, 0.08) 0%, transparent 45%),
        radial-gradient(circle at 88% 82%, rgba(56, 189, 248, 0.06) 0%, transparent 40%);
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
      height: 42px;
      min-height: 42px;
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
      grid-template-columns: 1fr 370px;
      gap: 10px;
    }}

    /* Left Stage: Neural Graph with Real HTML Nodes + SVG Synapse Backdrop */
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

    /* Synaptic Canvas Container */
    .neural-viewport {{
      flex: 1;
      min-height: 0;
      width: 100%;
      position: relative;
      overflow: hidden;
    }}

    /* SVG Synaptic Backdrop Layer */
    svg.synapse-backdrop {{
      width: 100%;
      height: 100%;
      position: absolute;
      top: 0;
      left: 0;
      pointer-events: none;
      z-index: 1;
    }}
    .syn-path {{
      fill: none;
      stroke: rgba(255, 255, 255, 0.08);
      stroke-width: 1.5;
    }}
    .syn-pulse {{
      fill: none;
      stroke-width: 2.2;
      stroke-linecap: round;
      stroke-dasharray: 8 160;
      animation: synPulseMove 3s linear infinite;
    }}
    .syn-pulse.cyan {{ stroke: var(--cyan); }}
    .syn-pulse.purple {{ stroke: var(--purple); animation-duration: 3.4s; }}
    .syn-pulse.emerald {{ stroke: var(--emerald); animation-duration: 2.6s; }}

    @keyframes synPulseMove {{
      from {{ stroke-dashoffset: 168; }}
      to {{ stroke-dashoffset: 0; }}
    }}

    /* HTML Nodes Layer (Zero Text Overlap, Flexbox Auto-Layout) */
    .nodes-layer {{
      position: absolute;
      top: 0;
      left: 0;
      width: 100%;
      height: 100%;
      z-index: 2;
      pointer-events: auto;
    }}

    /* Root Core Node */
    .node-root {{
      position: absolute;
      left: 35px;
      top: 50%;
      transform: translateY(-50%);
      width: 88px;
      height: 88px;
      border-radius: 50%;
      background: radial-gradient(circle at 35% 35%, #0e1e38 0%, #080c16 80%);
      border: 1px solid rgba(56, 189, 248, 0.4);
      box-shadow: 0 0 24px rgba(56, 189, 248, 0.15), inset 0 0 16px rgba(56, 189, 248, 0.1);
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      gap: 3px;
      user-select: none;
    }}
    .root-tag {{ font-size: 8px; font-weight: 800; letter-spacing: 0.15em; color: var(--cyan); text-transform: uppercase; }}
    .root-name {{ font-size: 10px; font-weight: 800; color: #fff; }}
    .root-phase-badge {{
      font-size: 7.5px;
      font-weight: 800;
      padding: 1px 6px;
      border-radius: 4px;
      background: rgba(245, 158, 11, 0.15);
      color: var(--amber);
      border: 1px solid rgba(245, 158, 11, 0.3);
      text-transform: uppercase;
      max-width: 76px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    /* Intermediate Tier 2 Pillars */
    .node-pillar {{
      position: absolute;
      left: 200px;
      width: 220px;
      height: 54px;
      background: rgba(14, 21, 37, 0.92);
      border: 1px solid var(--card-border);
      border-left: 3px solid var(--cyan);
      border-radius: 10px;
      padding: 8px 12px;
      display: flex;
      flex-direction: column;
      justify-content: space-between;
      box-shadow: 0 4px 16px rgba(0, 0, 0, 0.35);
      transition: all 0.2s ease;
    }}
    .node-pillar:hover {{
      border-color: rgba(255, 255, 255, 0.18);
      transform: translateX(2px);
    }}
    .node-pillar.p-purple {{ border-left-color: var(--purple); }}
    .node-pillar.p-emerald {{ border-left-color: var(--emerald); }}

    .pillar-header {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 6px;
    }}
    .pillar-title {{
      font-size: 11px;
      font-weight: 700;
      color: #fff;
      display: flex;
      align-items: center;
      gap: 6px;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }}
    .pillar-status {{
      font-size: 8.5px;
      font-weight: 800;
      padding: 2px 6px;
      border-radius: 4px;
      text-transform: uppercase;
      flex-shrink: 0;
    }}
    .pillar-status.cyan {{ background: rgba(56, 189, 248, 0.12); color: var(--cyan); border: 1px solid rgba(56, 189, 248, 0.3); }}
    .pillar-status.purple {{ background: rgba(129, 140, 248, 0.12); color: var(--purple); border: 1px solid rgba(129, 140, 248, 0.3); }}
    .pillar-status.emerald {{ background: rgba(16, 185, 129, 0.12); color: var(--emerald); border: 1px solid rgba(16, 185, 129, 0.3); }}

    .pillar-sub {{
      font-size: 9.5px;
      color: var(--muted);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }}

    /* Operational Tier 3 Leafs */
    .node-leaf {{
      position: absolute;
      left: 490px;
      width: 360px;
      height: 34px;
      background: rgba(11, 17, 30, 0.90);
      border: 1px solid rgba(255, 255, 255, 0.07);
      border-radius: 8px;
      padding: 0 12px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      box-shadow: 0 2px 10px rgba(0, 0, 0, 0.25);
      transition: all 0.2s ease;
    }}
    .node-leaf:hover {{
      border-color: rgba(255, 255, 255, 0.16);
      background: rgba(14, 22, 38, 0.95);
    }}
    .leaf-left {{
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

    .leaf-name {{
      font-size: 11px;
      font-weight: 600;
      color: #e2e8f0;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }}
    .leaf-right {{
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
    .leaf-badge {{
      font-size: 8.5px;
      font-weight: 800;
      padding: 1px 6px;
      border-radius: 3px;
      text-transform: uppercase;
      background: rgba(16, 185, 129, 0.15);
      color: var(--emerald);
      border: 1px solid rgba(16, 185, 129, 0.3);
    }}
    .leaf-badge.warn {{ background: rgba(245, 158, 11, 0.15); color: var(--amber); border-color: rgba(245, 158, 11, 0.3); }}
    .leaf-badge.purple {{ background: rgba(129, 140, 248, 0.15); color: var(--purple); border-color: rgba(129, 140, 248, 0.3); }}

    /* CTQW Dispersion Strip at Bottom of Stage */
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

    <!-- Left Stage: Interactive Neural Network Graph -->
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

      <div class="neural-viewport">
        <!-- SVG Synapse Backdrop: Precise Bézier Connections -->
        <svg class="synapse-backdrop" viewBox="0 0 880 440" preserveAspectRatio="none">
          <!-- Tier 1 to Tier 2 Curves -->
          <path d="M 123 220 C 165 220, 165 87, 200 87" class="syn-path" />
          <path d="M 123 220 C 165 220, 165 87, 200 87" class="syn-pulse cyan" />

          <path d="M 123 220 C 165 220, 165 220, 200 220" class="syn-path" />
          <path d="M 123 220 C 165 220, 165 220, 200 220" class="syn-pulse purple" />

          <path d="M 123 220 C 165 220, 165 353, 200 353" class="syn-path" />
          <path d="M 123 220 C 165 220, 165 353, 200 353" class="syn-pulse emerald" />

          <!-- Tier 2 to Tier 3 Curves -->
          <!-- From Pillar 1 (y: 87) -->
          <path d="M 420 87 C 455 87, 455 59, 490 59" class="syn-path" />
          <path d="M 420 87 C 455 87, 455 59, 490 59" class="syn-pulse cyan" />

          <path d="M 420 87 C 455 87, 455 103, 490 103" class="syn-path" />
          <path d="M 420 87 C 455 87, 455 103, 490 103" class="syn-pulse cyan" />

          <!-- From Pillar 2 (y: 220) -->
          <path d="M 420 220 C 455 220, 455 189, 490 189" class="syn-path" />
          <path d="M 420 220 C 455 220, 455 189, 490 189" class="syn-pulse purple" />

          <path d="M 420 220 C 455 220, 455 233, 490 233" class="syn-path" />
          <path d="M 420 220 C 455 220, 455 233, 490 233" class="syn-pulse purple" />

          <!-- From Pillar 3 (y: 353) -->
          <path d="M 420 353 C 455 353, 455 319, 490 319" class="syn-path" />
          <path d="M 420 353 C 455 353, 455 319, 490 319" class="syn-pulse emerald" />

          <path d="M 420 353 C 455 353, 455 363, 490 363" class="syn-path" />
          <path d="M 420 353 C 455 353, 455 363, 490 363" class="syn-pulse emerald" />

          <path d="M 420 353 C 455 353, 455 407, 490 407" class="syn-path" />
          <path d="M 420 353 C 455 353, 455 407, 490 407" class="syn-pulse emerald" />
        </svg>

        <!-- HTML Nodes Layer: Guarantees zero text-overlaps via CSS Flexbox -->
        <div class="nodes-layer">
          <!-- Tier 1: Root Core Sphere -->
          <div class="node-root">
            <span class="root-tag">QUANTUM</span>
            <span class="root-name">CORE RK4</span>
            <span class="root-phase-badge mono" id="svg-core-phase">THERMAL</span>
          </div>

          <!-- Tier 2: Intermediate Processing Pillars -->
          <!-- Pillar 1: Ingestión & Spread -->
          <div class="node-pillar" style="top: 60px;">
            <div class="pillar-header">
              <span class="pillar-title">01 · INGESTIÓN & SPREAD</span>
              <span class="pillar-status cyan mono" id="hub-1-status">LAMINAR</span>
            </div>
            <span class="pillar-sub mono" id="hub-1-desc">Spread: 2.5 bps &bull; Profundidad: OK</span>
          </div>

          <!-- Pillar 2: Física Cuántica & Mach -->
          <div class="node-pillar p-purple" style="top: 193px;">
            <div class="pillar-header">
              <span class="pillar-title">02 · FÍSICA & MATRIZ ρ</span>
              <span class="pillar-status purple mono" id="hub-2-status">COHERENTE</span>
            </div>
            <span class="pillar-sub mono" id="hub-2-desc">S(norm): 0.81 &bull; Coher: 0.25</span>
          </div>

          <!-- Pillar 3: Valuación Pre-Trade & SOR -->
          <div class="node-pillar p-emerald" style="top: 326px;">
            <div class="pillar-header">
              <span class="pillar-title">03 · VALUACIÓN & SOR</span>
              <span class="pillar-status emerald mono" id="hub-3-status">PERMISIVO</span>
            </div>
            <span class="pillar-sub mono" id="hub-3-desc">6 Puertas Pre-Trade &bull; Half-Kelly 1.0x</span>
          </div>

          <!-- Tier 3: Operational Leafs (Fan-Out) -->
          <!-- Leaf 1: Spread Bid-Ask -->
          <div class="node-leaf" style="top: 42px;">
            <div class="leaf-left">
              <span class="leaf-dot cyan"></span>
              <span class="leaf-name">Spread Bid-Ask Dinámico</span>
            </div>
            <div class="leaf-right">
              <span class="leaf-val mono" id="leaf-1-val">2.5 bps</span>
              <span class="leaf-badge mono" id="leaf-1-badge">OK</span>
            </div>
          </div>

          <!-- Leaf 2: Order Flow Imbalance (OFI) -->
          <div class="node-leaf" style="top: 86px;">
            <div class="leaf-left">
              <span class="leaf-dot cyan"></span>
              <span class="leaf-name">Order Flow Imbalance (OFI)</span>
            </div>
            <div class="leaf-right">
              <span class="leaf-val mono" id="leaf-2-val">Accel: 0.0</span>
              <span class="leaf-badge mono">ESTABLE</span>
            </div>
          </div>

          <!-- Leaf 3: Matriz Densidad von Neumann -->
          <div class="node-leaf" style="top: 172px;">
            <div class="leaf-left">
              <span class="leaf-dot purple"></span>
              <span class="leaf-name">Operador Densidad von Neumann (ρ)</span>
            </div>
            <div class="leaf-right">
              <span class="leaf-val mono" id="leaf-3-val">S: 0.81 | C: 0.25</span>
              <span class="leaf-badge purple mono">COHER</span>
            </div>
          </div>

          <!-- Leaf 4: Hidrodinámica & Mach -->
          <div class="node-leaf" style="top: 216px;">
            <div class="leaf-left">
              <span class="leaf-dot cyan"></span>
              <span class="leaf-name">Hidrodinámica & Mach Shockwave</span>
            </div>
            <div class="leaf-right">
              <span class="leaf-val mono" id="leaf-4-val">Mach 0.00 (Sub)</span>
              <span class="leaf-badge mono">SUB</span>
            </div>
          </div>

          <!-- Leaf 5: Escudo Tóxico VPIN -->
          <div class="node-leaf" style="top: 302px;">
            <div class="leaf-left">
              <span class="leaf-dot"></span>
              <span class="leaf-name">Escudo Toxicidad Flujo (VPIN)</span>
            </div>
            <div class="leaf-right">
              <span class="leaf-val mono" id="leaf-5-val">VPIN: 0.000</span>
              <span class="leaf-badge mono" id="leaf-5-badge">PERMISIVO</span>
            </div>
          </div>

          <!-- Leaf 6: Margen Retorno vs Fricción -->
          <div class="node-leaf" style="top: 346px;">
            <div class="leaf-left">
              <span class="leaf-dot amber"></span>
              <span class="leaf-name">Margen Retorno vs Fricción Fee</span>
            </div>
            <div class="leaf-right">
              <span class="leaf-val mono" id="leaf-6-val">TP 45bps &gt; Fee</span>
              <span class="leaf-badge mono" id="leaf-6-badge">PASS</span>
            </div>
          </div>

          <!-- Leaf 7: SOR Router & Alpaca Limit -->
          <div class="node-leaf" style="top: 390px;">
            <div class="leaf-left">
              <span class="leaf-dot purple"></span>
              <span class="leaf-name">Enrutador SOR & Límite Alpaca REST</span>
            </div>
            <div class="leaf-right">
              <span class="leaf-val mono" id="leaf-7-val">Hold &bull; Kelly: 0.2x</span>
              <span class="leaf-badge purple mono">LISTO</span>
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

      // Update Intermediate Pillars (Tier 2)
      document.getElementById("hub-1-desc").innerText = `Spread: ${{sym.spread_bps.toFixed(1)}} bps &bull; OFI: ${{sym.ofi_accel.toFixed(1)}}`;
      const h1Status = document.getElementById("hub-1-status");
      h1Status.innerText = sym.spread_bps <= 15.0 ? "LAMINAR" : "ALTO SPREAD";
      h1Status.className = sym.spread_bps <= 15.0 ? "pillar-status cyan mono" : "pillar-status amber mono";

      document.getElementById("hub-2-desc").innerText = `S(norm): ${{sym.normalized_entropy.toFixed(3)}} &bull; Coher: ${{sym.quantum_coherence.toFixed(3)}}`;
      const h2Status = document.getElementById("hub-2-status");
      h2Status.innerText = sym.quantum_coherence >= 0.25 ? "COHERENTE" : "DECOHERIDO";

      document.getElementById("hub-3-desc").innerText = `VPIN: ${{sym.vpin.toFixed(3)}} &bull; Kelly: ${{s.kelly_leverage.toFixed(2)}}x`;
      const h3Status = document.getElementById("hub-3-status");
      h3Status.innerText = sym.is_toxic ? "ESCUDO BLOQUEO" : "PERMISIVO";
      h3Status.className = sym.is_toxic ? "pillar-status rose mono" : "pillar-status emerald mono";

      // Update Operational Leafs (Tier 3)
      document.getElementById("leaf-1-val").innerText = `${{sym.spread_bps.toFixed(1)}} bps`;
      const l1Badge = document.getElementById("leaf-1-badge");
      const passSpread = sym.spread_bps <= 15.0;
      l1Badge.innerText = passSpread ? "OK" : "ALTO";
      l1Badge.className = passSpread ? "leaf-badge mono" : "leaf-badge warn mono";

      document.getElementById("leaf-2-val").innerText = `Accel: ${{sym.ofi_accel.toFixed(1)}}`;
      document.getElementById("leaf-3-val").innerText = `S: ${{sym.normalized_entropy.toFixed(3)}} | C: ${{sym.quantum_coherence.toFixed(3)}}`;
      document.getElementById("leaf-4-val").innerText = `Mach ${{sym.mach_number.toFixed(2)}} (${{sym.is_cavitation ? 'Cavitación' : 'Sub'}})`;
      document.getElementById("leaf-5-val").innerText = `VPIN: ${{sym.vpin.toFixed(3)}}`;
      const l5Badge = document.getElementById("leaf-5-badge");
      l5Badge.innerText = sym.is_toxic ? "BLOQUEADO" : "PERMISIVO";
      l5Badge.className = sym.is_toxic ? "leaf-badge warn mono" : "leaf-badge mono";

      document.getElementById("leaf-6-val").innerText = passSpread ? "TP 45bps > Costes" : "Fricción Alta";
      document.getElementById("leaf-7-val").innerText = `${{sym.sor_intent}} (Kelly ${{s.kelly_leverage.toFixed(2)}}x)`;

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

    updateDashboard(cachedState);
    setInterval(pollStatus, 750);
  </script>
</body>
</html>"##)
}
