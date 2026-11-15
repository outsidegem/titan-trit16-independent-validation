/*
 * Pure C11 & C++17 Integration & Cross-Language Differential Test Suite
 * for `titan_trit16.h`, including parity checks against `~/Titan_Unified_Core/rules.h`.
 */

#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "../include/titan_trit16.h"

#if defined(__clang__)
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wc23-extensions"
#pragma clang diagnostic ignored "-Wimplicit-int-conversion"
#pragma clang diagnostic ignored "-Wsign-conversion"
#endif
#include "fixtures/rules.h"
#if defined(__clang__)
#pragma clang diagnostic pop
#endif

static void test_abi_version_and_layout(void) {
    assert(titan_trit16_abi_version() == TITAN_TRIT16_ABI_VERSION_PACKED);
    assert(titan_trit16_abi_version() == 0x00010000u);
    assert(strcmp(titan_trit16_abi_version_string(), "1.0.0") == 0);
    assert(strcmp(titan_trit16_error_message(TITAN_OK), "OK") == 0);
    assert(strcmp(titan_trit16_error_message(TITAN_ERR_TRAP_DETECTED), "Hardware trap bits detected") == 0);

    /* Layout must match titan16_word_t.plane in rules.h */
    assert(sizeof(TitanTrit16) == 4u);
    assert(sizeof(titan16_word_t) == 4u);
    assert(offsetof(TitanTrit16, neg) == offsetof(titan16_word_t, plane.neg));
    assert(offsetof(TitanTrit16, pos) == offsetof(titan16_word_t, plane.pos));

    /* Enum discriminants must match 2-bit codes (0, 1, 2, 3) */
    assert((uint32_t)TITAN_KLEENE_UNKNOWN == 0u);
    assert((uint32_t)TITAN_KLEENE_TRUE == 1u);
    assert((uint32_t)TITAN_KLEENE_FALSE == 2u);
    assert((uint32_t)TITAN_KLEENE_TRAP == 3u);
}

static void test_constructors_and_validation(void) {
    TitanTrit16 z = titan_trit16_zero();
    assert(titan_trit16_pos(z) == 0u);
    assert(titan_trit16_neg(z) == 0u);
    assert(titan_trit16_is_valid(z));
    assert(!titan_trit16_has_traps(z));
    assert(titan_trit16_trap_mask(z) == 0u);

    /* Lossless constructor preserves traps */
    TitanTrit16 trapped = titan_trit16_from_planes(0x00A5u, 0x0005u);
    assert(!titan_trit16_is_valid(trapped));
    assert(titan_trit16_has_traps(trapped));
    assert(titan_trit16_trap_mask(trapped) == 0x0005u);

    /* Strict constructor rejects traps and outputs mask */
    TitanTrit16 out = titan_trit16_zero();
    uint16_t mask = 0u;
    TitanError err = titan_trit16_try_from_planes(0x00A5u, 0x0005u, &out, &mask);
    assert(err == TITAN_ERR_TRAP_DETECTED);
    assert(mask == 0x0005u);

    /* Strict constructor succeeds on disjoint planes */
    err = titan_trit16_try_from_planes(0xAAAAu, 0x5555u, &out, &mask);
    assert(err == TITAN_OK);
    assert(mask == 0u);
    assert(titan_trit16_pos(out) == 0xAAAAu);
    assert(titan_trit16_neg(out) == 0x5555u);
}

