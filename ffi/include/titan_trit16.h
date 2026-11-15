#ifndef TITAN_TRIT16_H
#define TITAN_TRIT16_H

/*
 * Titan Trit16 SDK — Stable C ABI Header (v1.0.0)
 *
 * Ownership, Alignment & Memory Lifetime Contract:
 * 1. All `TitanTrit16` words (4 bytes, 2-byte aligned) and `TitanKleene` values
 *    are plain value types owned by the caller. Zero heap allocations are
 *    performed by this library.
 * 2. Output pointers (`out`, `out_val`, `out_trap_mask`, `out_trap_masks`,
 *    `out_trap_count`) must point to valid, properly aligned, caller-allocated
 *    memory. `out_trap_mask` and `out_trap_count` are optional and may be NULL.
 *    Passing NULL or a misaligned pointer for a required pointer returns
 *    `TITAN_ERR_NULL_POINTER`.
 * 3. Strings returned by `titan_trit16_abi_version_string()` and
 *    `titan_trit16_error_message()` reside in static read-only memory (`'static`
 *    lifetime) and MUST NOT be modified or freed by the caller.
 * 4. All FFI entry points guard against Rust unwinding; panics never cross the
 *    C ABI boundary.
 */

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ==========================================================================
 * 1. ABI Version Constants
 * ========================================================================== */
#define TITAN_TRIT16_ABI_VERSION_MAJOR  1u
#define TITAN_TRIT16_ABI_VERSION_MINOR  0u
#define TITAN_TRIT16_ABI_VERSION_PATCH  0u
#define TITAN_TRIT16_ABI_VERSION_PACKED 0x00010000u

/* ==========================================================================
 * 2. Core Data Types & Error Codes
 * ========================================================================== */

/**
 * 16-trit bit-sliced word matching the two-plane layout of `titan16_word_t.plane`
 * in `rules.h` (`neg` at byte offset 0, `pos` at byte offset 2).
 *
 * Portability Note: Do not bit-cast `TitanTrit16` to `uint32_t` across
 * big-endian architectures; use `titan_trit16_to_u32_interleaved()` or the
 * `pos`/`neg` fields directly.
 */
typedef struct TitanTrit16 {
    uint16_t neg; /* Negative plane: bit i = 1 iff trit i == -1 (or Trap) */
    uint16_t pos; /* Positive plane: bit i = 1 iff trit i == +1 (or Trap) */
} TitanTrit16;

/**
 * Canonical 4-state trit values. Discriminants match the 2-bit interleaved
 * encodings in `rules.h` (`T_NULL=0`, `T_POS=1`, `T_NEG=2`, `T_ERR=3`).
 */
typedef enum TitanKleene {
    TITAN_KLEENE_UNKNOWN = 0, /* 0b00:  0 (Equilibrium / Unknown) */
    TITAN_KLEENE_TRUE    = 1, /* 0b01: +1 (True / Positive)       */
    TITAN_KLEENE_FALSE   = 2, /* 0b10: -1 (False / Negative)      */
    TITAN_KLEENE_TRAP    = 3  /* 0b11: Hardware Trap / Fault      */
} TitanKleene;

/**
 * Explicit status and error codes returned by fallible C ABI functions.
 */
typedef enum TitanError {
    TITAN_OK                 = 0,   /* Operation succeeded                         */
    TITAN_ERR_NULL_POINTER   = 1,   /* Required pointer was NULL or misaligned     */
    TITAN_ERR_TRAP_DETECTED  = 2,   /* Strict operation rejected trap bits         */
    TITAN_ERR_INVALID_INDEX  = 3,   /* Trit index out of range (must be 0..15)     */
    TITAN_ERR_INVALID_VALUE  = 4,   /* Enum discriminant or i8 out of range        */
    TITAN_ERR_INVALID_LENGTH = 5,   /* Batch length invalid or overflows address   */
    TITAN_ERR_PANIC          = 255  /* Internal panic caught at FFI boundary       */
} TitanError;

/* Compile-time ABI layout verification for C11 and C++11+ */
#if defined(__cplusplus) && (__cplusplus >= 201103L)
static_assert(sizeof(TitanTrit16) == 4, "TitanTrit16 must be 4 bytes");
static_assert(alignof(TitanTrit16) == 2, "TitanTrit16 must be 2-byte aligned");
static_assert(offsetof(TitanTrit16, neg) == 0, "TitanTrit16.neg must be at offset 0");
static_assert(offsetof(TitanTrit16, pos) == 2, "TitanTrit16.pos must be at offset 2");
static_assert(sizeof(TitanKleene) == 4, "TitanKleene must be 4 bytes");
static_assert(sizeof(TitanError) == 4, "TitanError must be 4 bytes");
#elif defined(__STDC_VERSION__) && (__STDC_VERSION__ >= 201112L)
_Static_assert(sizeof(TitanTrit16) == 4, "TitanTrit16 must be 4 bytes");
_Static_assert(_Alignof(TitanTrit16) == 2, "TitanTrit16 must be 2-byte aligned");
_Static_assert(offsetof(TitanTrit16, neg) == 0, "TitanTrit16.neg must be at offset 0");
_Static_assert(offsetof(TitanTrit16, pos) == 2, "TitanTrit16.pos must be at offset 2");
_Static_assert(sizeof(TitanKleene) == 4, "TitanKleene must be 4 bytes");
_Static_assert(sizeof(TitanError) == 4, "TitanError must be 4 bytes");
#endif

