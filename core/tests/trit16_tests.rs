//! Comprehensive unit, truth-table, differential, and conformance tests for `titan_trit16_core`.

use titan_trit16_core::{Kleene, KleeneError, Trit16, TritError};

const ALL_STATES: [Kleene; 4] = [Kleene::Unknown, Kleene::True, Kleene::False, Kleene::Trap];

/// Helper to construct a single-trit `Trit16` at bit 0 from a `Kleene` state.
fn single_trit(state: Kleene) -> Trit16 {
    let mut w = Trit16::zero();
    w.set(0, state).unwrap();
    w
}

#[test]
fn test_constructors_and_validation() {
    let z = Trit16::zero();
    assert_eq!(z.pos(), 0);
    assert_eq!(z.neg(), 0);
    assert!(z.is_valid());
    assert!(!z.has_traps());
    assert_eq!(z.trap_mask(), 0);

    // Lossless construction preserves traps
    let t = Trit16::from_planes(0b0110, 0b0010);
    assert!(!t.is_valid());
    assert!(t.has_traps());
    assert_eq!(t.trap_mask(), 0b0010);
    assert_eq!(t.pos(), 0b0110);
    assert_eq!(t.neg(), 0b0010);

    // Strict constructor rejects traps with full mask
    assert_eq!(
        Trit16::try_from_planes(0b1010, 0b0011),
        Err(TritError::TrapDetected(0b0010))
    );

    // Strict constructor succeeds when disjoint
    let valid = Trit16::try_from_planes(0x5555, 0xAAAA).unwrap();
    assert!(valid.is_valid());
    assert_eq!(valid.trap_mask(), 0);
}

#[test]
fn test_per_trit_get_set_and_bounds() {
    let mut t = Trit16::zero();
    t.set(0, Kleene::True).unwrap();
    t.set(1, Kleene::False).unwrap();
    t.set(2, Kleene::Unknown).unwrap();
    t.set(3, Kleene::Trap).unwrap();

    assert_eq!(t.get(0), Ok(Kleene::True));
    assert_eq!(t.get(1), Ok(Kleene::False));
    assert_eq!(t.get(2), Ok(Kleene::Unknown));
    assert_eq!(t.get(3), Ok(Kleene::Trap));
    assert_eq!(t.trap_mask(), 1 << 3);

    // Overwriting a Trap with a valid state clears the trap bit
    t.set_strict(3, Kleene::True).unwrap();
    assert!(t.is_valid());
    assert_eq!(t.get(3), Ok(Kleene::True));

    // Out-of-range index errors
    assert_eq!(t.get(16), Err(TritError::InvalidIndex(16)));
    assert_eq!(t.set(16, Kleene::True), Err(TritError::InvalidIndex(16)));
    assert_eq!(
        t.set_strict(16, Kleene::True),
        Err(TritError::InvalidIndex(16))
    );
    assert_eq!(
        t.set_strict(16, Kleene::Trap),
        Err(TritError::InvalidIndex(16))
    );

    // Strict setter rejects Trap at valid index
    assert_eq!(
        t.set_strict(5, Kleene::Trap),
        Err(TritError::TrapDetected(1 << 5))
    );
}

#[test]
fn test_scalar_kleene_conversions() {
    assert_eq!(Kleene::from_i8(1), Kleene::True);
    assert_eq!(Kleene::from_i8(-1), Kleene::False);
    assert_eq!(Kleene::from_i8(0), Kleene::Unknown);
    assert_eq!(Kleene::from_i8(2), Kleene::Trap);
    assert_eq!(Kleene::from_i8(-99), Kleene::Trap);

    assert_eq!(Kleene::try_from_i8(1), Ok(Kleene::True));
    assert_eq!(Kleene::try_from_i8(-1), Ok(Kleene::False));
    assert_eq!(Kleene::try_from_i8(0), Ok(Kleene::Unknown));
    assert_eq!(Kleene::try_from_i8(2), Err(KleeneError::InvalidInt(2)));

    assert_eq!(Kleene::True.to_i8(), 1);
    assert_eq!(Kleene::False.to_i8(), -1);
    assert_eq!(Kleene::Unknown.to_i8(), 0);
    assert_eq!(Kleene::Trap.to_i8(), 0);

    assert_eq!(Kleene::True.try_to_i8(), Ok(1));
    assert_eq!(Kleene::False.try_to_i8(), Ok(-1));
    assert_eq!(Kleene::Unknown.try_to_i8(), Ok(0));
    assert_eq!(Kleene::Trap.try_to_i8(), Err(KleeneError::TrapValue));
}

