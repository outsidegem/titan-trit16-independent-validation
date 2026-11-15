//! Cross-language differential, alignment, symbol export, and C11/C++17 build tests.

use std::ffi::CStr;
use std::process::Command;
use titan_trit16_core::{Kleene, Trit16, TritError};
use titan_trit16_ffi::*;

const ALL_STATES: [Kleene; 4] = [Kleene::Unknown, Kleene::True, Kleene::False, Kleene::Trap];

#[test]
fn test_abi_version_and_error_strings() {
    assert_eq!(titan_trit16_abi_version(), 0x0001_0000);
    let ver = unsafe { CStr::from_ptr(titan_trit16_abi_version_string()) };
    assert_eq!(ver.to_str().unwrap(), "1.0.0");

    let msg_ok = unsafe { CStr::from_ptr(titan_trit16_error_message(TitanError::Ok as i32)) };
    assert_eq!(msg_ok.to_str().unwrap(), "OK");

    // Verify legitimate all-Trap (0xFFFF / 0xFFFF_FFFF) across all value-returning operations
    let all_trap = titan_trit16_from_planes(0xFFFF, 0xFFFF);
    assert_eq!(titan_trit16_pos(all_trap), 0xFFFF);
    assert_eq!(titan_trit16_neg(all_trap), 0xFFFF);
    assert!(!titan_trit16_is_valid(all_trap));
    assert!(titan_trit16_has_traps(all_trap));
    assert_eq!(titan_trit16_trap_mask(all_trap), 0xFFFF);
    assert_eq!(titan_trit16_and(all_trap, all_trap), all_trap);
    assert_eq!(titan_trit16_or(all_trap, all_trap), all_trap);
    assert_eq!(titan_trit16_not(all_trap), all_trap);
    assert_eq!(
        titan_trit16_collide(all_trap, all_trap),
        titan_trit16_zero()
    );
    assert_eq!(titan_trit16_to_u32_interleaved(all_trap), 0xFFFF_FFFF);
    assert_eq!(titan_trit16_from_u32_interleaved(0xFFFF_FFFF), all_trap);
}

#[test]
fn test_pointer_alignment_and_aliasing_safety() {
    // Unaligned pointer rejection
    let mut raw_buf = [0u8; 16];
    let unaligned_trit_ptr = unsafe { raw_buf.as_mut_ptr().add(1).cast::<TitanTrit16>() };
    let unaligned_u16_ptr = unsafe { raw_buf.as_mut_ptr().add(1).cast::<u16>() };
    let unaligned_kleene_ptr = unsafe { raw_buf.as_mut_ptr().add(1).cast::<TitanKleene>() };

    let valid_word = titan_trit16_zero();
    let mut aligned_word = titan_trit16_zero();
    let mut aligned_mask = 0u16;

    assert_eq!(
        unsafe { titan_trit16_try_from_planes(0, 0, unaligned_trit_ptr, &mut aligned_mask) },
        TitanError::NullPointer
    );
    assert_eq!(
        unsafe { titan_trit16_try_from_planes(0, 0, &mut aligned_word, unaligned_u16_ptr) },
        TitanError::NullPointer
    );
    assert_eq!(
        unsafe { titan_trit16_get(valid_word, 0, unaligned_kleene_ptr) },
        TitanError::NullPointer
    );
    assert_eq!(
        unsafe { titan_trit16_set(unaligned_trit_ptr, 0, TitanKleene::True as i32) },
        TitanError::NullPointer
    );

    // Aliased batch validation safety
    let mut batch = [
        titan_trit16_from_planes(0x0003, 0x0001),
        titan_trit16_from_planes(0x0000, 0x0000),
        titan_trit16_from_planes(0x00F0, 0x00F0),
    ];
    let mut count = 0usize;
    let status = unsafe {
        titan_trit16_validate_batch(
            batch.as_ptr(),
            batch.len(),
            batch.as_mut_ptr().cast::<u16>(),
            &mut count,
        )
    };
    assert_eq!(status, TitanError::Ok);
    assert_eq!(count, 2);
}

