//! Titan Trit16 SDK — Stable C ABI Layer (`titan_trit16_ffi`)
//!
//! Wraps `titan_trit16_core` without duplicating or altering any logic.
//! Every `extern "C"` function prevents panics from crossing the FFI boundary
//! and validates pointer non-nullness, alignment, enum discriminants, and lengths.

#![deny(unsafe_op_in_unsafe_fn)]

pub mod jni;

use std::ffi::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use titan_trit16_core::{Kleene, KleeneError, Trit16, TritError};

/// Packed ABI version: `1.0.0` (`0x0001_0000`).
pub const TITAN_TRIT16_ABI_VERSION_MAJOR: u32 = 1;
pub const TITAN_TRIT16_ABI_VERSION_MINOR: u32 = 0;
pub const TITAN_TRIT16_ABI_VERSION_PATCH: u32 = 0;
pub const TITAN_TRIT16_ABI_VERSION_PACKED: u32 = (TITAN_TRIT16_ABI_VERSION_MAJOR << 16)
    | (TITAN_TRIT16_ABI_VERSION_MINOR << 8)
    | TITAN_TRIT16_ABI_VERSION_PATCH;

static ABI_VERSION_CSTR: &[u8] = b"1.0.0\0";

static MSG_OK: &[u8] = b"OK\0";
static MSG_NULL_POINTER: &[u8] = b"Null or misaligned pointer argument\0";
static MSG_TRAP_DETECTED: &[u8] = b"Hardware trap bits detected\0";
static MSG_INVALID_INDEX: &[u8] = b"Trit index out of range (must be 0..=15)\0";
static MSG_INVALID_VALUE: &[u8] = b"Invalid Kleene discriminant or integer value\0";
static MSG_INVALID_LENGTH: &[u8] = b"Invalid batch length\0";
static MSG_PANIC: &[u8] = b"Internal panic caught at FFI boundary\0";
static MSG_UNKNOWN_ERROR: &[u8] = b"Unknown error code\0";

/// C-compatible 16-trit word matching `titan16_word_t.plane` in `rules.h`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TitanTrit16 {
    pub neg: u16,
    pub pos: u16,
}

impl From<Trit16> for TitanTrit16 {
    #[inline]
    fn from(w: Trit16) -> Self {
        Self {
            neg: w.neg(),
            pos: w.pos(),
        }
    }
}

impl From<TitanTrit16> for Trit16 {
    #[inline]
    fn from(w: TitanTrit16) -> Self {
        Self::from_planes(w.pos, w.neg)
    }
}

/// C-compatible 4-state trit enum (`0 = Unknown`, `1 = True`, `2 = False`, `3 = Trap`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitanKleene {
    Unknown = 0,
    True = 1,
    False = 2,
    Trap = 3,
}

impl From<Kleene> for TitanKleene {
    #[inline]
    fn from(k: Kleene) -> Self {
        match k {
            Kleene::Unknown => Self::Unknown,
            Kleene::True => Self::True,
            Kleene::False => Self::False,
            Kleene::Trap => Self::Trap,
        }
    }
}

impl From<TitanKleene> for Kleene {
    #[inline]
    fn from(k: TitanKleene) -> Self {
        match k {
            TitanKleene::Unknown => Self::Unknown,
            TitanKleene::True => Self::True,
            TitanKleene::False => Self::False,
            TitanKleene::Trap => Self::Trap,
        }
    }
}

/// C-compatible status and error codes.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitanError {
    Ok = 0,
    NullPointer = 1,
    TrapDetected = 2,
    InvalidIndex = 3,
    InvalidValue = 4,
    InvalidLength = 5,
    Panic = 255,
}

// Compile-time layout assertions for C ABI stability
const _: [(); 4] = [(); core::mem::size_of::<TitanTrit16>()];
const _: [(); 2] = [(); core::mem::align_of::<TitanTrit16>()];
const _: [(); 0] = [(); core::mem::offset_of!(TitanTrit16, neg)];
const _: [(); 2] = [(); core::mem::offset_of!(TitanTrit16, pos)];
const _: [(); 4] = [(); core::mem::size_of::<TitanKleene>()];
const _: [(); 4] = [(); core::mem::align_of::<TitanKleene>()];
const _: [(); 4] = [(); core::mem::size_of::<TitanError>()];
const _: [(); 4] = [(); core::mem::align_of::<TitanError>()];