#[test]
fn test_exhaustive_scalar_truth_tables() {
    use Kleene::{False as F, Trap as X, True as T, Unknown as U};

    // Independent 4x4 truth table for scalar Kleene::and (trap-dominant)
    // Order: [Unknown, True, False, Trap]
    let expected_scalar_and: [[Kleene; 4]; 4] = [
        // B:  U  T  F  X
        /* U */ [U, U, F, X],
        /* T */ [U, T, F, X],
        /* F */ [F, F, F, X],
        /* X */ [X, X, X, X],
    ];

    // Independent 4x4 truth table for scalar Kleene::or (trap-dominant)
    let expected_scalar_or: [[Kleene; 4]; 4] = [
        // B:  U  T  F  X
        /* U */ [U, T, U, X],
        /* T */ [T, T, T, X],
        /* F */ [U, T, F, X],
        /* X */ [X, X, X, X],
    ];

    for (i, &a) in ALL_STATES.iter().enumerate() {
        for (j, &b) in ALL_STATES.iter().enumerate() {
            assert_eq!(
                Kleene::and(a, b),
                expected_scalar_and[i][j],
                "Scalar AND mismatch for ({a:?}, {b:?})"
            );
            assert_eq!(
                Kleene::or(a, b),
                expected_scalar_or[i][j],
                "Scalar OR mismatch for ({a:?}, {b:?})"
            );
        }
    }

    assert_eq!(Kleene::not(U), U);
    assert_eq!(Kleene::not(T), F);
    assert_eq!(Kleene::not(F), T);
    assert_eq!(Kleene::not(X), X);
}

#[test]
fn test_exhaustive_bitplane_truth_tables() {
    use Kleene::{False as F, Trap as X, True as T, Unknown as U};

    // Independent 4x4 truth table for bit-plane Trit16::and
    // Formula: pos = a.pos & b.pos, neg = a.neg | b.neg
    // Order: [Unknown (0,0), True (1,0), False (0,1), Trap (1,1)]
    let expected_bitplane_and: [[Kleene; 4]; 4] = [
        // B:  U  T  F  X
        /* U */ [U, U, F, F],
        /* T */ [U, T, F, X],
        /* F */ [F, F, F, F],
        /* X */ [F, X, F, X],
    ];

    // Independent 4x4 truth table for bit-plane Trit16::or
    // Formula: pos = a.pos | b.pos, neg = a.neg & b.neg
    let expected_bitplane_or: [[Kleene; 4]; 4] = [
        // B:  U  T  F  X
        /* U */ [U, T, U, T],
        /* T */ [T, T, T, T],
        /* F */ [U, T, F, X],
        /* X */ [T, T, X, X],
    ];

    // Independent 4x4 truth table for bit-plane Trit16::collide
    // Formula: agree_pos & !conflict, agree_neg & !conflict
    let expected_bitplane_collide: [[Kleene; 4]; 4] = [
        // B:  U  T  F  X
        /* U */ [U, U, U, U],
        /* T */ [U, T, U, U],
        /* F */ [U, U, F, U],
        /* X */ [U, U, U, U],
    ];

    for (i, &a) in ALL_STATES.iter().enumerate() {
        let wa = single_trit(a);
        for (j, &b) in ALL_STATES.iter().enumerate() {
            let wb = single_trit(b);

            assert_eq!(
                wa.and(wb).get(0).unwrap(),
                expected_bitplane_and[i][j],
                "Bit-plane AND mismatch for ({a:?}, {b:?})"
            );
            assert_eq!(
                wa.or(wb).get(0).unwrap(),
                expected_bitplane_or[i][j],
                "Bit-plane OR mismatch for ({a:?}, {b:?})"
            );
            assert_eq!(
                wa.collide(wb).get(0).unwrap(),
                expected_bitplane_collide[i][j],
                "Bit-plane COLLIDE mismatch for ({a:?}, {b:?})"
            );
        }

        // Unary NOT matches scalar Kleene::not for all 4 states
        assert_eq!(wa.not().get(0).unwrap(), Kleene::not(a));
    }
}

#[test]
fn test_exhaustive_differential_scalar_vs_bitplane() {
    use Kleene::{False as F, Trap as X, True as T, Unknown as U};

    let mut and_divergences = Vec::new();
    let mut or_divergences = Vec::new();

    for &a in &ALL_STATES {
        for &b in &ALL_STATES {
            let wa = single_trit(a);
            let wb = single_trit(b);

            let s_and = Kleene::and(a, b);
            let b_and = wa.and(wb).get(0).unwrap();

            let s_or = Kleene::or(a, b);
            let b_or = wa.or(wb).get(0).unwrap();

            // When neither operand is Trap (the 9 valid Kleene combinations),
            // scalar and bit-plane MUST be 100% identical.
            if a != X && b != X {
                assert_eq!(s_and, b_and, "Trap-free AND must match for ({a:?}, {b:?})");
                assert_eq!(s_or, b_or, "Trap-free OR must match for ({a:?}, {b:?})");
            }

            if s_and != b_and {
                and_divergences.push((a, b, s_and, b_and));
            }
            if s_or != b_or {
                or_divergences.push((a, b, s_or, b_or));
            }
        }
    }

    // Exactly 4 pairs diverge on AND: Trap with False or Unknown yields False in bit-plane vs Trap in scalar
    assert_eq!(
        and_divergences,
        vec![(U, X, X, F), (F, X, X, F), (X, U, X, F), (X, F, X, F),]
    );

    // Exactly 4 pairs diverge on OR: Trap with True or Unknown yields True in bit-plane vs Trap in scalar
    assert_eq!(
        or_divergences,
        vec![(U, X, X, T), (T, X, X, T), (X, U, X, T), (X, T, X, T),]
    );
}