#[test]
fn test_exhaustive_rust_vs_ffi_differential() {
    let sample_planes: [(u16, u16); 6] = [
        (0x0000, 0x0000),
        (0xAAAA, 0x0000),
        (0x5555, 0xAAAA),
        (0xFFFF, 0xFFFF),
        (0x1234, 0x5678),
        (0xF0F0, 0x0F0F),
    ];

    for &(pa, na) in &sample_planes {
        let ra = Trit16::from_planes(pa, na);
        let ca = titan_trit16_from_planes(pa, na);

        assert_eq!(titan_trit16_pos(ca), ra.pos());
        assert_eq!(titan_trit16_neg(ca), ra.neg());
        assert_eq!(titan_trit16_is_valid(ca), ra.is_valid());
        assert_eq!(titan_trit16_has_traps(ca), ra.has_traps());
        assert_eq!(titan_trit16_trap_mask(ca), ra.trap_mask());

        let r_enc = ra.to_u32_interleaved();
        let c_enc = titan_trit16_to_u32_interleaved(ca);
        assert_eq!(c_enc, r_enc);

        let c_dec = titan_trit16_from_u32_interleaved(c_enc);
        assert_eq!(Trit16::from(c_dec), Trit16::from_u32_interleaved(r_enc));

        let mut strict_out = titan_trit16_zero();
        let mut mask = 0u16;
        let strict_res =
            unsafe { titan_trit16_try_from_u32_interleaved(c_enc, &mut strict_out, &mut mask) };
        match Trit16::try_from_u32_interleaved(r_enc) {
            Ok(w) => {
                assert_eq!(strict_res, TitanError::Ok);
                assert_eq!(mask, 0);
                assert_eq!(Trit16::from(strict_out), w);
            }
            Err(TritError::TrapDetected(m)) => {
                assert_eq!(strict_res, TitanError::TrapDetected);
                assert_eq!(mask, m);
            }
            Err(_) => unreachable!(),
        }

        for &(pb, nb) in &sample_planes {
            let rb = Trit16::from_planes(pb, nb);
            let cb = titan_trit16_from_planes(pb, nb);

            assert_eq!(Trit16::from(titan_trit16_and(ca, cb)), ra.and(rb));
            assert_eq!(Trit16::from(titan_trit16_or(ca, cb)), ra.or(rb));
            assert_eq!(Trit16::from(titan_trit16_not(ca)), ra.not());
            assert_eq!(Trit16::from(titan_trit16_collide(ca, cb)), ra.collide(rb));

            let mut out = titan_trit16_zero();
            let mut trap_mask = 0u16;
            let status = unsafe { titan_trit16_and_strict(ca, cb, &mut out, &mut trap_mask) };
            match ra.and_strict(rb) {
                Ok(expected) => {
                    assert_eq!(status, TitanError::Ok);
                    assert_eq!(Trit16::from(out), expected);
                }
                Err(TritError::TrapDetected(expected_mask)) => {
                    assert_eq!(status, TitanError::TrapDetected);
                    assert_eq!(trap_mask, expected_mask);
                }
                Err(_) => unreachable!(),
            }
        }
    }

    // Scalar differential across all 16 combinations
    for &a in &ALL_STATES {
        let ca = TitanKleene::from(a) as i32;
        let mut out_not = TitanKleene::Unknown;
        assert_eq!(
            unsafe { titan_kleene_not(ca, &mut out_not) },
            TitanError::Ok
        );
        assert_eq!(Kleene::from(out_not), Kleene::not(a));

        for &b in &ALL_STATES {
            let cb = TitanKleene::from(b) as i32;
            let mut out_and = TitanKleene::Unknown;
            let mut out_or = TitanKleene::Unknown;
            assert_eq!(
                unsafe { titan_kleene_and(ca, cb, &mut out_and) },
                TitanError::Ok
            );
            assert_eq!(
                unsafe { titan_kleene_or(ca, cb, &mut out_or) },
                TitanError::Ok
            );
            assert_eq!(Kleene::from(out_and), Kleene::and(a, b));
            assert_eq!(Kleene::from(out_or), Kleene::or(a, b));
        }
    }
}

