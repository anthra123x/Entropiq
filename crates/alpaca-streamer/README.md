# Alpaca-Streamer

## 1. Propósito y alcance
Módulo de conectividad en tiempo real y ejecución con Alpaca Markets. Administra conexiones WebSocket bidireccionales con reconexión defensiva, ingestión asíncrona mediante canales acotados, fijación de afinidad de CPU a núcleos dedicados, servidor de telemetría HTTP y el supervisor de producción.

## 2. Contratos
- `AlpacaStreamer`: `with_oauth_token`, `with_key_secret`, `stream_symbol(symbols, callback)`, `stream_with_symbol(symbols, callback)`.
- `AlpacaDataFeed`: Selección de canal de mercado: `Iex`, `Sip`, `Crypto` (`wss://stream.data.alpaca.markets/v1beta3/crypto/us`).
- `EnvConfig`: Cargador defensivo de variables de entorno (`.env` o proceso): stripping de comillas, fallback seguro y detección de autenticación.
- `AlpacaStreamMessage`: Mensajería WebSocket: `Quote` (`AlpacaQuote`), `Trade` (`AlpacaTrade`), `Subscription`, `Error`.
- `parse_alpaca_timestamp_ns`: Conversión zero-allocation de timestamps RFC3339/ISO8601 a nanosegundos Unix.
- `pin_current_thread_to_core(core_id)`: Aislamiento institucional de hilos en CPU Linux vía `pthread_setaffinity_np`.
- `start_web_server`: Servidor HTTP asíncrono para health check (`/health`), estado JSON (`/api/status`) y panel interactivo tipo red neuronal (`WebDashboardState`, `SymbolStateTelemetry`, `PositionTelemetry`, `PipelineStepTelemetry`, `PipelineLogEntry`).

## 3. Dependencias
- Internas: `quantum-engine`, `market-microstructure`, `market-simulator`.
- Externas: `tokio`, `tokio-tungstenite`, `rustls`, `serde`, `serde_json`, `futures-util`, `reqwest`.

## 4. Mapa de archivos
- `src/lib.rs` → Exportaciones principales del módulo.
- `src/client.rs` → Cliente WebSocket con handshake, suscripción y streaming.
- `src/model.rs` → Estructuras de datos del feed Alpaca v2 y parser de tiempo.
- `src/config.rs` → Carga de configuración y credenciales de entorno.
- `src/affinity.rs` → Fijación de afinidad de hilos a núcleos físicos de CPU.
- `src/web.rs` → Servidor HTTP integrado para monitoreo y verificación de salud en cloud.
- `src/bin/supervisor.rs` → Binario del supervisor de producción, pipeline pre-trade de 6 etapas y gestión de órdenes.

## 5. Decisiones y gotchas
- **Límite de conexiones WebSocket en Paper Trading**: Alpaca limita a 1 sola conexión activa por cuenta. Conexiones simultáneas devuelven error 406 (`connection limit exceeded`).
- **Órdenes Limit vs Market en Cripto**: Las órdenes tipo `limit` en Alpaca requieren estrictamente cantidad fraccional (`qty`); no admiten valor nocional (`notional`). Órdenes enviadas con `notional` en modo limit son rechazadas con error HTTP 422.
- **Filtro de latencia transfronteriza**: La latencia de red entre servidores locales fuera de EE.UU. y los endpoints de Alpaca suele oscilar entre 90 ms y 300 ms; filtros estáticos de latencia menores a este umbral descartan todo el flujo en entornos locales.
- **Mandato Long-Only en Acciones**: En acciones e índices estadounidenses se opera exclusivamente en largo (`Side::Buy`), evitando posiciones cortas expuestas a short squeezes y deriva positiva del mercado.
- **Liquidación Real al Bid (Anti Spread-Trap)**: Alpaca liquida posiciones (`DELETE /v2/positions`) mediante órdenes Market, las cuales se cruzan al mejor Bid (`best_bid`) en compras. El cálculo de ganancia neta se evalúa estrictamente contra el Bid ejecutable real ($\ge +\$0.05$ USD y retorno neto positivo), eliminando falsas salidas en pérdida provocadas por fluctuaciones del precio medio (`mid_price`).
- **Bloqueo Atómico en Memoria (Zero Pre-Trade Latency)**: Se reemplazaron las llamadas HTTP bloqueantes previas al trade por un guardia en memoria local (`live_positions`), reduciendo la latencia previa de 800 ms a 0 ms y bloqueando compras duplicadas. La reconciliación de detalles de ejecución se realiza en segundo plano cada 20s con periodo de gracia.
- **Cancelación Atómica Integrada**: `DELETE /v2/positions` en Alpaca cancela automáticamente las órdenes asociadas al activo antes de liquidar, reduciendo la latencia de salida de 800 ms a ~350 ms al suprimir la llamada previa redundante a `/v2/orders`.
- **Mandato de Cero Pérdidas y Cosecha Estricta de Ganancias**: Queda prohibido liquidar posiciones en negativo (sin stop-loss destructivos ni centinelas que asuman pérdidas). La posición se sostiene hasta que el bid ejecutable garantice ganancia neta (mínimo +$0.05 USD y retorno positivo), asegurando un crecimiento monótono del capital.

## 6. Cómo verificar
```bash
cargo test -p alpaca-streamer
cargo run -p alpaca-streamer --example live_stream_demo
```

