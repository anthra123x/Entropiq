# Market-Simulator

## 1. Propósito y alcance
Simulador estocástico determinista y banco de pruebas de microestructura. Genera flujos sintéticos de mercado calibrados por regímenes, simula colas FIFO de prioridad de órdenes L2 y proporciona serialización/deserialización binaria compacta a disco de alto rendimiento (1.4+ GB/s).

## 2. Contratos
- `MicrostructureSimulator`: `new(initial_price, tick_size, seed)`, `next_tick(regime) -> (Quote, Option<Trade>)`.
- `MarketRegime`: Regímenes de mercado simulados: `ThermalEquilibrium`, `IcebergAbsorption`, `QuantumBreakout`, `LiquidityCavitation`.
- `QueueOrderTracker`: `new(price, side, size, qty_ahead)`, `on_trade(trade) -> f64`, rastreo de posición en cola FIFO con estado `QueueOrderStatus`.
- `BinaryQuoteRecord` / `BinaryTradeRecord`: Registros planos de 32 bytes con alineación estricta para serialización cero-copia.
- `BinaryDiskRecorder` / `BinaryDiskReader`: Grabador y reproductor de archivos binarios en bloques alineados a página.
- `BinaryBufferReplayer`: Reproducción en memoria para backtesting de alta frecuencia.

## 3. Dependencias
- Internas: `market-microstructure`.
- Externas: Ninguna (utiliza generador PRNG XorShift64 interno con algoritmo Box-Muller).

## 4. Mapa de archivos
- `src/lib.rs` → Exportaciones del simulador y pruebas de ciclo de vida.
- `src/rng.rs` → Generador pseudoaleatorio ultrarrápido (`FastRng`).
- `src/regimes.rs` → Generador de cotizaciones y trades sintéticos según régimen.
- `src/queue_tracker.rs` → Seguimiento de ejecución y fill de órdenes en cola FIFO.
- `src/binary_feed.rs` → Definición de registros binarios de 32 bytes y replayer en buffer.
- `src/binary_disk.rs` → Entrada/Salida a disco con buffers alineados.

## 5. Decisiones y gotchas
- **Alineación de 32 bytes**: Cada cotización o trade binario ocupa exactamente 32 bytes sin acolchado residual, permitiendo que 2 registros quepan perfectamente en una línea de caché típica de 64 bytes de la CPU.
- **Determinismo estricto**: Inicializar con la misma semilla (`seed`) reproduce exactamente la misma secuencia de ticks y ejecuciones para auditorías y regression tests.

## 6. Cómo verificar
```bash
cargo test -p market-simulator
```