#[test]
fn test_c11_and_cpp17_compilation_static_and_shared_linkage() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let c_src = format!("{manifest_dir}/tests/c_abi_test.c");

    let build_status = Command::new("cargo")
        .args(["build", "--lib", "-j", "1"])
        .current_dir(manifest_dir)
        .status()
        .expect("Failed to invoke cargo build");
    assert!(build_status.success());

    let workspace_target = format!("{manifest_dir}/../target/debug");
    let crate_target = format!("{manifest_dir}/target/debug");
    let target_dir =
        if std::path::Path::new(&format!("{workspace_target}/libtitan_trit16_ffi.a")).exists() {
            workspace_target
        } else {
            crate_target
        };
    let staticlib = format!("{target_dir}/libtitan_trit16_ffi.a");
    let c11_static_bin = format!("{target_dir}/c_abi_test_c11_static");
    let c11_shared_bin = format!("{target_dir}/c_abi_test_c11_shared");
    let cpp17_bin = format!("{target_dir}/c_abi_test_cpp17");

    // 1. Strict C11 compilation + static library linkage
    let clang_c11 = Command::new("clang")
        .args([
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Wpedantic",
            "-Wconversion",
            "-Wshadow",
            "-Wstrict-prototypes",
            "-Werror",
            &c_src,
            &staticlib,
            "-ldl",
            "-lpthread",
            "-lm",
            "-o",
            &c11_static_bin,
        ])
        .output()
        .expect("Failed to invoke clang C11 static build");
    assert!(
        clang_c11.status.success(),
        "clang C11 static failed: {}",
        String::from_utf8_lossy(&clang_c11.stderr)
    );
    let run_c11_static = Command::new(&c11_static_bin).output().unwrap();
    assert!(run_c11_static.status.success());

    // 2. Strict C11 compilation + shared library (.so) linkage
    let clang_shared = Command::new("clang")
        .args([
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Wpedantic",
            "-Werror",
            &c_src,
            "-L",
            &target_dir,
            "-ltitan_trit16_ffi",
            &format!("-Wl,-rpath,{target_dir}"),
            "-o",
            &c11_shared_bin,
        ])
        .output()
        .expect("Failed to invoke clang C11 shared build");
    assert!(
        clang_shared.status.success(),
        "clang C11 shared failed: {}",
        String::from_utf8_lossy(&clang_shared.stderr)
    );
    let run_c11_shared = Command::new(&c11_shared_bin).output().unwrap();
    assert!(run_c11_shared.status.success());

    // 3. Strict C++17 compilation + static linkage
    let clang_cpp17 = Command::new("clang++")
        .args([
            "-std=c++17",
            "-Wall",
            "-Wextra",
            "-Wpedantic",
            "-Werror",
            "-x",
            "c++",
            &c_src,
            "-x",
            "none",
            &staticlib,
            "-ldl",
            "-lpthread",
            "-lm",
            "-o",
            &cpp17_bin,
        ])
        .output()
        .expect("Failed to invoke clang++ C++17 build");
    assert!(
        clang_cpp17.status.success(),
        "clang++ C++17 failed: {}",
        String::from_utf8_lossy(&clang_cpp17.stderr)
    );
    let run_cpp17 = Command::new(&cpp17_bin).output().unwrap();
    assert!(run_cpp17.status.success());

    // 4. Verify exported symbols in libtitan_trit16_ffi.so (33 C ABI + 28 JNI)
    let sharedlib = format!("{target_dir}/libtitan_trit16_ffi.so");
    let nm_out = Command::new("nm")
        .args(["-D", "--defined-only", &sharedlib])
        .output()
        .expect("Failed to invoke nm");
    assert!(nm_out.status.success());
    let symbols = String::from_utf8_lossy(&nm_out.stdout);
    let c_sym_count = symbols
        .lines()
        .filter(|l| l.contains(" T titan_trit16_") || l.contains(" T titan_kleene_"))
        .count();
    let jni_sym_count = symbols
        .lines()
        .filter(|l| l.contains(" T Java_com_titan_trit16_Trit16Native_"))
        .count();
    assert_eq!(
        c_sym_count, 33,
        "Expected exactly 33 exported C ABI symbols"
    );
    assert_eq!(
        jni_sym_count, 28,
        "Expected exactly 28 exported JNI symbols"
    );
}

