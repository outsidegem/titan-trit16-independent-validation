/*
 * Copyright 2026 Titan Project Contributors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#ifndef TITAN_2_0_RULES_H
#define TITAN_2_0_RULES_H

// TITAN 2.0 - The Sensory Stack & 16-Trit Hardware Logic Protocol
// Discrete Balanced Ternary Logic Engine ({-1, 0, +1}) for Bare-Metal ARM64

#include <stdint.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>

// ============================================================================
// 1. THE CORE TERNARY 2-BIT STATES (Single Trit)
// ============================================================================
#define T_NULL  0b00  // 0  (Equilibrium / Unknown / Zero)
#define T_POS   0b01  // +1 (True / Positive Assertion)
#define T_NEG   0b10  // -1 (False / Negative Assertion)
#define T_ERR   0b11  // Hardware Trap / Illegal State (Immediate Fault)

// 4-Trit Sensory Bit Masks (for 8-bit packed packets)
#define MASK_LOGIC      0b00000011  // Trit 1: The Base Fact (Shift 0)
#define MASK_INTENSITY  0b00001100  // Trit 2: The Magnitude (Shift 2)
#define MASK_CONFIDENCE 0b00110000  // Trit 3: Internal Certainty (Shift 4)
#define MASK_CONTEXT    0b11000000  // Trit 4: Relationship / Intent (Shift 6)

// Convert single 2-bit trit to human integer {-1, 0, +1} with fail-stop trap check
static inline int decode_trit_safe(uint8_t trit_val) {
    switch (trit_val & 0x03) {
        case T_POS:  return 1;
        case T_NEG:  return -1;
        case T_NULL: return 0;
        case T_ERR:
        default:
            fprintf(stderr, "[FATAL] Hardware Trap 0b11 encountered in decode_trit_safe!\n");
            return 0;
    }
}

// Legacy helper for backward compatibility
static inline int decode_to_human(uint8_t trit_val) {
    if ((trit_val & 0x03) == T_POS) return 1;
    if ((trit_val & 0x03) == T_NEG) return -1;
    return 0;
}

// Convert trit to string
static inline const char* trit_to_str(uint8_t trit_val) {
    switch (trit_val & 0x03) {
        case T_POS:  return " +1";
        case T_NEG:  return " -1";
        case T_NULL: return "  0";
        case T_ERR:
        default:     return "ERR";
    }
}

// Check if any trit in an 8-bit packet is in the illegal T_ERR (0b11) state
static inline bool titan_validate_packet(uint8_t packet) {
    for (int shift = 0; shift < 8; shift += 2) {
        if (((packet >> shift) & 0x03) == T_ERR) {
            return false;
        }
    }
    return true;
}

// Pack 4 senses into a single Zero Token byte (8-bit)
static inline uint8_t encode_titan_state(uint8_t logic, uint8_t intensity, uint8_t confidence, uint8_t context) {
    return ((context & 0x03) << 6) | 
           ((confidence & 0x03) << 4) | 
           ((intensity & 0x03) << 2) | 
           (logic & 0x03);
}

// Extract a specific sense from the byte
static inline uint8_t read_sense(uint8_t state, uint8_t mask, uint8_t shift) {
    return (state & mask) >> shift;
}

// ============================================================================
// 2. 16-TRIT BIT-SLICED HARDWARE KERNEL (32-Bit Register Word)
// ============================================================================
// Bit-Plane Representation:
//   Bits 31..16 : Positive Plane (Bit i = 1 if Trit i == +1)
//   Bits 15..0  : Negative Plane (Bit i = 1 if Trit i == -1)
//   Null State  : Bit i is 0 in both planes
//   Trap (ERR)  : Bit i is 1 in both planes (HARDWARE FAULT: pos & neg != 0)

typedef union {
    uint32_t raw;
    struct {
        uint16_t neg; // Bits 0..15  (Negative Plane)
        uint16_t pos; // Bits 16..31 (Positive Plane)
    } plane;
} titan16_word_t;

// Check if 16-trit word is structurally valid (no overlapping pos & neg bits)
static inline bool titan16_is_valid(titan16_word_t w) {
    return (w.plane.pos & w.plane.neg) == 0;
}

// Returns the bitmask of any illegal hardware trap trits
static inline uint16_t titan16_detect_traps(titan16_word_t w) {
    return (w.plane.pos & w.plane.neg);
}

// Construct titan16_word_t from raw planes
static inline titan16_word_t titan16_make(uint16_t pos_plane, uint16_t neg_plane) {
    titan16_word_t w;
    w.plane.pos = pos_plane;
    w.plane.neg = neg_plane;
    return w;
}

// Set a specific trit index (0..15)
static inline titan16_word_t titan16_set_trit(titan16_word_t w, int index, int8_t trit_val) {
    if (index < 0 || index >= 16) return w;
    uint16_t mask = (uint16_t)(1u << index);
    w.plane.pos &= ~mask;
    w.plane.neg &= ~mask;
    if (trit_val == 1) {
        w.plane.pos |= mask;
    } else if (trit_val == -1) {
        w.plane.neg |= mask;
    }
    return w;
}

// Get a specific trit value (0..15) returning -1, 0, +1
static inline int8_t titan16_get_trit(titan16_word_t w, int index) {
    if (index < 0 || index >= 16) return 0;
    uint16_t mask = (uint16_t)(1u << index);
    bool is_pos = (w.plane.pos & mask) != 0;
    bool is_neg = (w.plane.neg & mask) != 0;
    if (is_pos && is_neg) return 0; // Trap
    if (is_pos) return 1;
    if (is_neg) return -1;
    return 0;
}

// --- ARM64 BRANCHLESS KLEENE LOGIC OPERATIONS (16-TRIT PARALLEL) ---

// Kleene AND (Min): P_out = P_a & P_b, N_out = N_a | N_b
static inline titan16_word_t titan16_and(titan16_word_t a, titan16_word_t b) {
    titan16_word_t r;
    r.plane.pos = a.plane.pos & b.plane.pos;
    r.plane.neg = a.plane.neg | b.plane.neg;
    return r;
}

// Kleene OR (Max): P_out = P_a | P_b, N_out = N_a & N_b
static inline titan16_word_t titan16_or(titan16_word_t a, titan16_word_t b) {
    titan16_word_t r;
    r.plane.pos = a.plane.pos | b.plane.pos;
    r.plane.neg = a.plane.neg & b.plane.neg;
    return r;
}

// Kleene NOT (Negation): Swap Positive and Negative Planes
static inline titan16_word_t titan16_not(titan16_word_t a) {
    titan16_word_t r;
    r.plane.pos = a.plane.neg;
    r.plane.neg = a.plane.pos;
    return r;
}

// 16-Trit Collision / Consensus Engine:
// Resolves agreement where identical, collapses conflict to equilibrium
static inline titan16_word_t titan16_collide(titan16_word_t a, titan16_word_t b) {
    titan16_word_t r;
    uint16_t agree_pos = a.plane.pos & b.plane.pos;
    uint16_t agree_neg = a.plane.neg & b.plane.neg;
    uint16_t conflict  = (a.plane.pos & b.plane.neg) | (a.plane.neg & b.plane.pos);
    
    // Kept where agreed; conflict collapses to 0
    r.plane.pos = agree_pos & ~conflict;
    r.plane.neg = agree_neg & ~conflict;
    return r;
}

// Convert 16-trit word from interleaved 32-bit format (2 bits per trit) to Bit-Plane format
static inline titan16_word_t titan16_from_interleaved(uint32_t interleaved) {
    titan16_word_t w;
    w.plane.pos = 0;
    w.plane.neg = 0;
    for (int i = 0; i < 16; i++) {
        uint32_t trit = (interleaved >> (i * 2)) & 0x03;
        if (trit == T_POS) {
            w.plane.pos |= (1u << i);
        } else if (trit == T_NEG) {
            w.plane.neg |= (1u << i);
        }
    }
    return w;
}

// Convert 16-trit word from Bit-Plane format back to interleaved 32-bit format
static inline uint32_t titan16_to_interleaved(titan16_word_t w) {
    uint32_t out = 0;
    for (int i = 0; i < 16; i++) {
        uint16_t mask = (1u << i);
        bool p = (w.plane.pos & mask) != 0;
        bool n = (w.plane.neg & mask) != 0;
        if (p && !n) {
            out |= ((uint32_t)T_POS << (i * 2));
        } else if (!p && n) {
            out |= ((uint32_t)T_NEG << (i * 2));
        } else if (p && n) {
            out |= ((uint32_t)T_ERR << (i * 2));
        }
    }
    return out;
}

#endif // TITAN_2_0_RULES_H
