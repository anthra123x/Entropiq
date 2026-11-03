# Quantum-Engine

## 1. Propósito y alcance
Motor unificado de orquestación analítica cuántico-econofísica. Transforma el flujo de datos de microestructura en matrices de densidad, clasifica las fases cuánticas de mercado en tiempo real, ejecuta el Smart Order Router (SOR) y audita el portafolio mediante dimensionamiento Half-Kelly.

## 2. Contratos
- `QuantumMarketEngine`: `new(tick_size)`, `on_quote(quote) -> Option<QuantumAnalyticsSnapshot>`, `on_trade(trade)`, `on_l2_book(book)`, `set_external_index_bias(bias)`.
- `QuantumAnalyticsSnapshot`: Contenedor inmutable de telemetría: `mid_price`, `micro_price`, `normalized_entropy`, `quantum_coherence`, `vpin`, `state_bias`, `tunneling_prob_ask/bid`, `active_phase`, `fluid_metrics`.
- `MarketQuantumPhase`: Clasificación de estado: `ThermalNoise`, `CoherentCondensate`, `SuperconductingBreakout`, `LiquidityCavitation`.
- `PhaseClassifier`: Algoritmo bayesiano/econofísico para clasificar la fase cuántica activa y su nivel de confianza.
- `SmartOrderRouter`: `route_intent(snapshot, best_bid, best_ask, kelly_f) -> OrderIntent` (`AggressiveTaker`, `PassiveMaker`, `NeutralHold`, `EmergencyCancelAll`).
- `PortfolioAuditor`: Auditor estadístico $O(1)$ con algoritmo de Welford online, ratio de Sharpe, ratio de Calmar, apalancamiento Kelly dinámico y control de drawdown.

## 3. Dependencias
- Internas: `quantum-core`, `market-microstructure`.
- Externas: Ninguna (mantiene cero reservas dinámicas en el ciclo de cálculo).

## 4. Mapa de archivos
- `src/lib.rs` → Módulo raíz y re-exportación de contratos públicos del motor.
- `src/engine.rs` → Bucle principal `QuantumMarketEngine` y generación de snapshots.
- `src/hamiltonian.rs` → Construcción del operador Hamiltoniano con sesgo macro Lead-Lag.
- `src/phase.rs` → Clasificador de fases cuánticas (`PhaseClassifier`).
- `src/sor.rs` → Smart Order Router (`OrderIntent`, filtros de cavitación y toxicidad).
- `src/audit.rs` → Monitoreo de riesgo, apalancamiento Half-Kelly y métricas de rendimiento.

## 5. Decisiones y gotchas
- **Prioridad de seguridad en SOR**: Ante choque de cavitación (Mach $\ge 1.5$) o flujo tóxico informado (VPIN $\ge 0.75$), el router emite inmediatamente `EmergencyCancelAll`.
- **Condición estricta de Breakout**: No se permite transición a `SuperconductingBreakout` si la coherencia cuántica es residual ($< 0.10$) o si el flujo es supersónico, evitando operaciones en caídas de cuchillo (falling knives).
- **Ejecución Directa en CoherentCondensate**: Las acumulaciones institucionales detectadas bajo flujo laminar se enrutan como `AggressiveTaker` para capturar el punto de inflexión sin dejar órdenes pasivas colgadas en el libro.

## 6. Cómo verificar
```bash
cargo test -p quantum-engine
```