#[inline]
fn is_valid_ptr<T>(ptr: *const T) -> bool {
    !ptr.is_null() && ptr.is_aligned()
}

#[inline]
fn is_valid_opt_ptr<T>(ptr: *const T) -> bool {
    ptr.is_null() || ptr.is_aligned()
}

#[inline]
fn parse_kleene_discriminant(val: i32) -> Result<Kleene, TitanError> {
    match val {
        0 => Ok(Kleene::Unknown),
        1 => Ok(Kleene::True),
        2 => Ok(Kleene::False),
        3 => Ok(Kleene::Trap),
        _ => Err(TitanError::InvalidValue),
    }
}

#[inline]
unsafe fn write_optional_u16(ptr: *mut u16, val: u16) {
    if !ptr.is_null() && ptr.is_aligned() {
        unsafe {
            ptr.write(val);
        }
    }
}

#[inline]
fn map_trit_error(err: TritError, out_trap_mask: *mut u16) -> TitanError {
    match err {
        TritError::TrapDetected(mask) => {
            unsafe {
                write_optional_u16(out_trap_mask, mask);
            }
            TitanError::TrapDetected
        }
        TritError::InvalidIndex(_) => TitanError::InvalidIndex,
    }
}

#[inline]
fn with_ffi_guard<F>(f: F) -> TitanError
where
    F: FnOnce() -> TitanError,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(code) => code,
        Err(_) => TitanError::Panic,
    }
}

// ============================================================================
// 3. ABI Version & Diagnostics
// ============================================================================

#[no_mangle]
pub extern "C" fn titan_trit16_abi_version() -> u32 {
    TITAN_TRIT16_ABI_VERSION_PACKED
}

#[no_mangle]
pub extern "C" fn titan_trit16_abi_version_string() -> *const c_char {
    ABI_VERSION_CSTR.as_ptr().cast::<c_char>()
}

#[no_mangle]
pub extern "C" fn titan_trit16_error_message(error_code: i32) -> *const c_char {
    let bytes = match error_code {
        0 => MSG_OK,
        1 => MSG_NULL_POINTER,
        2 => MSG_TRAP_DETECTED,
        3 => MSG_INVALID_INDEX,
        4 => MSG_INVALID_VALUE,
        5 => MSG_INVALID_LENGTH,
        255 => MSG_PANIC,
        _ => MSG_UNKNOWN_ERROR,
    };
    bytes.as_ptr().cast::<c_char>()
}

// ============================================================================
// 4. Constructors & Plane Accessors
// ============================================================================

#[no_mangle]
pub extern "C" fn titan_trit16_zero() -> TitanTrit16 {
    TitanTrit16::from(Trit16::zero())
}

#[no_mangle]
pub extern "C" fn titan_trit16_from_planes(pos: u16, neg: u16) -> TitanTrit16 {
    TitanTrit16::from(Trit16::from_planes(pos, neg))
}

/// # Safety
/// `out` must be a valid, aligned pointer to a `TitanTrit16`. `out_trap_mask` must be null
/// or a valid, aligned pointer to a `u16`.
#[no_mangle]
pub unsafe extern "C" fn titan_trit16_try_from_planes(
    pos: u16,
    neg: u16,
    out: *mut TitanTrit16,
    out_trap_mask: *mut u16,
) -> TitanError {
    with_ffi_guard(|| {
        if !is_valid_opt_ptr(out_trap_mask) || !is_valid_ptr(out) {
            return TitanError::NullPointer;
        }
        unsafe {
            write_optional_u16(out_trap_mask, 0);
        }
        match Trit16::try_from_planes(pos, neg) {
            Ok(w) => {
                unsafe {
                    out.write(TitanTrit16::from(w));
                }
                TitanError::Ok
            }
            Err(e) => map_trit_error(e, out_trap_mask),
        }
    })
}

