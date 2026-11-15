#include "titan_trit16.h"

#include <assert.h>
#include <stdio.h>

int main(void) {
    TitanTrit16 zero = titan_trit16_zero();

    assert(titan_trit16_abi_version() ==
           TITAN_TRIT16_ABI_VERSION_PACKED);
    assert(titan_trit16_is_valid(zero));
    assert(!titan_trit16_has_traps(zero));

    TitanTrit16 truth =
        titan_trit16_from_planes(0x0001u, 0x0000u);

    TitanKleene value = TITAN_KLEENE_UNKNOWN;

    assert(titan_trit16_get(truth, 0u, &value) == TITAN_OK);
    assert(value == TITAN_KLEENE_TRUE);

    TitanTrit16 result = titan_trit16_and(truth, truth);

    assert(titan_trit16_get(result, 0u, &value) == TITAN_OK);
    assert(value == TITAN_KLEENE_TRUE);

    uint32_t encoded = titan_trit16_to_u32_interleaved(truth);
    TitanTrit16 decoded =
        titan_trit16_from_u32_interleaved(encoded);

    assert(decoded.pos == truth.pos);
    assert(decoded.neg == truth.neg);

    TitanTrit16 rejected = {0};
    uint16_t trap_mask = 0;

    assert(titan_trit16_try_from_planes(
        0x0004u, 0x0004u, &rejected, &trap_mask
    ) == TITAN_ERR_TRAP_DETECTED);

    assert(trap_mask == 0x0004u);

    assert(titan_trit16_get(
        truth, 16u, &value
    ) == TITAN_ERR_INVALID_INDEX);

    TitanTrit16 trap_word =
        titan_trit16_from_planes(0x0008u, 0x0008u);

    assert(titan_trit16_and_strict(
        trap_word, truth, &rejected, &trap_mask
    ) == TITAN_ERR_TRAP_DETECTED);

    assert(trap_mask == 0x0008u);

    printf("TRIT16 Linux C consumer: PASS (ABI %s)\n",
           titan_trit16_abi_version_string());

    return 0;
}
