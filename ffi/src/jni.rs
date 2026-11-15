//! Zero-allocation JNI bridge (`com.titan.trit16.Trit16Native`).
//!
//! All JNI entry points operate on register-packed primitive values (`jint` / `jlong`)
//! without dereferencing `JNIEnv*` or `jclass`, eliminating JNI pointer hazards
//! and heap allocations across the language boundary.
//!
//! ### Packed `jlong` (`i64`) Result Envelope for Fallible Operations
//! - Bits `0..=31` (`u32`): Result payload (packed `(pos << 16) | neg` or `TitanKleene` code)
//! - Bits `32..=47` (`u16`): Trap bitmask (`0x0000..=0xFFFF`) when status is `TrapDetected`
//! - Bits `48..=55` (`u8`): `TitanError` discriminant (`0 = Ok`, `2 = TrapDetected`, `3 = InvalidIndex`, `4 = InvalidValue`, `255 = Panic`)

use crate::{TitanError, TitanKleene, TITAN_TRIT16_ABI_VERSION_PACKED};
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use titan_trit16_core::{Kleene, KleeneError, Trit16, TritError};

#[inline]
const fn pack_trit16(w: Trit16) -> i32 {
    (((w.pos() as u32) << 16) | (w.neg() as u32)) as i32
}

#[inline]
const fn unpack_trit16(raw_planes: i32) -> Trit16 {
    let u = raw_planes as u32;
    let pos = ((u >> 16) & 0xFFFF) as u16;
    let neg = (u & 0xFFFF) as u16;
    Trit16::from_planes(pos, neg)
}

#[inline]
const fn pack_ok(payload: u32) -> i64 {
    payload as u64 as i64
}

#[inline]
const fn pack_err(err: TitanError, trap_mask: u16) -> i64 {
    (((err as u64) << 48) | ((trap_mask as u64) << 32)) as i64
}

#[inline]
fn pack_trit_result(res: Result<Trit16, TritError>) -> i64 {
    match res {
        Ok(w) => pack_ok(pack_trit16(w) as u32),
        Err(TritError::TrapDetected(mask)) => pack_err(TitanError::TrapDetected, mask),
        Err(TritError::InvalidIndex(_)) => pack_err(TitanError::InvalidIndex, 0),
    }
}

#[inline]
fn parse_kleene(code: i32) -> Result<Kleene, TitanError> {
    match code {
        0 => Ok(Kleene::Unknown),
        1 => Ok(Kleene::True),
        2 => Ok(Kleene::False),
        3 => Ok(Kleene::Trap),
        _ => Err(TitanError::InvalidValue),
    }
}

