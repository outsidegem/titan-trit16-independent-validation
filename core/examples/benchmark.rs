// Phase 4 benchmark – placed under core crate
use std::time::Instant;
// Simple deterministic pseudo‑random generator (linear congruential)
const LCG_A: u64 = 6364136223846793005;
const LCG_C: u64 = 1;
struct Lcg {
    state: u64,
}
impl Lcg {
    fn next_u16(&mut self) -> u16 {
        self.state = self.state.wrapping_mul(LCG_A).wrapping_add(LCG_C);
        (self.state >> 48) as u16
    }
}

use titan_trit16_core::{Kleene, Trit16};

fn random_trit16(lcg: &mut Lcg) -> Trit16 {
    let pos: u16 = lcg.next_u16();
    let neg: u16 = lcg.next_u16();
    Trit16::from_planes(pos, neg)
}

fn encode_bool(trit: Trit16) -> u16 {
    trit.pos()
}

fn main() {
    const REPS: usize = 50_000; // 5 M ops per size
    const SIZES: [usize; 3] = [1, 64, 1024];
    let mut rng = Lcg { state: 0xC0FFEE };
    println!("Phase 4 benchmark – Trit16 (release)");
    for &size in &SIZES {
        // prepare vectors
        let a: Vec<Trit16> = (0..size).map(|_| random_trit16(&mut rng)).collect();
        let b: Vec<Trit16> = (0..size).map(|_| random_trit16(&mut rng)).collect();
        let mut out = vec![Trit16::zero(); size];
        // warm‑up
        for i in 0..size {
            out[i] = a[i].and(b[i]);
        }
        // timed bit‑plane AND
        let start = Instant::now();
        for _ in 0..REPS {
            for i in 0..size {
                out[i] = a[i].and(b[i]);
            }
        }
        let dur = start.elapsed();
        let ns = (dur.as_nanos() as f64) / (REPS as f64 * size as f64);
        println!("[{{size}}] Bit‑plane AND: {:>8.2} ns/op ({:?})", ns, dur);
        // scalar Kleene
        let sa: Vec<Kleene> = a.iter().map(|w| w.get(0).unwrap()).collect();
        let sb: Vec<Kleene> = b.iter().map(|w| w.get(0).unwrap()).collect();
        let mut so = vec![Kleene::Unknown; size];
        for i in 0..size {
            so[i] = Kleene::and(sa[i], sb[i]);
        }
        let start = Instant::now();
        for _ in 0..REPS {
            for i in 0..size {
                so[i] = Kleene::and(sa[i], sb[i]);
            }
        }
        let dur = start.elapsed();
        let ns = (dur.as_nanos() as f64) / (REPS as f64 * size as f64);
        println!(
            "[{{size}}] Scalar Kleene AND: {:>8.2} ns/op ({:?})",
            ns, dur
        );
        // packed bool (u16)
        let pa: Vec<u16> = a.iter().map(|w| encode_bool(*w)).collect();
        let pb: Vec<u16> = b.iter().map(|w| encode_bool(*w)).collect();
        let mut pc = vec![0u16; size];
        for i in 0..size {
            pc[i] = pa[i] & pb[i];
        }
        let start = Instant::now();
        for _ in 0..REPS {
            for i in 0..size {
                pc[i] = pa[i] & pb[i];
            }
        }
        let dur = start.elapsed();
        let ns = (dur.as_nanos() as f64) / (REPS as f64 * size as f64);
        println!("[{{size}}] Packed‑bool AND: {:>8.2} ns/op ({:?})", ns, dur);
        // decode‑operate‑repack
        let mut dr = vec![Trit16::zero(); size];
        for i in 0..size {
            let r = Kleene::and(a[i].get(0).unwrap(), b[i].get(0).unwrap());
            let mut w = Trit16::zero();
            w.set(0, r).unwrap();
            dr[i] = w;
        }
        let start = Instant::now();
        for _ in 0..REPS {
            for i in 0..size {
                let r = Kleene::and(a[i].get(0).unwrap(), b[i].get(0).unwrap());
                let mut w = Trit16::zero();
                w.set(0, r).unwrap();
                dr[i] = w;
            }
        }
        let dur = start.elapsed();
        let ns = (dur.as_nanos() as f64) / (REPS as f64 * size as f64);
        println!(
            "[{{size}}] Decode‑operate‑repack AND: {:>8.2} ns/op ({:?})",
            ns, dur
        );
        // correctness for trap‑free subset
        for i in 0..size {
            if a[i].has_traps() || b[i].has_traps() {
                continue;
            }
            assert_eq!(
                out[i].get(0).unwrap(),
                Kleene::and(a[i].get(0).unwrap(), b[i].get(0).unwrap())
            );
        }
    }
    println!("Benchmark finished – all checks passed.");
}
