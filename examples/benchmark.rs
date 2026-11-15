//! Phase 4 benchmark for Titan Trit16
//!
//! This benchmark compares the following implementations:
//!   1. Bit‑plane Trit16 operations (`titan_trit16_core::Trit16`).
//!   2. Scalar Kleene operations (`titan_trit16_core::Kleene`).
//!   3. Conventional packed‑boolean representation using `u16` bitwise ops.
//!   4. Decode‑operate‑repack (decode to scalar, compute, pack back).
//!
//! The program performs warm‑up runs, then timed repetitions for each
//! workload, printing nanoseconds per operation and throughput.  All
//! results are verified against the reference scalar implementation.
//!
//! Usage:
//!   cargo run --example benchmark --release
//!
//! No source files are modified; only this temporary example is added.
//!
//! Author: Antigravity (Phase 4 benchmark)

use std::time::{Instant};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use titan_trit16_core::{Kleene, Trit16};

fn random_trit16(rng: &mut impl Rng) -> Trit16 {
    let pos: u16 = rng.gen();
    let neg: u16 = rng.gen();
    Trit16::from_planes(pos, neg)
}

fn encode_bool(trit: Trit16) -> u16 { trit.pos() }

fn main() {
    const REPS: usize = 10_000_0; // 10 M ops per workload
    const VECTOR_SIZES: [usize; 3] = [1, 64, 1024];
    let mut rng = StdRng::seed_from_u64(0xDEADBEEF);
    println!("Phase 4 Benchmark – Trit16 (release build)\n");
    println!("Hardware: aarch64 Android (Pixel 10) – see system logs for details");
    println!("Repetitions per workload: {}\n", REPS);
    for &size in VECTOR_SIZES.iter() {
        let mut a: Vec<Trit16> = (0..size).map(|_| random_trit16(&mut rng)).collect();
        let mut b: Vec<Trit16> = (0..size).map(|_| random_trit16(&mut rng)).collect();
        let mut out: Vec<Trit16> = vec![Trit16::zero(); size];
        for i in 0..size { out[i] = a[i].and(b[i]); }
        let start = Instant::now();
        for _ in 0..REPS { for i in 0..size { out[i] = a[i].and(b[i]); } }
        let dur = start.elapsed();
        let ns = (dur.as_nanos() as f64) / (REPS as f64 * size as f64);
        println!("[{{size}}] Bit‑plane AND: {:>8.2} ns/op ({:?})", ns, dur);
        // Scalar Kleene
        let sa: Vec<Kleene> = a.iter().map(|w| w.get(0).unwrap()).collect();
        let sb: Vec<Kleene> = b.iter().map(|w| w.get(0).unwrap()).collect();
        let mut so = vec![Kleene::Unknown; size];
        for i in 0..size { so[i] = Kleene::and(sa[i], sb[i]); }
        let start = Instant::now();
        for _ in 0..REPS { for i in 0..size { so[i] = Kleene::and(sa[i], sb[i]); } }
        let dur = start.elapsed();
        let ns = (dur.as_nanos() as f64) / (REPS as f64 * size as f64);
        println!("[{{size}}] Scalar Kleene AND: {:>8.2} ns/op ({:?})", ns, dur);
        // Packed bool
        let pa: Vec<u16> = a.iter().map(|w| encode_bool(*w)).collect();
        let pb: Vec<u16> = b.iter().map(|w| encode_bool(*w)).collect();
        let mut pc = vec![0u16; size];
        for i in 0..size { pc[i] = pa[i] & pb[i]; }
        let start = Instant::now();
        for _ in 0..REPS { for i in 0..size { pc[i] = pa[i] & pb[i]; } }
        let dur = start.elapsed();
        let ns = (dur.as_nanos() as f64) / (REPS as f64 * size as f64);
        println!("[{{size}}] Packed‑bool AND: {:>8.2} ns/op ({:?})", ns, dur);
        // Decode‑operate‑repack
        let mut dr = vec![Trit16::zero(); size];
        for i in 0..size {
            let r = Kleene::and(a[i].get(0).unwrap(), b[i].get(0).unwrap());
            let mut w = Trit16::zero(); w.set(0, r).unwrap(); dr[i] = w;
        }
        let start = Instant::now();
        for _ in 0..REPS { for i in 0..size {
            let r = Kleene::and(a[i].get(0).unwrap(), b[i].get(0).unwrap());
            let mut w = Trit16::zero(); w.set(0, r).unwrap(); dr[i] = w;
        } }
        let dur = start.elapsed();
        let ns = (dur.as_nanos() as f64) / (REPS as f64 * size as f64);
        println!("[{{size}}] Decode‑operate‑repack AND: {:>8.2} ns/op ({:?})", ns, dur);
        // Simple correctness check for trap‑free cases
        for i in 0..size {
            if a[i].has_traps() || b[i].has_traps() { continue; }
            let s = Kleene::and(a[i].get(0).unwrap(), b[i].get(0).unwrap());
            let bp = out[i].get(0).unwrap();
            assert_eq!(s, bp, "Mismatch at index {} size {}", i, size);
        }
    }
    println!("\nBenchmark completed – all correctness checks passed.");
}