static void test_exhaustive_truth_tables_and_rules_h_parity(void) {
    /* Pack all 16 combinations of (A, B) into a single 16-trit word */
    TitanTrit16 wa = titan_trit16_zero();
    TitanTrit16 wb = titan_trit16_zero();

    for (uint8_t a = 0u; a < 4u; ++a) {
        for (uint8_t b = 0u; b < 4u; ++b) {
            uint8_t idx = (uint8_t)(a * 4u + b);
            assert(titan_trit16_set(&wa, idx, (int32_t)a) == TITAN_OK);
            assert(titan_trit16_set(&wb, idx, (int32_t)b) == TITAN_OK);
        }
    }

    TitanTrit16 r_and = titan_trit16_and(wa, wb);
    TitanTrit16 r_or  = titan_trit16_or(wa, wb);
    TitanTrit16 r_not = titan_trit16_not(wa);
    TitanTrit16 r_col = titan_trit16_collide(wa, wb);

    /* Compare directly against legacy rules.h bit-plane operations */
    titan16_word_t la = titan16_make(wa.pos, wa.neg);
    titan16_word_t lb = titan16_make(wb.pos, wb.neg);

    titan16_word_t l_and = titan16_and(la, lb);
    titan16_word_t l_or  = titan16_or(la, lb);
    titan16_word_t l_not = titan16_not(la);
    titan16_word_t l_col = titan16_collide(la, lb);

    assert(r_and.pos == l_and.plane.pos && r_and.neg == l_and.plane.neg);
    assert(r_or.pos  == l_or.plane.pos  && r_or.neg  == l_or.plane.neg);
    assert(r_not.pos == l_not.plane.pos && r_not.neg == l_not.plane.neg);
    assert(r_col.pos == l_col.plane.pos && r_col.neg == l_col.plane.neg);

    /* Verify critical regression cases in C */
    TitanKleene k = TITAN_KLEENE_UNKNOWN;
    /* Index 12 is (Trap=3, Unknown=0): Trap AND Unknown -> False (2) in bit-plane */
    assert(titan_trit16_get(r_and, 12u, &k) == TITAN_OK && k == TITAN_KLEENE_FALSE);
    /* Scalar Kleene: Trap AND Unknown -> Trap (3) */
    assert(titan_kleene_and(TITAN_KLEENE_TRAP, TITAN_KLEENE_UNKNOWN, &k) == TITAN_OK && k == TITAN_KLEENE_TRAP);

    /* Index 13 is (Trap=3, True=1): Trap OR True -> True (1) in bit-plane */
    assert(titan_trit16_get(r_or, 13u, &k) == TITAN_OK && k == TITAN_KLEENE_TRUE);
    /* Scalar Kleene: Trap OR True -> Trap (3) */
    assert(titan_kleene_or(TITAN_KLEENE_TRAP, TITAN_KLEENE_TRUE, &k) == TITAN_OK && k == TITAN_KLEENE_TRAP);

    /* Index 15 is (Trap=3, Trap=3): NOT Trap -> Trap (3); COLLIDE(Trap, Trap) -> Unknown (0) */
    assert(titan_trit16_get(r_not, 15u, &k) == TITAN_OK && k == TITAN_KLEENE_TRAP);
    assert(titan_trit16_get(r_col, 15u, &k) == TITAN_OK && k == TITAN_KLEENE_UNKNOWN);

    /* Serialization parity with rules.h titan16_to_interleaved and lossless round-trip */
    uint32_t packed = titan_trit16_to_u32_interleaved(wa);
    assert(packed == titan16_to_interleaved(la));

    TitanTrit16 unpacked = titan_trit16_from_u32_interleaved(packed);
    assert(unpacked.pos == wa.pos && unpacked.neg == wa.neg);

    /* Strict deserialization rejects trap-containing packed word with full mask */
    TitanTrit16 strict_out = titan_trit16_zero();
    uint16_t mask = 0u;
    assert(titan_trit16_try_from_u32_interleaved(packed, &strict_out, &mask) == TITAN_ERR_TRAP_DETECTED);
    assert(mask == titan_trit16_trap_mask(wa));
}