#[no_mangle]
pub extern "C" fn titan_trit16_pos(w: TitanTrit16) -> u16 {
    Trit16::from(w).pos()
}

#[no_mangle]
pub extern "C" fn titan_trit16_neg(w: TitanTrit16) -> u16 {
    Trit16::from(w).neg()
}

// ============================================================================
// 5. Validation & Trap Detection
// ============================================================================

#[no_mangle]
pub extern "C" fn titan_trit16_is_valid(w: TitanTrit16) -> bool {
    Trit16::from(w).is_valid()
}

#[no_mangle]
pub extern "C" fn titan_trit16_has_traps(w: TitanTrit16) -> bool {
    Trit16::from(w).has_traps()
}

#[no_mangle]
pub extern "C" fn titan_trit16_trap_mask(w: TitanTrit16) -> u16 {
    Trit16::from(w).trap_mask()
}

/// # Safety
/// If `len > 0`, `words` must point to `len` valid, aligned `TitanTrit16` elements.
/// `out_trap_masks` (if non-null) must point to `len` writable `u16` elements.
/// `out_trap_count` (if non-null) must be a valid, aligned pointer to a `usize`.
#[no_mangle]
pub unsafe extern "C" fn titan_trit16_validate_batch(
    words: *const TitanTrit16,
    len: usize,
    out_trap_masks: *mut u16,
    out_trap_count: *mut usize,
) -> TitanError {
    with_ffi_guard(|| {
        if !is_valid_opt_ptr(out_trap_count) || !is_valid_opt_ptr(out_trap_masks) {
            return TitanError::NullPointer;
        }
        if !out_trap_count.is_null() {
            unsafe {
                out_trap_count.write(0);
            }
        }
        if len == 0 {
            return TitanError::Ok;
        }
        if !is_valid_ptr(words) {
            return TitanError::NullPointer;
        }
        if len > (isize::MAX as usize) / core::mem::size_of::<TitanTrit16>() {
            return TitanError::InvalidLength;
        }
        // Use raw pointer iteration rather than `slice::from_raw_parts` so no
        // shared `&[TitanTrit16]` reference coexists with writes to `out_trap_masks`.
        let mut trap_words = 0usize;
        for i in 0..len {
            let item = unsafe { words.add(i).read() };
            let mask = Trit16::from(item).trap_mask();
            if mask != 0 {
                trap_words += 1;
            }
            if !out_trap_masks.is_null() {
                unsafe {
                    out_trap_masks.add(i).write(mask);
                }
            }
        }
        if !out_trap_count.is_null() {
            unsafe {
                out_trap_count.write(trap_words);
            }
        }
        TitanError::Ok
    })
}

// ============================================================================
// 6. Per-Trit Accessors (Lossless & Strict)
// ============================================================================

/// # Safety
/// `out_val` must be a valid, aligned pointer to a `TitanKleene`.
#[no_mangle]
pub unsafe extern "C" fn titan_trit16_get(
    w: TitanTrit16,
    index: u8,
    out_val: *mut TitanKleene,
) -> TitanError {
    with_ffi_guard(|| {
        if !is_valid_ptr(out_val) {
            return TitanError::NullPointer;
        }
        match Trit16::from(w).get(index) {
            Ok(k) => {
                unsafe {
                    out_val.write(TitanKleene::from(k));
                }
                TitanError::Ok
            }
            Err(e) => map_trit_error(e, core::ptr::null_mut()),
        }
    })
}

/// # Safety
/// `w` must be a valid, aligned pointer to an initialized `TitanTrit16`.
#[no_mangle]
pub unsafe extern "C" fn titan_trit16_set(w: *mut TitanTrit16, index: u8, val: i32) -> TitanError {
    with_ffi_guard(|| {
        if !is_valid_ptr(w) {
            return TitanError::NullPointer;
        }
        let k = match parse_kleene_discriminant(val) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let mut inner = Trit16::from(unsafe { w.read() });
        match inner.set(index, k) {
            Ok(()) => {
                unsafe {
                    w.write(TitanTrit16::from(inner));
                }
                TitanError::Ok
            }
            Err(e) => map_trit_error(e, core::ptr::null_mut()),
        }
    })
}