#[test]
fn test_regression_edge_cases() {
    let trap = single_trit(Kleene::Trap);
    let unknown = single_trit(Kleene::Unknown);
    let tr = single_trit(Kleene::True);
    let fa = single_trit(Kleene::False);

    // 1. Trap AND Unknown -> False in bit-plane, Trap in scalar
    assert_eq!(trap.and(unknown).get(0).unwrap(), Kleene::False);
    assert_eq!(Kleene::and(Kleene::Trap, Kleene::Unknown), Kleene::Trap);

    // 2. Trap OR True -> True in bit-plane, Trap in scalar
    assert_eq!(trap.or(tr).get(0).unwrap(), Kleene::True);
    assert_eq!(Kleene::or(Kleene::Trap, Kleene::True), Kleene::Trap);

    // 3. NOT preserves Trap in both
    assert_eq!(trap.not().get(0).unwrap(), Kleene::Trap);
    assert_eq!(Kleene::not(Kleene::Trap), Kleene::Trap);

    // 4. COLLIDE keeps agreement on True/False, collapses conflict and Trap (even Trap+Trap) to Unknown
    assert_eq!(tr.collide(tr).get(0).unwrap(), Kleene::True);
    assert_eq!(fa.collide(fa).get(0).unwrap(), Kleene::False);
    assert_eq!(tr.collide(fa).get(0).unwrap(), Kleene::Unknown);
    assert_eq!(trap.collide(trap).get(0).unwrap(), Kleene::Unknown);
}

#[test]
fn test_strict_operations_reject_traps_with_union_mask() {
    let a = Trit16::from_planes(0b0001, 0b0001); // trap at bit 0
    let b = Trit16::from_planes(0b0100, 0b0100); // trap at bit 2
    let clean = Trit16::from_planes(0b0010, 0b1000);

    // Union of trap masks when both have traps
    assert_eq!(a.and_strict(b), Err(TritError::TrapDetected(0b0101)));
    assert_eq!(a.or_strict(b), Err(TritError::TrapDetected(0b0101)));
    assert_eq!(a.collide_strict(b), Err(TritError::TrapDetected(0b0101)));

    // Single operand with trap
    assert_eq!(a.and_strict(clean), Err(TritError::TrapDetected(0b0001)));
    assert_eq!(clean.or_strict(b), Err(TritError::TrapDetected(0b0100)));
    assert_eq!(a.not_strict(), Err(TritError::TrapDetected(0b0001)));

    // Clean operands succeed and match non-strict results
    let clean2 = Trit16::from_planes(0b0011, 0b0100);
    assert_eq!(clean.and_strict(clean2), Ok(clean.and(clean2)));
    assert_eq!(clean.or_strict(clean2), Ok(clean.or(clean2)));
    assert_eq!(clean.not_strict(), Ok(clean.not()));
    assert_eq!(clean.collide_strict(clean2), Ok(clean.collide(clean2)));
}