#[test]
fn test_jni_bridge_differential() {
    use core::ffi::c_void;
    use titan_trit16_ffi::jni::*;
    let env: *mut c_void = core::ptr::null_mut();
    let cls: *mut c_void = core::ptr::null_mut();

    assert_eq!(
        Java_com_titan_trit16_Trit16Native_abiVersion(env, cls),
        0x0001_0000
    );

    // Verify all-Trap (0xFFFF / 0xFFFFFFFF) round-trip through JNI register packing
    let all_trap_packed = Java_com_titan_trit16_Trit16Native_fromPlanes(env, cls, 0xFFFF, 0xFFFF);
    assert_eq!(all_trap_packed as u32, 0xFFFF_FFFF);
    assert_eq!(((all_trap_packed as u32) >> 16) & 0xFFFF, 0xFFFF);
    assert_eq!((all_trap_packed as u32) & 0xFFFF, 0xFFFF);
    assert_eq!(
        Java_com_titan_trit16_Trit16Native_isValid(env, cls, all_trap_packed),
        0
    );
    assert_eq!(
        Java_com_titan_trit16_Trit16Native_hasTraps(env, cls, all_trap_packed),
        1
    );
    assert_eq!(
        Java_com_titan_trit16_Trit16Native_trapMask(env, cls, all_trap_packed),
        0xFFFF
    );

    let strict_trap = Java_com_titan_trit16_Trit16Native_tryFromPlanes(env, cls, 0xFFFF, 0xFFFF);
    assert_eq!((strict_trap >> 48) as i32, TitanError::TrapDetected as i32);
    assert_eq!(((strict_trap >> 32) & 0xFFFF) as u16, 0xFFFF);

    // Exhaustive 16-state check through JNI
    for (idx_a, &sa) in ALL_STATES.iter().enumerate() {
        for (idx_b, &sb) in ALL_STATES.iter().enumerate() {
            let mut wa = 0i32;
            let mut wb = 0i32;
            let res_a = Java_com_titan_trit16_Trit16Native_setTrit(env, cls, wa, 0, idx_a as i32);
            let res_b = Java_com_titan_trit16_Trit16Native_setTrit(env, cls, wb, 0, idx_b as i32);
            assert_eq!((res_a >> 48) as i32, TitanError::Ok as i32);
            assert_eq!((res_b >> 48) as i32, TitanError::Ok as i32);
            wa = (res_a & 0xFFFF_FFFF) as i32;
            wb = (res_b & 0xFFFF_FFFF) as i32;

            let r_and = Java_com_titan_trit16_Trit16Native_andPlanes(env, cls, wa, wb);
            let r_or = Java_com_titan_trit16_Trit16Native_orPlanes(env, cls, wa, wb);
            let r_not = Java_com_titan_trit16_Trit16Native_notPlanes(env, cls, wa);
            let r_col = Java_com_titan_trit16_Trit16Native_collidePlanes(env, cls, wa, wb);

            let mut ra = Trit16::zero();
            let mut rb = Trit16::zero();
            ra.set(0, sa).unwrap();
            rb.set(0, sb).unwrap();

            assert_eq!(
                (Java_com_titan_trit16_Trit16Native_getTrit(env, cls, r_and, 0) & 0xFF) as i32,
                TitanKleene::from(ra.and(rb).get(0).unwrap()) as i32
            );
            assert_eq!(
                (Java_com_titan_trit16_Trit16Native_getTrit(env, cls, r_or, 0) & 0xFF) as i32,
                TitanKleene::from(ra.or(rb).get(0).unwrap()) as i32
            );
            assert_eq!(
                (Java_com_titan_trit16_Trit16Native_getTrit(env, cls, r_not, 0) & 0xFF) as i32,
                TitanKleene::from(ra.not().get(0).unwrap()) as i32
            );
            assert_eq!(
                (Java_com_titan_trit16_Trit16Native_getTrit(env, cls, r_col, 0) & 0xFF) as i32,
                TitanKleene::from(ra.collide(rb).get(0).unwrap()) as i32
            );

            // Scalar Kleene JNI
            assert_eq!(
                (Java_com_titan_trit16_Trit16Native_kleeneAnd(env, cls, idx_a as i32, idx_b as i32)
                    & 0xFF) as i32,
                TitanKleene::from(Kleene::and(sa, sb)) as i32
            );
            assert_eq!(
                (Java_com_titan_trit16_Trit16Native_kleeneOr(env, cls, idx_a as i32, idx_b as i32)
                    & 0xFF) as i32,
                TitanKleene::from(Kleene::or(sa, sb)) as i32
            );
        }
    }

    // Telemetry check via JNI
    assert_eq!(
        (Java_com_titan_trit16_Trit16Native_kleeneEvaluateTelemetry(
            env,
            cls,
            TitanKleene::True as i32,
            TitanKleene::True as i32,
            TitanKleene::True as i32
        ) & 0xFF) as i32,
        TitanKleene::True as i32
    );
    assert_eq!(
        (Java_com_titan_trit16_Trit16Native_kleeneEvaluateTelemetry(
            env,
            cls,
            TitanKleene::Trap as i32,
            TitanKleene::False as i32,
            TitanKleene::True as i32
        ) & 0xFF) as i32,
        TitanKleene::Trap as i32
    );
}