/// # Safety
/// `w` must be a valid, aligned pointer to an initialized `TitanTrit16`.
/// `out_trap_mask` must be null or a valid, aligned pointer to a `u16`.
#[no_mangle]
pub unsafe extern "C" fn titan_trit16_set_strict(
    w: *mut TitanTrit16,
    index: u8,
    val: i32,
    out_trap_mask: *mut u16,
) -> TitanError {
    with_ffi_guard(|| {
        if !is_valid_opt_ptr(out_trap_mask) || !is_valid_ptr(w) {
            return TitanError::NullPointer;
        }
        unsafe {
            write_optional_u16(out_trap_mask, 0);
        }
        let k = match parse_kleene_discriminant(val) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let mut inner = Trit16::from(unsafe { w.read() });
        match inner.set_strict(index, k) {
            Ok(()) => {
                unsafe {
                    w.write(TitanTrit16::from(inner));
                }
                TitanError::Ok
            }
            Err(e) => map_trit_error(e, out_trap_mask),
        }
    })
}

// ============================================================================
// 7. Ordinary Bit-Plane Operations (Canonical Hardware Formulas)
//    All underlying core operations are pure, allocation-free `const fn`s
//    over `u16` bit-planes and are mathematically infallible. Executing them
//    directly avoids reserving any valid 32-bit `TitanTrit16` pattern (such as
//    all-Trap `{ neg: 0xFFFF, pos: 0xFFFF }`) as an in-band panic sentinel.
// ============================================================================

#[no_mangle]
pub extern "C" fn titan_trit16_and(a: TitanTrit16, b: TitanTrit16) -> TitanTrit16 {
    TitanTrit16::from(Trit16::from(a).and(Trit16::from(b)))
}

#[no_mangle]
pub extern "C" fn titan_trit16_or(a: TitanTrit16, b: TitanTrit16) -> TitanTrit16 {
    TitanTrit16::from(Trit16::from(a).or(Trit16::from(b)))
}

#[no_mangle]
pub extern "C" fn titan_trit16_not(a: TitanTrit16) -> TitanTrit16 {
    TitanTrit16::from(Trit16::from(a).not())
}

#[no_mangle]
pub extern "C" fn titan_trit16_collide(a: TitanTrit16, b: TitanTrit16) -> TitanTrit16 {
    TitanTrit16::from(Trit16::from(a).collide(Trit16::from(b)))
}

// ============================================================================
// 8. Strict Bit-Plane Operations (Pre-Operation Trap Rejection)
// ============================================================================

/// # Safety
/// `out` must be a valid, aligned pointer to a `TitanTrit16`.
/// `out_trap_mask` must be null or a valid, aligned pointer to a `u16`.
#[no_mangle]
pub unsafe extern "C" fn titan_trit16_and_strict(
    a: TitanTrit16,
    b: TitanTrit16,
    out: *mut TitanTrit16,
    out_trap_mask: *mut u16,
) -> TitanError {
    with_ffi_guard(|| {
        if !is_valid_opt_ptr(out_trap_mask) || !is_valid_ptr(out) {
            return TitanError::NullPointer;
        }
        unsafe {
            write_optional_u16(out_trap_mask, 0);
        }
        match Trit16::from(a).and_strict(Trit16::from(b)) {
            Ok(res) => {
                unsafe {
                    out.write(TitanTrit16::from(res));
                }
                TitanError::Ok
            }
            Err(e) => map_trit_error(e, out_trap_mask),
        }
    })
}

/// # Safety
/// `out` must be a valid, aligned pointer to a `TitanTrit16`.
/// `out_trap_mask` must be null or a valid, aligned pointer to a `u16`.
#[no_mangle]
pub unsafe extern "C" fn titan_trit16_or_strict(
    a: TitanTrit16,
    b: TitanTrit16,
    out: *mut TitanTrit16,
    out_trap_mask: *mut u16,
) -> TitanError {
    with_ffi_guard(|| {
        if !is_valid_opt_ptr(out_trap_mask) || !is_valid_ptr(out) {
            return TitanError::NullPointer;
        }
        unsafe {
            write_optional_u16(out_trap_mask, 0);
        }
        match Trit16::from(a).or_strict(Trit16::from(b)) {
            Ok(res) => {
                unsafe {
                    out.write(TitanTrit16::from(res));
                }
                TitanError::Ok
            }
            Err(e) => map_trit_error(e, out_trap_mask),
        }
    })
}