#[inline]
fn with_jni_guard<F>(f: F) -> i64
where
    F: FnOnce() -> i64,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(val) => val,
        Err(_) => pack_err(TitanError::Panic, 0),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_abiVersion(
    _env: *mut c_void,
    _class: *mut c_void,
) -> i32 {
    TITAN_TRIT16_ABI_VERSION_PACKED as i32
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_fromPlanes(
    _env: *mut c_void,
    _class: *mut c_void,
    pos: i32,
    neg: i32,
) -> i32 {
    if (pos & !0xFFFF) != 0 || (neg & !0xFFFF) != 0 {
        // Clamp/mask happens in caller; lossless constructor masks low 16 bits
    }
    pack_trit16(Trit16::from_planes(
        (pos & 0xFFFF) as u16,
        (neg & 0xFFFF) as u16,
    ))
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_tryFromPlanes(
    _env: *mut c_void,
    _class: *mut c_void,
    pos: i32,
    neg: i32,
) -> i64 {
    with_jni_guard(|| {
        if (pos & !0xFFFF) != 0 || (neg & !0xFFFF) != 0 {
            return pack_err(TitanError::InvalidValue, 0);
        }
        pack_trit_result(Trit16::try_from_planes(pos as u16, neg as u16))
    })
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_isValid(
    _env: *mut c_void,
    _class: *mut c_void,
    raw_planes: i32,
) -> u8 {
    u8::from(unpack_trit16(raw_planes).is_valid())
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_hasTraps(
    _env: *mut c_void,
    _class: *mut c_void,
    raw_planes: i32,
) -> u8 {
    u8::from(unpack_trit16(raw_planes).has_traps())
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_trapMask(
    _env: *mut c_void,
    _class: *mut c_void,
    raw_planes: i32,
) -> i32 {
    unpack_trit16(raw_planes).trap_mask() as i32
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_getTrit(
    _env: *mut c_void,
    _class: *mut c_void,
    raw_planes: i32,
    index: i32,
) -> i64 {
    with_jni_guard(|| {
        if !(0..=255).contains(&index) {
            return pack_err(TitanError::InvalidIndex, 0);
        }
        match unpack_trit16(raw_planes).get(index as u8) {
            Ok(k) => pack_ok(TitanKleene::from(k) as u32),
            Err(TritError::InvalidIndex(_)) => pack_err(TitanError::InvalidIndex, 0),
            Err(TritError::TrapDetected(m)) => pack_err(TitanError::TrapDetected, m),
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_setTrit(
    _env: *mut c_void,
    _class: *mut c_void,
    raw_planes: i32,
    index: i32,
    kleene_code: i32,
) -> i64 {
    with_jni_guard(|| {
        if !(0..=255).contains(&index) {
            return pack_err(TitanError::InvalidIndex, 0);
        }
        let k = match parse_kleene(kleene_code) {
            Ok(v) => v,
            Err(e) => return pack_err(e, 0),
        };
        let mut w = unpack_trit16(raw_planes);
        match w.set(index as u8, k) {
            Ok(()) => pack_ok(pack_trit16(w) as u32),
            Err(TritError::InvalidIndex(_)) => pack_err(TitanError::InvalidIndex, 0),
            Err(TritError::TrapDetected(m)) => pack_err(TitanError::TrapDetected, m),
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_setTritStrict(
    _env: *mut c_void,
    _class: *mut c_void,
    raw_planes: i32,
    index: i32,
    kleene_code: i32,
) -> i64 {
    with_jni_guard(|| {
        if !(0..=255).contains(&index) {
            return pack_err(TitanError::InvalidIndex, 0);
        }
        let k = match parse_kleene(kleene_code) {
            Ok(v) => v,
            Err(e) => return pack_err(e, 0),
        };
        let mut w = unpack_trit16(raw_planes);
        match w.set_strict(index as u8, k) {
            Ok(()) => pack_ok(pack_trit16(w) as u32),
            Err(TritError::InvalidIndex(_)) => pack_err(TitanError::InvalidIndex, 0),
            Err(TritError::TrapDetected(m)) => pack_err(TitanError::TrapDetected, m),
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_andPlanes(
    _env: *mut c_void,
    _class: *mut c_void,
    a_planes: i32,
    b_planes: i32,
) -> i32 {
    pack_trit16(unpack_trit16(a_planes).and(unpack_trit16(b_planes)))
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_orPlanes(
    _env: *mut c_void,
    _class: *mut c_void,
    a_planes: i32,
    b_planes: i32,
) -> i32 {
    pack_trit16(unpack_trit16(a_planes).or(unpack_trit16(b_planes)))
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_notPlanes(
    _env: *mut c_void,
    _class: *mut c_void,
    a_planes: i32,
) -> i32 {
    pack_trit16(unpack_trit16(a_planes).not())
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_collidePlanes(
    _env: *mut c_void,
    _class: *mut c_void,
    a_planes: i32,
    b_planes: i32,
) -> i32 {
    pack_trit16(unpack_trit16(a_planes).collide(unpack_trit16(b_planes)))
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_andStrict(
    _env: *mut c_void,
    _class: *mut c_void,
    a_planes: i32,
    b_planes: i32,
) -> i64 {
    with_jni_guard(|| pack_trit_result(unpack_trit16(a_planes).and_strict(unpack_trit16(b_planes))))
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_orStrict(
    _env: *mut c_void,
    _class: *mut c_void,
    a_planes: i32,
    b_planes: i32,
) -> i64 {
    with_jni_guard(|| pack_trit_result(unpack_trit16(a_planes).or_strict(unpack_trit16(b_planes))))
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_notStrict(
    _env: *mut c_void,
    _class: *mut c_void,
    a_planes: i32,
) -> i64 {
    with_jni_guard(|| pack_trit_result(unpack_trit16(a_planes).not_strict()))
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_collideStrict(
    _env: *mut c_void,
    _class: *mut c_void,
    a_planes: i32,
    b_planes: i32,
) -> i64 {
    with_jni_guard(|| {
        pack_trit_result(unpack_trit16(a_planes).collide_strict(unpack_trit16(b_planes)))
    })
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_toU32Interleaved(
    _env: *mut c_void,
    _class: *mut c_void,
    raw_planes: i32,
) -> i32 {
    unpack_trit16(raw_planes).to_u32_interleaved() as i32
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_fromU32Interleaved(
    _env: *mut c_void,
    _class: *mut c_void,
    interleaved: i32,
) -> i32 {
    pack_trit16(Trit16::from_u32_interleaved(interleaved as u32))
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_tryFromU32Interleaved(
    _env: *mut c_void,
    _class: *mut c_void,
    interleaved: i32,
) -> i64 {
    with_jni_guard(|| pack_trit_result(Trit16::try_from_u32_interleaved(interleaved as u32)))
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_kleeneFromI8(
    _env: *mut c_void,
    _class: *mut c_void,
    val: i8,
) -> i32 {
    TitanKleene::from(Kleene::from_i8(val)) as i32
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_kleeneTryFromI8(
    _env: *mut c_void,
    _class: *mut c_void,
    val: i8,
) -> i64 {
    with_jni_guard(|| match Kleene::try_from_i8(val) {
        Ok(k) => pack_ok(TitanKleene::from(k) as u32),
        Err(KleeneError::InvalidInt(_)) | Err(KleeneError::TrapValue) => {
            pack_err(TitanError::InvalidValue, 0)
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_kleeneToI8(
    _env: *mut c_void,
    _class: *mut c_void,
    code: i32,
) -> i64 {
    with_jni_guard(|| match parse_kleene(code) {
        Ok(k) => pack_ok((k.to_i8() as u8) as u32),
        Err(e) => pack_err(e, 0),
    })
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_kleeneTryToI8(
    _env: *mut c_void,
    _class: *mut c_void,
    code: i32,
) -> i64 {
    with_jni_guard(|| match parse_kleene(code) {
        Ok(k) => match k.try_to_i8() {
            Ok(v) => pack_ok((v as u8) as u32),
            Err(KleeneError::TrapValue) => pack_err(TitanError::TrapDetected, 1),
            Err(KleeneError::InvalidInt(_)) => pack_err(TitanError::InvalidValue, 0),
        },
        Err(e) => pack_err(e, 0),
    })
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_kleeneAnd(
    _env: *mut c_void,
    _class: *mut c_void,
    a_code: i32,
    b_code: i32,
) -> i64 {
    with_jni_guard(|| {
        let a = match parse_kleene(a_code) {
            Ok(v) => v,
            Err(e) => return pack_err(e, 0),
        };
        let b = match parse_kleene(b_code) {
            Ok(v) => v,
            Err(e) => return pack_err(e, 0),
        };
        pack_ok(TitanKleene::from(Kleene::and(a, b)) as u32)
    })
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_kleeneOr(
    _env: *mut c_void,
    _class: *mut c_void,
    a_code: i32,
    b_code: i32,
) -> i64 {
    with_jni_guard(|| {
        let a = match parse_kleene(a_code) {
            Ok(v) => v,
            Err(e) => return pack_err(e, 0),
        };
        let b = match parse_kleene(b_code) {
            Ok(v) => v,
            Err(e) => return pack_err(e, 0),
        };
        pack_ok(TitanKleene::from(Kleene::or(a, b)) as u32)
    })
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_kleeneNot(
    _env: *mut c_void,
    _class: *mut c_void,
    a_code: i32,
) -> i64 {
    with_jni_guard(|| {
        let a = match parse_kleene(a_code) {
            Ok(v) => v,
            Err(e) => return pack_err(e, 0),
        };
        pack_ok(TitanKleene::from(Kleene::not(a)) as u32)
    })
}

#[no_mangle]
pub extern "system" fn Java_com_titan_trit16_Trit16Native_kleeneEvaluateTelemetry(
    _env: *mut c_void,
    _class: *mut c_void,
    v_code: i32,
    t_code: i32,
    p_code: i32,
) -> i64 {
    with_jni_guard(|| {
        let v = match parse_kleene(v_code) {
            Ok(val) => val,
            Err(e) => return pack_err(e, 0),
        };
        let t = match parse_kleene(t_code) {
            Ok(val) => val,
            Err(e) => return pack_err(e, 0),
        };
        let p = match parse_kleene(p_code) {
            Ok(val) => val,
            Err(e) => return pack_err(e, 0),
        };
        pack_ok(TitanKleene::from(Kleene::evaluate_telemetry(v, t, p)) as u32)
    })
}