static void test_strict_ops_and_pointer_validation(void) {
    TitanTrit16 a = titan_trit16_from_planes(0x0001u, 0x0001u); /* trap bit 0 */
    TitanTrit16 b = titan_trit16_from_planes(0x0008u, 0x0008u); /* trap bit 3 */
    TitanTrit16 out = titan_trit16_zero();
    uint16_t mask = 0u;

    assert(titan_trit16_and_strict(a, b, &out, &mask) == TITAN_ERR_TRAP_DETECTED && mask == 0x0009u);
    assert(titan_trit16_or_strict(a, b, &out, &mask) == TITAN_ERR_TRAP_DETECTED && mask == 0x0009u);
    assert(titan_trit16_collide_strict(a, b, &out, &mask) == TITAN_ERR_TRAP_DETECTED && mask == 0x0009u);
    assert(titan_trit16_not_strict(a, &out, &mask) == TITAN_ERR_TRAP_DETECTED && mask == 0x0001u);

    /* Null pointer checks */
    assert(titan_trit16_try_from_planes(0u, 0u, NULL, &mask) == TITAN_ERR_NULL_POINTER);
    assert(titan_trit16_get(a, 0u, NULL) == TITAN_ERR_NULL_POINTER);
    assert(titan_trit16_set(NULL, 0u, TITAN_KLEENE_TRUE) == TITAN_ERR_NULL_POINTER);
    assert(titan_trit16_set_strict(NULL, 0u, TITAN_KLEENE_TRUE, &mask) == TITAN_ERR_NULL_POINTER);
    assert(titan_trit16_and_strict(a, b, NULL, &mask) == TITAN_ERR_NULL_POINTER);
    assert(titan_trit16_or_strict(a, b, NULL, &mask) == TITAN_ERR_NULL_POINTER);
    assert(titan_trit16_not_strict(a, NULL, &mask) == TITAN_ERR_NULL_POINTER);
    assert(titan_trit16_collide_strict(a, b, NULL, &mask) == TITAN_ERR_NULL_POINTER);
    assert(titan_trit16_try_from_u32_interleaved(0u, NULL, &mask) == TITAN_ERR_NULL_POINTER);
    assert(titan_kleene_and(0, 0, NULL) == TITAN_ERR_NULL_POINTER);

    /* Misaligned pointer rejection check */
    uint8_t raw_bytes[16] = {0};
    TitanTrit16* unaligned_word_ptr = (TitanTrit16*)(void*)(&raw_bytes[1]);
    assert(titan_trit16_set(unaligned_word_ptr, 0u, TITAN_KLEENE_TRUE) == TITAN_ERR_NULL_POINTER);

    /* Out-of-bounds index and invalid enum discriminant checks */
    TitanKleene k = TITAN_KLEENE_UNKNOWN;
    assert(titan_trit16_get(a, 16u, &k) == TITAN_ERR_INVALID_INDEX);
    assert(titan_trit16_set(&out, 16u, TITAN_KLEENE_TRUE) == TITAN_ERR_INVALID_INDEX);
    assert(titan_trit16_set(&out, 0u, 99) == TITAN_ERR_INVALID_VALUE);
    assert(titan_kleene_and(99, 0, &k) == TITAN_ERR_INVALID_VALUE);
    assert(titan_kleene_try_from_i8(42, &k) == TITAN_ERR_INVALID_VALUE);

    int8_t i8_out = 0;
    assert(titan_kleene_to_i8(TITAN_KLEENE_TRAP, &i8_out) == TITAN_OK && i8_out == 0);
    assert(titan_kleene_try_to_i8(TITAN_KLEENE_TRAP, &i8_out) == TITAN_ERR_TRAP_DETECTED);

    /* Batch validation (including in-place aliasing safety) */
    TitanTrit16 batch[3];
    batch[0] = titan_trit16_from_planes(0x0001u, 0x0001u);
    batch[1] = titan_trit16_from_planes(0x0002u, 0x0000u);
    batch[2] = titan_trit16_from_planes(0x0010u, 0x0010u);

    uint16_t batch_masks[3] = {0u, 0u, 0u};
    size_t trap_count = 0u;
    assert(titan_trit16_validate_batch(NULL, 3u, batch_masks, &trap_count) == TITAN_ERR_NULL_POINTER);
    assert(titan_trit16_validate_batch(NULL, 0u, NULL, &trap_count) == TITAN_OK && trap_count == 0u);
    assert(titan_trit16_validate_batch(batch, 3u, batch_masks, &trap_count) == TITAN_OK);
    assert(trap_count == 2u);
    assert(batch_masks[0] == 0x0001u && batch_masks[1] == 0x0000u && batch_masks[2] == 0x0010u);

    /* Aliased in-place write over batch buffer */
    assert(titan_trit16_validate_batch(batch, 3u, (uint16_t*)(void*)batch, &trap_count) == TITAN_OK);
    assert(trap_count == 2u);
}

static void test_all_trap_value_returning_operations(void) {
    /* Verify 0xFFFF / 0xFFFFFFFF is preserved as legitimate all-Trap data across all value-returning FFI functions */
    TitanTrit16 all_trap = titan_trit16_from_planes(0xFFFFu, 0xFFFFu);
    assert(titan_trit16_pos(all_trap) == 0xFFFFu);
    assert(titan_trit16_neg(all_trap) == 0xFFFFu);
    assert(!titan_trit16_is_valid(all_trap));
    assert(titan_trit16_has_traps(all_trap));
    assert(titan_trit16_trap_mask(all_trap) == 0xFFFFu);

    TitanTrit16 and_res = titan_trit16_and(all_trap, all_trap);
    assert(and_res.pos == 0xFFFFu && and_res.neg == 0xFFFFu);

    TitanTrit16 or_res = titan_trit16_or(all_trap, all_trap);
    assert(or_res.pos == 0xFFFFu && or_res.neg == 0xFFFFu);

    TitanTrit16 not_res = titan_trit16_not(all_trap);
    assert(not_res.pos == 0xFFFFu && not_res.neg == 0xFFFFu);

    TitanTrit16 col_res = titan_trit16_collide(all_trap, all_trap);
    assert(col_res.pos == 0x0000u && col_res.neg == 0x0000u);

    uint32_t packed = titan_trit16_to_u32_interleaved(all_trap);
    assert(packed == 0xFFFFFFFFu);

    TitanTrit16 unpacked = titan_trit16_from_u32_interleaved(0xFFFFFFFFu);
    assert(unpacked.pos == 0xFFFFu && unpacked.neg == 0xFFFFu);

    assert(titan_kleene_from_i8(0) == TITAN_KLEENE_UNKNOWN);
    assert(titan_kleene_from_i8(1) == TITAN_KLEENE_TRUE);
    assert(titan_kleene_from_i8(-1) == TITAN_KLEENE_FALSE);
    assert(titan_kleene_from_i8(127) == TITAN_KLEENE_TRAP);
}

int main(void) {
    test_abi_version_and_layout();
    test_constructors_and_validation();
    test_exhaustive_truth_tables_and_rules_h_parity();
    test_strict_ops_and_pointer_validation();
    test_all_trap_value_returning_operations();
    printf("[C/C++ ABI TEST] All compilation, linking, layout, and runtime assertions PASSED.\n");
    return 0;
}