/// # Safety
/// `out` must be a valid, aligned pointer to a `TitanTrit16`.
/// `out_trap_mask` must be null or a valid, aligned pointer to a `u16`.
#[no_mangle]
pub unsafe extern "C" fn titan_trit16_not_strict(
    a: TitanTrit16,
    out: *mut TitanTrit16,
    out_trap_mask: *mut u16,
) -> TitanError {
    with_ffi_guard(|| {
        if !is_valid_opt_ptr(out_trap_mask) || !is_valid_ptr(out) {
            return TitanError::NullPointer;
        }
        unsafe {
            write_optional_u16(out_trap_mask, 0);
        }
        match Trit16::from(a).not_strict() {
            Ok(res) => {
                unsafe {
                    out.write(TitanTrit16::from(res));
                }
                TitanError::Ok
            }
            Err(e) => map_trit_error(e, out_trap_mask),
        }
    })
}

/// # Safety
/// `out` must be a valid, aligned pointer to a `TitanTrit16`.
/// `out_trap_mask` must be null or a valid, aligned pointer to a `u16`.
#[no_mangle]
pub unsafe extern "C" fn titan_trit16_collide_strict(
    a: TitanTrit16,
    b: TitanTrit16,
    out: *mut TitanTrit16,
    out_trap_mask: *mut u16,
) -> TitanError {
    with_ffi_guard(|| {
        if !is_valid_opt_ptr(out_trap_mask) || !is_valid_ptr(out) {
            return TitanError::NullPointer;
        }
        unsafe {
            write_optional_u16(out_trap_mask, 0);
        }
        match Trit16::from(a).collide_strict(Trit16::from(b)) {
            Ok(res) => {
                unsafe {
                    out.write(TitanTrit16::from(res));
                }
                TitanError::Ok
            }
            Err(e) => map_trit_error(e, out_trap_mask),
        }
    })
}

// ============================================================================
// 9. Interleaved 32-Bit Serialization (Lossless & Strict)
//    `to_u32_interleaved` and `from_u32_interleaved` are bounded 16-iteration
//    `const fn`s and are mathematically infallible; every `u32` (including
//    `0xFFFF_FFFF`) is a valid lossless 4-state Trit16 encoding.
// ============================================================================

#[no_mangle]
pub extern "C" fn titan_trit16_to_u32_interleaved(w: TitanTrit16) -> u32 {
    Trit16::from(w).to_u32_interleaved()
}

#[no_mangle]
pub extern "C" fn titan_trit16_from_u32_interleaved(raw: u32) -> TitanTrit16 {
    TitanTrit16::from(Trit16::from_u32_interleaved(raw))
}

/// # Safety
/// `out` must be a valid, aligned pointer to a `TitanTrit16`.
/// `out_trap_mask` must be null or a valid, aligned pointer to a `u16`.
#[no_mangle]
pub unsafe extern "C" fn titan_trit16_try_from_u32_interleaved(
    raw: u32,
    out: *mut TitanTrit16,
    out_trap_mask: *mut u16,
) -> TitanError {
    with_ffi_guard(|| {
        if !is_valid_opt_ptr(out_trap_mask) || !is_valid_ptr(out) {
            return TitanError::NullPointer;
        }
        unsafe {
            write_optional_u16(out_trap_mask, 0);
        }
        match Trit16::try_from_u32_interleaved(raw) {
            Ok(w) => {
                unsafe {
                    out.write(TitanTrit16::from(w));
                }
                TitanError::Ok
            }
            Err(e) => map_trit_error(e, out_trap_mask),
        }
    })
}

// ============================================================================
// 10. Scalar Kleene Operations (Trap-Dominant Reference)
// ============================================================================