#[test]
fn test_mixed_state_packed_16_trit_word_and_serialization() {
    // Pack all 16 combinations (a, b) into trits 0..15 of two Trit16 words!
    let mut word_a = Trit16::zero();
    let mut word_b = Trit16::zero();

    for (i, &sa) in ALL_STATES.iter().enumerate() {
        for (j, &sb) in ALL_STATES.iter().enumerate() {
            let idx = (i * 4 + j) as u8;
            word_a.set(idx, sa).unwrap();
            word_b.set(idx, sb).unwrap();
        }
    }

    // Verify lossless round-trip serialization preserves all 16 trits including traps
    let enc_a = word_a.to_u32_interleaved();
    let enc_b = word_b.to_u32_interleaved();
    assert_eq!(Trit16::from_u32_interleaved(enc_a), word_a);
    assert_eq!(Trit16::from_u32_interleaved(enc_b), word_b);

    // Verify strict deserialization rejects words with traps and reports the full mask
    assert_eq!(
        Trit16::try_from_u32_interleaved(enc_a),
        Err(TritError::TrapDetected(word_a.trap_mask()))
    );
    assert_eq!(
        Trit16::try_from_u32_interleaved(enc_b),
        Err(TritError::TrapDetected(word_b.trap_mask()))
    );

    // Verify parallel operations across all 16 lanes simultaneously match single-trit operations
    let res_and = word_a.and(word_b);
    let res_or = word_a.or(word_b);
    let res_not = word_a.not();
    let res_col = word_a.collide(word_b);

    for (i, &sa) in ALL_STATES.iter().enumerate() {
        for (j, &sb) in ALL_STATES.iter().enumerate() {
            let idx = (i * 4 + j) as u8;
            let wa = single_trit(sa);
            let wb = single_trit(sb);

            assert_eq!(res_and.get(idx).unwrap(), wa.and(wb).get(0).unwrap());
            assert_eq!(res_or.get(idx).unwrap(), wa.or(wb).get(0).unwrap());
            assert_eq!(res_not.get(idx).unwrap(), wa.not().get(0).unwrap());
            assert_eq!(res_col.get(idx).unwrap(), wa.collide(wb).get(0).unwrap());
        }
    }

    // COLLIDE output is always trap-free, so try_from_u32_interleaved must succeed on it
    let enc_col = res_col.to_u32_interleaved();
    assert_eq!(Trit16::try_from_u32_interleaved(enc_col), Ok(res_col));
}

#[test]
fn test_original_reference_vector_conformance() {
    // Exact test vector from ~/Titan_Unified_Core/kleene_logic.rs lines 215-235
    let stream_a = Trit16::from_planes(0xAAAA, 0x0000);
    let stream_b = Trit16::from_planes(0x5555, 0xAAAA);

    assert!(stream_a.is_valid());
    assert!(stream_b.is_valid());

    let and_res = stream_a.and(stream_b);
    let or_res = stream_a.or(stream_b);
    let not_res = stream_a.not();
    let col_res = stream_a.collide(stream_b);

    assert_eq!((and_res.pos(), and_res.neg()), (0x0000, 0xAAAA));
    assert_eq!((or_res.pos(), or_res.neg()), (0xFFFF, 0x0000));
    assert_eq!((not_res.pos(), not_res.neg()), (0x0000, 0xAAAA));
    assert_eq!((col_res.pos(), col_res.neg()), (0x0000, 0x0000));

    let packed_calldata = col_res.to_u32_interleaved();
    assert_eq!(packed_calldata, 0x0000_0000);
    let unpacked = Trit16::from_u32_interleaved(packed_calldata);
    assert_eq!(col_res, unpacked);
}

#[test]
fn test_exhaustive_u16_swar_serialization_equivalence() {
    // Independent 16-iteration reference loop for interleaved packing
    fn reference_to_u32_interleaved(pos: u16, neg: u16) -> u32 {
        let mut out: u32 = 0;
        for i in 0..16 {
            let mask = 1u16 << i;
            let p = (pos & mask) != 0;
            let n = (neg & mask) != 0;
            let code: u32 = match (p, n) {
                (false, false) => 0b00,
                (true, false) => 0b01,
                (false, true) => 0b10,
                (true, true) => 0b11,
            };
            out |= code << (i * 2);
        }
        out
    }

    // Exhaustively verify all 65,536 plane values across positive, negative, and mixed trap patterns
    for plane in 0..=u16::MAX {
        let neg = plane.wrapping_mul(0x9E37).rotate_left(5);
        let w = Trit16::from_planes(plane, neg);
        let packed = w.to_u32_interleaved();
        assert_eq!(packed, reference_to_u32_interleaved(plane, neg));
        let unpacked = Trit16::from_u32_interleaved(packed);
        assert_eq!(unpacked, w);
    }
}

#[test]
fn test_evaluate_telemetry_and_display_formatting() {
    use Kleene::{False as F, Trap as X, True as T, Unknown as U};

    // Nominal, degraded, failure, and trap telemetry scenarios from kleene_logic.rs
    assert_eq!(Kleene::evaluate_telemetry(T, T, T), T);
    assert_eq!(Kleene::evaluate_telemetry(T, U, T), U);
    assert_eq!(Kleene::evaluate_telemetry(T, T, F), F);
    assert_eq!(Kleene::evaluate_telemetry(F, X, T), X); // Trap dominates False in scalar telemetry

    assert_eq!(format!("{T}"), "+1");
    assert_eq!(format!("{F}"), "-1");
    assert_eq!(format!("{U}"), "0");
    assert_eq!(format!("{X}"), "TRAP");

    let mut w = Trit16::zero();
    w.set(15, T).unwrap();
    w.set(14, F).unwrap();
    w.set(13, U).unwrap();
    w.set(0, X).unwrap();
    assert_eq!(format!("{w}"), "[+-0000000000000!]");
}