/* ==========================================================================
 * 3. ABI Version & Diagnostics
 * ========================================================================== */

uint32_t titan_trit16_abi_version(void);
const char* titan_trit16_abi_version_string(void);
const char* titan_trit16_error_message(int32_t error_code);

/* ==========================================================================
 * 4. Constructors & Plane Accessors
 * ========================================================================== */

TitanTrit16 titan_trit16_zero(void);
TitanTrit16 titan_trit16_from_planes(uint16_t pos, uint16_t neg);

TitanError titan_trit16_try_from_planes(
    uint16_t pos,
    uint16_t neg,
    TitanTrit16* out,
    uint16_t* out_trap_mask
);

uint16_t titan_trit16_pos(TitanTrit16 w);
uint16_t titan_trit16_neg(TitanTrit16 w);

/* ==========================================================================
 * 5. Validation & Trap Detection
 * ========================================================================== */

bool titan_trit16_is_valid(TitanTrit16 w);
bool titan_trit16_has_traps(TitanTrit16 w);
uint16_t titan_trit16_trap_mask(TitanTrit16 w);

TitanError titan_trit16_validate_batch(
    const TitanTrit16* words,
    size_t len,
    uint16_t* out_trap_masks,
    size_t* out_trap_count
);

/* ==========================================================================
 * 6. Per-Trit Accessors (Lossless & Strict)
 * ========================================================================== */

TitanError titan_trit16_get(
    TitanTrit16 w,
    uint8_t index,
    TitanKleene* out_val
);

TitanError titan_trit16_set(
    TitanTrit16* w,
    uint8_t index,
    int32_t val
);

TitanError titan_trit16_set_strict(
    TitanTrit16* w,
    uint8_t index,
    int32_t val,
    uint16_t* out_trap_mask
);

/* ==========================================================================
 * 7. Ordinary Bit-Plane Operations (Canonical Hardware Formulas)
 * ========================================================================== */

TitanTrit16 titan_trit16_and(TitanTrit16 a, TitanTrit16 b);
TitanTrit16 titan_trit16_or(TitanTrit16 a, TitanTrit16 b);
TitanTrit16 titan_trit16_not(TitanTrit16 a);
TitanTrit16 titan_trit16_collide(TitanTrit16 a, TitanTrit16 b);

/* ==========================================================================
 * 8. Strict Bit-Plane Operations (Pre-Operation Trap Rejection)
 * ========================================================================== */

TitanError titan_trit16_and_strict(
    TitanTrit16 a,
    TitanTrit16 b,
    TitanTrit16* out,
    uint16_t* out_trap_mask
);

TitanError titan_trit16_or_strict(
    TitanTrit16 a,
    TitanTrit16 b,
    TitanTrit16* out,
    uint16_t* out_trap_mask
);

TitanError titan_trit16_not_strict(
    TitanTrit16 a,
    TitanTrit16* out,
    uint16_t* out_trap_mask
);

TitanError titan_trit16_collide_strict(
    TitanTrit16 a,
    TitanTrit16 b,
    TitanTrit16* out,
    uint16_t* out_trap_mask
);

/* ==========================================================================
 * 9. Interleaved 32-Bit Serialization (Lossless & Strict)
 * ========================================================================== */

uint32_t titan_trit16_to_u32_interleaved(TitanTrit16 w);
TitanTrit16 titan_trit16_from_u32_interleaved(uint32_t raw);

TitanError titan_trit16_try_from_u32_interleaved(
    uint32_t raw,
    TitanTrit16* out,
    uint16_t* out_trap_mask
);

/* ==========================================================================
 * 10. Scalar Kleene Operations (Trap-Dominant Reference)
 * ========================================================================== */

TitanKleene titan_kleene_from_i8(int8_t val);
TitanError titan_kleene_try_from_i8(int8_t val, TitanKleene* out);
TitanError titan_kleene_to_i8(int32_t val, int8_t* out);
TitanError titan_kleene_try_to_i8(int32_t val, int8_t* out);

TitanError titan_kleene_and(int32_t a, int32_t b, TitanKleene* out);
TitanError titan_kleene_or(int32_t a, int32_t b, TitanKleene* out);
TitanError titan_kleene_not(int32_t a, TitanKleene* out);

#ifdef __cplusplus
}
#endif

#endif /* TITAN_TRIT16_H */