#[no_mangle]
pub extern "C" fn titan_kleene_from_i8(val: i8) -> TitanKleene {
    TitanKleene::from(Kleene::from_i8(val))
}

/// # Safety
/// `out` must be a valid, aligned pointer to a `TitanKleene`.
#[no_mangle]
pub unsafe extern "C" fn titan_kleene_try_from_i8(val: i8, out: *mut TitanKleene) -> TitanError {
    with_ffi_guard(|| {
        if !is_valid_ptr(out) {
            return TitanError::NullPointer;
        }
        match Kleene::try_from_i8(val) {
            Ok(k) => {
                unsafe {
                    out.write(TitanKleene::from(k));
                }
                TitanError::Ok
            }
            Err(KleeneError::InvalidInt(_)) | Err(KleeneError::TrapValue) => {
                TitanError::InvalidValue
            }
        }
    })
}

/// # Safety
/// `out` must be a valid, aligned pointer to an `i8`.
#[no_mangle]
pub unsafe extern "C" fn titan_kleene_to_i8(val: i32, out: *mut i8) -> TitanError {
    with_ffi_guard(|| {
        if !is_valid_ptr(out) {
            return TitanError::NullPointer;
        }
        let k = match parse_kleene_discriminant(val) {
            Ok(v) => v,
            Err(e) => return e,
        };
        unsafe {
            out.write(k.to_i8());
        }
        TitanError::Ok
    })
}

/// # Safety
/// `out` must be a valid, aligned pointer to an `i8`.
#[no_mangle]
pub unsafe extern "C" fn titan_kleene_try_to_i8(val: i32, out: *mut i8) -> TitanError {
    with_ffi_guard(|| {
        if !is_valid_ptr(out) {
            return TitanError::NullPointer;
        }
        let k = match parse_kleene_discriminant(val) {
            Ok(v) => v,
            Err(e) => return e,
        };
        match k.try_to_i8() {
            Ok(v) => {
                unsafe {
                    out.write(v);
                }
                TitanError::Ok
            }
            Err(KleeneError::TrapValue) => TitanError::TrapDetected,
            Err(KleeneError::InvalidInt(_)) => TitanError::InvalidValue,
        }
    })
}

/// # Safety
/// `out` must be a valid, aligned pointer to a `TitanKleene`.
#[no_mangle]
pub unsafe extern "C" fn titan_kleene_and(a: i32, b: i32, out: *mut TitanKleene) -> TitanError {
    with_ffi_guard(|| {
        if !is_valid_ptr(out) {
            return TitanError::NullPointer;
        }
        let ka = match parse_kleene_discriminant(a) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let kb = match parse_kleene_discriminant(b) {
            Ok(v) => v,
            Err(e) => return e,
        };
        unsafe {
            out.write(TitanKleene::from(Kleene::and(ka, kb)));
        }
        TitanError::Ok
    })
}

/// # Safety
/// `out` must be a valid, aligned pointer to a `TitanKleene`.
#[no_mangle]
pub unsafe extern "C" fn titan_kleene_or(a: i32, b: i32, out: *mut TitanKleene) -> TitanError {
    with_ffi_guard(|| {
        if !is_valid_ptr(out) {
            return TitanError::NullPointer;
        }
        let ka = match parse_kleene_discriminant(a) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let kb = match parse_kleene_discriminant(b) {
            Ok(v) => v,
            Err(e) => return e,
        };
        unsafe {
            out.write(TitanKleene::from(Kleene::or(ka, kb)));
        }
        TitanError::Ok
    })
}

/// # Safety
/// `out` must be a valid, aligned pointer to a `TitanKleene`.
#[no_mangle]
pub unsafe extern "C" fn titan_kleene_not(a: i32, out: *mut TitanKleene) -> TitanError {
    with_ffi_guard(|| {
        if !is_valid_ptr(out) {
            return TitanError::NullPointer;
        }
        let ka = match parse_kleene_discriminant(a) {
            Ok(v) => v,
            Err(e) => return e,
        };
        unsafe {
            out.write(TitanKleene::from(Kleene::not(ka)));
        }
        TitanError::Ok
    })
}
