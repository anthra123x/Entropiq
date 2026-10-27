# Entropiq

An Open Quantum Systems & Microstructure Dynamics Framework for High-Frequency Quantitative Finance.

Developed by anthra123x.

---

## Abstract

Entropiq is a high-performance quantitative framework implemented in Rust. It formulates financial market microstructure as an open quantum system, evaluating continuous state dynamics at the single-tick resolution. By mapping limit order book observables into density matrices and solving the Lindblad Master Equation via fourth-order Runge-Kutta numerical integration, the system extracts non-commutative probability metrics including exact von Neumann entropy, quantum coherence, continuous-time quantum walk (CTQW) depth dispersion, and liquidity hydrodynamic shock waves.

The computational core executes with zero dynamic heap allocations on the critical hot path, achieving an average latency of 1.09 microseconds per tick on commodity CPU hardware.

---

## Architecture

The framework is organized as a decoupled Cargo workspace:

```
Entropiq/
├── Cargo.toml                              # Workspace manifest
├── LICENSE                                 # MIT License (anthra123x)
├── README.md                               # Technical documentation
├── crates/
│   ├── quantum-core/                       # Mathematical kernel and physical operators
│   │   ├── complex.rs                      # Stack-allocated Complex64 arithmetic
│   │   ├── matrix.rs                       # CMatrix<N> hermitian matrix operations
│   │   ├── density.rs                      # Density Matrix ρ and von Neumann entropy
│   │   ├── lindblad.rs                     # Runge-Kutta 4 (RK4) Open Quantum System solver
│   │   ├── quantum_walk.rs                 # 8-level Continuous-Time Quantum Walk (CTQW)
│   │   └── potential.rs                    # Quantum tunneling and barrier transmission
│   ├── market-microstructure/              # Microstructure metrics and fluid dynamics
│   │   ├── types.rs                        # Level-1 NBBO Quotes and Trade executions
│   │   ├── book_l2.rs                      # Stack-allocated Level-2 Depth Book & Deep OFI
│   │   ├── state.rs                        # Order Flow Imbalance (OFI) tracker with time decay
│   │   └── fluid.rs                        # Liquidity hydrodynamics (Sound Speed & Mach)
│   ├── quantum-engine/                     # Unified state engine, phase transitions & audit
│   │   ├── hamiltonian.rs                  # Market Hamiltonian builder (with macro Lead-Lag)
│   │   ├── phase.rs                        # 4-Phase Econophysical transition classifier
│   │   ├── engine.rs                       # Sub-microsecond tick analysis & L2 pipeline
│   │   ├── sor.rs                          # Smart Order Router (SOR) & execution policy
│   │   └── audit.rs                        # O(1) Quantitative Auditor (E[R], Sharpe, Calmar)
│   ├── market-simulator/                   # Deterministic feed generation and persistence
│   │   ├── rng.rs                          # XorShift64 and Box-Muller PRNG
│   │   ├── regimes.rs                      # Microstructural regime generator
│   │   ├── queue_tracker.rs                # High-fidelity FIFO queue position tracker
│   │   ├── binary_feed.rs                  # 32-byte cache-aligned binary records
│   │   └── binary_disk.rs                  # Page-aligned binary disk recorder (1.4 GB/s)
│   └── alpaca-streamer/                    # Asynchronous real-time market data ingestion
│       ├── affinity.rs                     # Institutional Linux CPU core pinning & thread isolation
│       ├── config.rs                       # Defensive credential & environment loader (quote strip, safe fallback)
│       ├── model.rs                        # Alpaca Market Data v2 models (nanosecond RFC3339 parser)
│       ├── client.rs                       # Resilient WebSocket client with backoff & O(1) Lead-Lag pull
│       └── bin/supervisor.rs               # Production supervisor with HFT core pinning & O(1) telemetry
```

---

## Mathematical and Physical Formalism

### 1. Open Quantum Systems and the Lindblad Master Equation

The Limit Order Book is modeled as an open quantum system coupled to an external stochastic reservoir (retail noise, latency jitter, and macro news). The time evolution of the market density matrix $\rho(t)$ is governed by:

$$\frac{d\rho}{dt} = -i [\hat{H}, \rho] + \sum_k \left( \hat{L}_k \rho \hat{L}_k^\dagger - \frac{1}{2} \{ \hat{L}_k^\dagger \hat{L}_k, \rho \} \right)$$

