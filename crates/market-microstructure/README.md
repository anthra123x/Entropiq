# Market-Microstructure

## 1. Propósito y alcance
Manejo de estados microestructurales de mercado de alta frecuencia. Procesa cotizaciones L1 (NBBO), libros de órdenes L2 en stack, Order Flow Imbalance (OFI), métricas de flujo tóxico (VPIN) e hidrodinámica de liquidez (número de Mach y riesgo de cavitación).

## 2. Contratos
- `Quote`: `new(ts, bid, bid_size, ask, ask_size)`, `spread()`, `mid_price()`, `micro_price()`, `imbalance()`.
- `Trade`: `new(ts, price, size, taker_side)`, clasificación direccional agresiva `Side::Buy` / `Side::Sell`.
- `L2Book<DEPTH>`: Libro L2 de tamaño fijo en stack, `deep_ofi(&prev_book)`, `cumulative_barrier_height(side, levels)`.
- `MarketTracker`: Rastreador de microestructura con decaimiento temporal exponencial de volumen, OFI acumulado y cálculo de VPIN en cubetas de volumen institucional.
- `LiquidityFluidModel`: Modelo acústico/hidrodinámico de liquidez con velocidad de propagación $c_s = \sqrt{\gamma \cdot \text{depth} \cdot \text{spread}}$, velocidad de flujo $v$, número de Mach $M = v / c_s$ y detección de choque de cavitación.

## 3. Dependencias
- Internas: Ninguna (módulo fundacional de microestructura).
- Externas: Ninguna (optimizado para stack allocation y latencia submilisegundo).

## 4. Mapa de archivos
- `src/lib.rs` → Exportaciones públicas de tipos y pruebas unitarias.
- `src/types.rs` → Estructuras base de eventos de mercado: `Quote`, `Trade`, enum `Side`.
- `src/book_l2.rs` → Representación y métricas de libro L2 multicapa (`L2Book`).
- `src/state.rs` → `MarketTracker`, seguimiento de OFI temporal y cubetas VPIN.
- `src/fluid.rs` → Dinámica de fluidos en liquidez y cálculo del número de Mach.

## 5. Decisiones y gotchas
- **Cero latencia en L2**: `L2Book<DEPTH>` utiliza arrays fijos en stack sin `Vec` ni indirección en punteros para garantizar afinidad con la caché L1 de CPU.
- **Normalización de Mach**: En activos de precio nominal elevado (ej. BTC a \$70k+), las fluctuaciones nominales en dólares deben escalarse por el mid-price para no inducir falsos choques supersónicos (Mach $\ge 1.5$).
- **Decaimiento temporal**: El decaimiento exponencial de volumen se aplica de forma determinista usando el delta temporal en nanosegundos del timestamp del exchange.

## 6. Cómo verificar
```bash
cargo test -p market-microstructure
```

