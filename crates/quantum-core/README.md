# Quantum-Core

## 1. Propósito y alcance
Núcleo algebraico y formalismo de física cuántica para el modelado de microestructura como sistema cuántico abierto. Implementa matrices complejas en stack sin asignación dinámica de memoria (`no_std` compatible en el hot path), operadores Lindblad, cálculo de entropía de von Neumann, dispersión CTQW y efecto túnel.

## 2. Contratos
- `Complex64`: `new(re, im)`, aritmética básica (`+`, `-`, `*`), conjugado `conj()`, norma euclidiana `norm_sq()`, `norm()`.
- `CMatrix<N>`: Matriz hermítica/complexa estática $N \times N$, multiplicación `matmul`, traza `trace()`, conmutador `commutator()`, anticonmutador `anticommutator()`.
- `DensityMatrix<N>`: Operador densidad $\rho$, pureza `purity()`, entropía exacta de von Neumann `von_neumann_entropy()`, `normalized_entropy()` en $[0, 1]$, coherencia cuántica $C(\rho) = 2|\rho_{01}|$.
- `LindbladSolver`: Integración numérica Runge-Kutta 4 (RK4) para la ecuación maestra de Lindblad con preservación de traza unitaria y semidefinición positiva de Cauchy-Schwarz.
- `QuantumPotential`: Transmisión y reflexión cuántica a través de barreras de liquidez bid/ask vía función hiperbólica $T = (1 + \frac{V_0^2}{4E(V_0-E)} \sinh^2(\kappa L))^{-1}$.
- `QuantumDepthWalk`: Caminata cuántica en tiempo continuo (CTQW) sobre retículo unidimensional de 8 niveles de profundidad L2.

## 3. Dependencias
- Internas: Ninguna (módulo base del workspace).
- Externas: Ninguna (utiliza exclusivamente tipos primitivos de Rust estándar con cero allocations en heap).

## 4. Mapa de archivos
- `src/lib.rs` → Exportación de tipos públicos y pruebas de integración física.
- `src/complex.rs` → Representación y aritmética de números complejos en 64 bits.
- `src/matrix.rs` → Álgebra de matrices cuadradas estáticas de tamaño $N$.
- `src/density.rs` → Matriz de densidad $\rho$, espectro de autovalores, entropía y coherencia.
- `src/lindblad.rs` → Solucionador RK4 para disipación térmica y saltos direccionales.
- `src/potential.rs` → Cálculo de barreras de potencial y probabilidad de penetración túnel.
- `src/quantum_walk.rs` → Dinámica de dispersión de paquete de ondas CTQW en el book.

## 5. Decisiones y gotchas
- **Cero heap allocation**: Toda estructura de datos usa arrays planos `[Complex64; N*N]`.
- **Estabilidad numérica en RK4**: La traza se renormaliza explícitamente a 1.0 en cada paso para evitar deriva numérica flotante en simulaciones continuas.
- **Caso borde en entropía**: Si los autovalores $\lambda_i \le 0$, se descartan para evitar evaluar $\ln(0)$ o $\text{NaN}$.

## 6. Cómo verificar
```bash
cargo test -p quantum-core
cargo test -p quantum-core -- --nocapture
```

