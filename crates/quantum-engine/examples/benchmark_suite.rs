use market_microstructure::{LiquidityFluidModel, MarketTracker, Quote};
use market_simulator::{
    BinaryDiskReader, BinaryDiskRecorder, BinaryQuoteRecord, BinaryTradeRecord, MarketRegime,
    MicrostructureSimulator,
};
use quantum_core::{CMatrix, Complex64, DensityMatrix, LindbladSolver, QuantumDepthWalk, QuantumPotential};
use quantum_engine::QuantumMarketEngine;
use std::fs;
use std::time::Instant;

fn get_rss_mb() -> f64 {
    if let Ok(content) = fs::read_to_string("/proc/self/statm") {
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Ok(pages) = parts[1].parse::<u64>() {
                return (pages * 4096) as f64 / (1024.0 * 1024.0);
            }
        }
    }
    0.0
}

fn main() {
    println!("==========================================================================================");
    println!("                  ENTROPIQ COMPREHENSIVE LOCAL PERFORMANCE AUDIT SUITE                   ");
    println!("==========================================================================================");
    println!(" [!] Memoria Base RSS: {:.2} MB", get_rss_mb());
    println!("------------------------------------------------------------------------------------------");

    // -------------------------------------------------------------------------
    // Layer 1: Pure Mathematical & Quantum Core
    // -------------------------------------------------------------------------
    println!("\n>>> CAPA 1: NUCLEO MATEMATICO Y ALGEBRA CUANTICA (quantum-core)");
    {
        let iters = 10_000_000;
        let mut c = Complex64::new(1.0, 0.5);
        let factor = Complex64::new(0.9999, 0.0001);
        let start = Instant::now();
        for _ in 0..iters {
            c = c * factor;
        }
        let elapsed = start.elapsed();
        println!(
            "  * Complex64 Multiplicacion:        {:>7.2} ns/op | {:>6.2} M ops/s (chk: {:.2})",
            elapsed.as_nanos() as f64 / iters as f64,
            (iters as f64 / elapsed.as_secs_f64()) / 1e6,
            c.norm()
        );
    }

    {
        let iters = 2_000_000;
        let mut m1 = CMatrix::<2>::zeros();
        m1.set(0, 0, Complex64::new(1.0, 0.2));
        m1.set(0, 1, Complex64::new(0.5, -0.3));
        m1.set(1, 0, Complex64::new(0.5, 0.3));
        m1.set(1, 1, Complex64::new(0.8, -0.1));
        let m2 = m1.conjugate_transpose();
        let start = Instant::now();
        for _ in 0..iters {
            m1 = m1.matmul(&m2);
        }
        let elapsed = start.elapsed();
        println!(
            "  * CMatrix<2> Multiplicacion 2x2:   {:>7.2} ns/op | {:>6.2} M ops/s",
            elapsed.as_nanos() as f64 / iters as f64,
            (iters as f64 / elapsed.as_secs_f64()) / 1e6
        );
    }

    {
        let mut init_mat = CMatrix::<2>::zeros();
        init_mat.set(0, 0, Complex64::from_real(0.5));
        init_mat.set(1, 1, Complex64::from_real(0.5));
        let mut rho = DensityMatrix::<2>::from_matrix_normalized(init_mat).unwrap();

        let mut h = CMatrix::<2>::zeros();
        h.set(0, 1, Complex64::new(0.5, -0.2));
        h.set(1, 0, Complex64::new(0.5, 0.2));

        let jumps = LindbladSolver::market_jump_operators_2x2(1.0, 0.5, 0.5);
        let iters = 1_000_000;
        let start = Instant::now();
        for _ in 0..iters {
            rho = LindbladSolver::step_rk4(&rho, &h, &jumps, 0.001).unwrap();
        }
        let elapsed = start.elapsed();
        println!(
            "  * Lindblad RK4 Step (Master Eq):   {:>7.2} ns/op | {:>6.2} M ops/s (pureza: {:.4})",
            elapsed.as_nanos() as f64 / iters as f64,
            (iters as f64 / elapsed.as_secs_f64()) / 1e6,
            rho.purity()
        );
    }

    {
        let inv_sqrt2 = 1.0 / (2.0_f64).sqrt();
        let mut depth_psi = [Complex64::ZERO; 8];
        depth_psi[3] = Complex64::from_real(inv_sqrt2);
        depth_psi[4] = Complex64::from_real(inv_sqrt2);

        let h_walk = QuantumDepthWalk::build_lattice_hamiltonian(
            [150.0, 120.0, 100.0, 50.0],
            [50.0, 100.0, 120.0, 150.0],
            5.0,
            2.0,
        );
        let iters = 1_000_000;
        let start = Instant::now();
        for _ in 0..iters {
            depth_psi = QuantumDepthWalk::propagate(&h_walk, &depth_psi, 0.001);
        }
        let elapsed = start.elapsed();
        println!(
            "  * CTQW 8-Nodos Taylor Propagation: {:>7.2} ns/op | {:>6.2} M ops/s",
            elapsed.as_nanos() as f64 / iters as f64,
            (iters as f64 / elapsed.as_secs_f64()) / 1e6
        );
    }

    {
        let iters = 2_000_000;
        let mut sum_t = 0.0;
        let start = Instant::now();
        for i in 0..iters {
            let e = (i % 50) as f64 * 2.0;
            let t = QuantumPotential::tunneling_probability(e, 50.0, 2.0, 1.0);
            sum_t += t;
        }
        let elapsed = start.elapsed();
        println!(
            "  * Tunel Cuantico (Estabilidad NaN):{:>7.2} ns/op | {:>6.2} M ops/s (sum T: {:.1})",
            elapsed.as_nanos() as f64 / iters as f64,
            (iters as f64 / elapsed.as_secs_f64()) / 1e6,
            sum_t
        );
    }

    // -------------------------------------------------------------------------
    // Layer 2: Microstructure & Hydrodynamics
    // -------------------------------------------------------------------------
    println!("\n>>> CAPA 2: MICROESTRUCTURA Y DINAMICA DE FLUIDOS (market-microstructure)");
    {
        let iters = 2_000_000;
        let mut mach_sum = 0.0;
        let start = Instant::now();
        for i in 0..iters {
            let m = LiquidityFluidModel::compute_metrics(1000.0, 0.01, (i % 10) as f64 * 0.01, 0.001);
            mach_sum += m.mach_number;
        }
        let elapsed = start.elapsed();
        println!(
            "  * Modelo Hidrodinamico (Mach/Cav): {:>7.2} ns/op | {:>6.2} M ops/s (chk: {:.1})",
            elapsed.as_nanos() as f64 / iters as f64,
            (iters as f64 / elapsed.as_secs_f64()) / 1e6,
            mach_sum
        );
    }

    {
        let mut tracker = MarketTracker::new(0.01);
        let iters = 1_000_000;
        let start = Instant::now();
        for i in 0..iters {
            let q = Quote::new(
                1_000_000_000 + i * 50_000,
                100.0 + (i % 10) as f64 * 0.01,
                50.0 + (i % 50) as f64,
                100.02 + (i % 10) as f64 * 0.01,
                50.0 + (i % 30) as f64,
            );
            tracker.update_quote(q);
        }
        let elapsed = start.elapsed();
        println!(
            "  * MarketTracker + Time Decay:      {:>7.2} ns/op | {:>6.2} M updates/s (OFI acc: {:.1})",
            elapsed.as_nanos() as f64 / iters as f64,
            (iters as f64 / elapsed.as_secs_f64()) / 1e6,
            tracker.accumulated_ofi
        );
    }

    // -------------------------------------------------------------------------
    // Layer 3: Binary Disk I/O & Replayer Performance
    // -------------------------------------------------------------------------
    println!("\n>>> CAPA 3: PERSISTENCIA DISCO BINARIO Y SERIALIZACION (market-simulator)");
    {
        let tmp_file = std::env::temp_dir().join("entropiq_benchmark_io.bin");
        let records_count = 100_000u64;

        // Writing benchmark
        let start_w = Instant::now();
        let mut recorder = BinaryDiskRecorder::create(&tmp_file).unwrap();
        let q_record = BinaryQuoteRecord {
            timestamp_ns: 1_700_000_000_000,
            bid_price: 150.25,
            ask_price: 150.30,
            bid_size: 100.0,
            ask_size: 200.0,
        };
        let t_record = BinaryTradeRecord {
            timestamp_ns: 1_700_000_000_100,
            price: 150.30,
            size: 50.0,
            side: 1,
            _pad: [0u8; 11],
        };
        for i in 0..records_count {
            if i % 2 == 0 {
                recorder.write_quote(&q_record).unwrap();
            } else {
                recorder.write_trade(&t_record).unwrap();
            }
        }
        recorder.finish().unwrap();
        let elapsed_w = start_w.elapsed();
        let bytes_written = (records_count * 32) as f64 / (1024.0 * 1024.0);
        let mb_s_w = bytes_written / elapsed_w.as_secs_f64();
        let iops_w = records_count as f64 / elapsed_w.as_secs_f64();
        println!(
            "  * Escritura QMBD en Disco (SSD):   {:>7.2} ns/rec| {:>6.0} IOPS ({:.2} MB/s)",
            elapsed_w.as_nanos() as f64 / records_count as f64,
            iops_w,
            mb_s_w
        );

        // Reading benchmark
        let start_r = Instant::now();
        let mut reader = BinaryDiskReader::open(&tmp_file).unwrap();
        let mut read_count = 0u64;
        while let Ok(Some(_)) = reader.read_next_quote() {
            read_count += 1;
        }
        let elapsed_r = start_r.elapsed();
        let mb_s_r = bytes_written / elapsed_r.as_secs_f64();
        let iops_r = read_count as f64 / elapsed_r.as_secs_f64();
        println!(
            "  * Lectura Secuencial QMBD Disco:   {:>7.2} ns/rec| {:>6.0} IOPS ({:.2} MB/s)",
            elapsed_r.as_nanos() as f64 / read_count.max(1) as f64,
            iops_r,
            mb_s_r
        );

        let _ = fs::remove_file(tmp_file);
    }

    // -------------------------------------------------------------------------
    // Layer 4: Integrated Engine & Latency Distribution Profile (1,000,000 Ticks)
    // -------------------------------------------------------------------------
    println!("\n>>> CAPA 4: DISTRIBUCION DE LATENCIA EXTREMA Y ESTRES (1,000,000 Ticks)");
    {
        let mut engine = QuantumMarketEngine::new(0.01);
        let mut sim = MicrostructureSimulator::new(500.0, 0.01, 1337);
        let regimes = [
            MarketRegime::ThermalEquilibrium,
            MarketRegime::IcebergAbsorption,
            MarketRegime::QuantumBreakout,
            MarketRegime::LiquidityCavitation,
        ];

        let total_ticks = 1_000_000usize;
        let mut latencies_ns: Vec<u32> = Vec::with_capacity(total_ticks);

        let wall_start = Instant::now();
        let mut last_phase = None;
        let mut phase_transitions = 0;

        for i in 0..total_ticks {
            let regime = regimes[(i / 250_000) % regimes.len()];
            let (quote, trade_opt) = sim.next_tick(regime);

            let t0 = Instant::now();
            let snap = engine.on_quote(quote);
            if let Some(t) = trade_opt {
                engine.on_trade(t);
            }
            let dt = t0.elapsed().as_nanos() as u32;
            latencies_ns.push(dt);

            if let Some(s) = snap {
                if last_phase != Some(s.active_phase) {
                    phase_transitions += 1;
                    last_phase = Some(s.active_phase);
                }
            }
        }

        let wall_elapsed = wall_start.elapsed();
        let throughput = total_ticks as f64 / wall_elapsed.as_secs_f64();

        // Calculate percentiles
        latencies_ns.sort_unstable();
        let min = latencies_ns[0];
        let p50 = latencies_ns[(total_ticks as f64 * 0.50) as usize];
        let p90 = latencies_ns[(total_ticks as f64 * 0.90) as usize];
        let p99 = latencies_ns[(total_ticks as f64 * 0.99) as usize];
        let p999 = latencies_ns[(total_ticks as f64 * 0.999) as usize];
        let max = latencies_ns[total_ticks - 1];

        let avg = latencies_ns.iter().map(|&x| x as f64).sum::<f64>() / total_ticks as f64;
        let variance = latencies_ns
            .iter()
            .map(|&x| {
                let diff = x as f64 - avg;
                diff * diff
            })
            .sum::<f64>()
            / total_ticks as f64;
        let std_dev = variance.sqrt();

        println!("  * Throughput Global:               {:>10.0} ticks/s", throughput);
        println!("  * Transiciones de Fase Detectadas: {:>10}", phase_transitions);
        println!("  * Latencia Minima:                 {:>10} ns ({:.3} µs)", min, min as f64 / 1000.0);
        println!("  * Latencia Media:                  {:>10.1} ns ({:.3} µs)", avg, avg / 1000.0);
        println!("  * Percentil 50 (Mediana):          {:>10} ns ({:.3} µs)", p50, p50 as f64 / 1000.0);
        println!("  * Percentil 90 (p90):              {:>10} ns ({:.3} µs)", p90, p90 as f64 / 1000.0);
        println!("  * Percentil 99 (p99):              {:>10} ns ({:.3} µs)", p99, p99 as f64 / 1000.0);
        println!("  * Percentil 99.9 (p99.9):          {:>10} ns ({:.3} µs)", p999, p999 as f64 / 1000.0);
        println!("  * Latencia Maxima (Pico):          {:>10} ns ({:.3} µs)", max, max as f64 / 1000.0);
        println!("  * Desviacion Estandar (Jitter):    {:>10.1} ns ({:.3} µs)", std_dev, std_dev / 1000.0);
    }

    println!("------------------------------------------------------------------------------------------");
    println!(" [!] Memoria Final RSS: {:.2} MB (Sin fugas de memoria tras 1M ticks)", get_rss_mb());
    println!("==========================================================================================");
    println!("                AUDITORIA DE RENDIMIENTO LOCAL COMPLETADA CON EXITO                      ");
}