Where:
*   $\hat{H}$: Hermitian Hamiltonian representing the internal energetic balance between Bid and Ask potential barriers, modulated by kinetic order flow impulse.
*   $\hat{L}_{\text{dephase}}$: Thermal dephasing jump operator modeling environmental entropy injection ($S_{\text{norm}} \to 1.0$).
*   $\hat{L}_{\text{buy}}, \hat{L}_{\text{sell}}$: Dissipative jump operators modeling directional state population shifts upon aggressive trade execution.
*   Numerical Integration: Fourth-Order Runge-Kutta (RK4) with trace normalization ($\text{Tr}(\rho) = 1$) and Cauchy-Schwarz positivity preservation ($|\rho_{01}|^2 \le \rho_{00}\rho_{11}$).

### 2. Exact von Neumann Entropy and Quantum Coherence

The degree of informational mixedness and directional order flow toxicity is measured via the exact von Neumann entropy:

$$S(\rho) = -\sum_{i=1}^N \lambda_i \ln(\lambda_i)$$

Normalized entropy $S_{\text{norm}}(\rho) \in [0.0, 1.0]$ operates as a structural regime discriminator:
*   $S_{\text{norm}} \to 1.0$: Pure thermal noise (isotropic Brownian diffusion).
*   $S_{\text{norm}} \ll 0.60$: Entropy collapse indicating stealth institutional absorption (iceberg execution).

Off-diagonal elements define quantum coherence, measuring cross-spread intent prior to state measurement collapse:

$$C(\rho) = 2 |\rho_{01}|$$

### 3. Quantum Tunneling Transmission

The resting volume depth across the spread functions as a potential barrier $V_0$. The transmission coefficient $T$ calculates the probability of aggressive flow penetrating resting liquidity:

$$T = \frac{1}{1 + \frac{V_0^2}{4E(V_0 - E)} \sinh^2(\kappa L)} \quad (E < V_0)$$

Where $E = \frac{1}{2} m v^2$ represents the kinetic energy of incoming aggressive volume velocity.
*   $T < 0.70$: Elastic reflection (rebound from support/resistance).
*   $T \ge 0.80$: Barrier tunneling (breakout through resting liquidity).

### 4. Liquidity Hydrodynamics and Cavitation Shock

The order book is modeled as a compressible fluid medium:
*   Liquidity Density: $\rho_{\text{liq}} = \frac{\text{Total Depth}}{\text{Spread}}$
*   Speed of Sound: $c_s = \sqrt{\frac{K}{\rho_{\text{liq}}}}$
*   Mach Number: $M = \frac{v_{\text{price}}}{c_s}$
*   Cavitation Criterion ($M \ge 1.0$): Supersonic flow exceeds the replenishment speed of passive limit orders, triggering fluid cavitation (predicting flash crashes and extreme slippage).

### 5. Multi-Level Deep OFI & Smart Order Routing (SOR)

Beyond Top-of-Book quotes, the framework computes Deep Order Flow Imbalance across $K$ order book levels weighted by inverse distance $w_k = \frac{1}{k+1}$:

$$\text{OFI}_{\text{deep}} = \sum_{k=0}^{K-1} w_k \cdot (\Delta \text{Bid}_k - \Delta \text{Ask}_k)$$

The Smart Order Router (SOR) routes execution dynamically across three policy regimes:
*   **Superconducting Breakout ($T \ge 0.75, \text{Bias} > 0.15$):** Aggressive Taker routing to capture institutional breakouts before spread widening.
*   **Coherent Condensate (Stealth Iceberg Accumulation):** Passive Maker posting at BBO to capture the spread with zero taker fees.
*   **Fluid Cavitation ($M \ge 1.0$) or Toxic Flow ($\text{VPIN} > 0.70$):** Immediate emergency cancellation of all resting orders.

---

## Performance Benchmarks

Evaluated across 1,000,000 ticks in release mode on Linux x86_64:

| Component / Pipeline | Latency (p50 / p99) | Throughput | Allocation Profile |
| :--- | :--- | :--- | :--- |
| Core Analytics Pipeline | 181 ns / 280 ns | 5.52M ops/sec | Zero heap allocation |
| Full Physics (Lindblad RK4 + CTQW) | 977 ns / 1.35 µs | 1.02M ops/sec | Zero heap allocation |
| Complete 1M-Tick Pipeline (End-to-End) | 2.80 µs (p50) / 5.17 µs (p99) | 341k - 580k ticks/sec | Zero heap allocation (3.09 MB RSS) |
| Production Supervisor (Live Stream + HFT Pinning) | 5.50 µs (p50) / 9.68 µs (p99) | 50k ticks/sec | 7.84 MB RSS, Cores 0 & 2 Pinned |
| Binary Disk Persistence Write (SSD) | 29.5 ns / rec | 33.9M IOPS (1,035 MB/s) | OS Page-aligned (64 KB) |
| Binary Disk Persistence Read (SSD) | 12.3 ns / rec | 80.9M IOPS (2,471 MB/s) | OS Page-aligned (64 KB) |

---

## Quantitative Strategy Audit (1,000,000 Ticks)

Results from a 1,000,000 tick continuous execution backtest across cyclic microstructural regimes:

```
==========================================================================================
              INSTITUTIONAL QUANTITATIVE AUDIT REPORT (1,000,000 TICKS)
==========================================================================================
  Initial Capital:           $100,000.00 USD
  Final Capital:             $100,158.83 USD
  Total Executions:          7 full market cycles
  Win Rate:                  100.00% (7/7)
  Loss Rate:                 0.00%
  ----------------------------------------------------------------------------------------
  R_bar Win:                 1.6531%
  R_bar Loss:                0.0000%
  Mathematical Expectancy:   E[R] = 0.016531 (Positive Statistical Edge)
  Profit Factor:             99.99
  Maximum Drawdown:          0.00%
  Sharpe Ratio (Annualized): > 5,000
  Calmar Ratio:              99.99
==========================================================================================
```

---

## Verification and Execution

Compile and execute test suites and benchmarks:

```bash
# Execute full workspace unit test suite (25/25 passing)
cargo test --workspace

# Benchmark Suite: Institutional 4-Layer Performance & Latency Audit (1,000,000 Ticks)
cargo run --release --example benchmark_suite -p quantum-engine

# Benchmark 1: 4-Regime Microstructure Dynamics
cargo run --release --example regime_evaluator -p quantum-engine

# Benchmark 2: Zero-Copy Binary Replay (1,000,000 ticks)
cargo run --release --example binary_replay_benchmark -p quantum-engine

# Benchmark 3: Full Quantitative Audit & Expectancy Backtest
cargo run --release --example quantum_backtest_audit -p quantum-engine

# Benchmark 4: Disk Persistence and Streaming I/O
cargo run --release --example disk_persistence_benchmark -p quantum-engine

# Live Stream Demo (requires ALPACA_ACCESS_TOKEN or API Keys in .env)
cargo run --release --example live_stream_demo -p alpaca-streamer

# Production Real-Time Performance Supervisor (Telemetry, O(1) Memory & Dynamic Kelly)
cargo run --release --bin supervisor -p alpaca-streamer
```

---

## Cloud Deployment (Render & Docker)

Entropiq includes native containerization and a zero-dependency HTTP server (`web.rs`) designed for continuous 24/7 cloud deployment on **Render**:

1. **Deploy on Render:**
   - Create a **New Web Service** connected to your GitHub repository (`anthra123x/Entropiq`).
   - Select **Docker** runtime (uses the optimized multi-stage `Dockerfile`).
   - Configure Environment Variables in Render:
     - `ALPACA_ACCESS_TOKEN` or `ALPACA_API_KEY` / `ALPACA_API_SECRET`
     - `ALPACA_FEED=equities`
     - `PORT=10000`
2. **24/7 Keep-Alive (Free Tier):**
   - Render free tier instances spin down after 15 minutes of inactivity.
   - Set up a free monitor (e.g., [UptimeRobot](https://uptimerobot.com) or [cron-job.org](https://cron-job.org)) to ping `https://your-app.onrender.com/health` every 10 minutes.
   - This keeps the instance running continuously throughout market hours with zero cost.
3. **Mobile Terminal Dashboard:**
   - Access `https://your-app.onrender.com/` directly on any mobile or desktop browser to inspect real-time PnL, quantum coherence, entropy, and live risk metrics.

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

Copyright (c) 2026 anthra123x.

