//! 16-Trit Bit-Sliced Hardware Engine (`Trit16`).
//!
//! Preserves the two-plane (`pos`, `neg`) encoding and branchless bitwise
//! operations from `~/Titan_Unified_Core/kleene_logic.rs` and `rules.h`,
//! while adding explicit trap-aware accessors and strict validation variants.

use crate::kleene::Kleene;
use core::fmt;
use thiserror::Error;

/// Errors returned by `Trit16` validation, indexing, and strict operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TritError {
    /// One or more trap bits (`pos & neg != 0`) were detected.
    /// The `u16` payload is the bitmask of all offending trit positions.
    #[error("Hardware trap bits detected (mask: 0x{0:04X})")]
    TrapDetected(u16),

    /// A trit index outside the valid `0..=15` range was supplied.
    #[error("Trit index {0} out of range (must be 0..=15)")]
    InvalidIndex(u8),
}

/// 16-trit bit-sliced vector stored as separate positive and negative 16-bit planes.
///
/// Per-trit encoding at bit position `i` (`0..=15`):
/// - `pos[i] = 0, neg[i] = 0` -> [`Kleene::Unknown`] (`0`, interleaved `0b00`)
/// - `pos[i] = 1, neg[i] = 0` -> [`Kleene::True`] (`+1`, interleaved `0b01`)
/// - `pos[i] = 0, neg[i] = 1` -> [`Kleene::False`] (`-1`, interleaved `0b10`)
/// - `pos[i] = 1, neg[i] = 1` -> [`Kleene::Trap`] (Fault, interleaved `0b11`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Trit16 {
    pos: u16,
    neg: u16,
}

/// Branchless 4-stage SWAR Morton bit-dilation: spreads 16 bits of `x` into
/// the even bit positions (`0, 2, 4, ..., 30`) of a `u32`.
#[inline]
const fn spread16_even(x: u16) -> u32 {
    let mut v = x as u32;
    v = (v | (v << 8)) & 0x00FF_00FF;
    v = (v | (v << 4)) & 0x0F0F_0F0F;
    v = (v | (v << 2)) & 0x3333_3333;
    v = (v | (v << 1)) & 0x5555_5555;
    v
}

/// Branchless 4-stage SWAR Morton bit-compaction: extracts the 16 even bit
/// positions (`0, 2, 4, ..., 30`) of `v` into the low 16 bits of a `u16`.
#[inline]
const fn compact16_even(mut v: u32) -> u16 {
    v &= 0x5555_5555;
    v = (v | (v >> 1)) & 0x3333_3333;
    v = (v | (v >> 2)) & 0x0F0F_0F0F;
    v = (v | (v >> 4)) & 0x00FF_00FF;
    v = (v | (v >> 8)) & 0x0000_FFFF;
    v as u16
}

impl Trit16 {
    /// Creates an all-`Unknown` (`0`) 16-trit word (`pos = 0, neg = 0`).
    pub const fn zero() -> Self {
        Self { pos: 0, neg: 0 }
    }

    /// Lossless constructor from raw positive and negative planes.
    ///
    /// Preserves all 4 states, including any overlapping trap bits (`pos & neg`).
    pub const fn from_planes(pos: u16, neg: u16) -> Self {
        Self { pos, neg }
    }

    /// Strict constructor from raw positive and negative planes.
    ///
    /// Rejects any overlapping trap bits with `Err(TritError::TrapDetected(pos & neg))`.
    pub const fn try_from_planes(pos: u16, neg: u16) -> Result<Self, TritError> {
        let mask = pos & neg;
        if mask != 0 {
            Err(TritError::TrapDetected(mask))
        } else {
            Ok(Self { pos, neg })
        }
    }

    /// Read-only accessor for the positive plane (`u16`).
    pub const fn pos(&self) -> u16 {
        self.pos
    }

    /// Read-only accessor for the negative plane (`u16`).
    pub const fn neg(&self) -> u16 {
        self.neg
    }

    /// Returns `true` iff no trit is in the `Trap` state (`(pos & neg) == 0`).
    pub const fn is_valid(&self) -> bool {
        (self.pos & self.neg) == 0
    }

    /// Returns `true` iff at least one trit is in the `Trap` state (`(pos & neg) != 0`).
    pub const fn has_traps(&self) -> bool {
        (self.pos & self.neg) != 0
    }

    /// Returns the 16-bit mask of all trits currently in the `Trap` state (`pos & neg`).
    pub const fn trap_mask(&self) -> u16 {
        self.pos & self.neg
    }

    /// Reads the state of a single trit at `index` (`0..=15`).
    ///
    /// Distinguishes out-of-range indices (`Err(TritError::InvalidIndex(index))`)
    /// from a valid `Ok(Kleene::Trap)` state.
    pub const fn get(&self, index: u8) -> Result<Kleene, TritError> {
        if index >= 16 {
            return Err(TritError::InvalidIndex(index));
        }
        let mask = 1u16 << index;
        let p = (self.pos & mask) != 0;
        let n = (self.neg & mask) != 0;
        match (p, n) {
            (true, false) => Ok(Kleene::True),
            (false, true) => Ok(Kleene::False),
            (false, false) => Ok(Kleene::Unknown),
            (true, true) => Ok(Kleene::Trap),
        }
    }

    /// Sets the state of a single trit at `index` (`0..=15`), including [`Kleene::Trap`].
    pub fn set(&mut self, index: u8, value: Kleene) -> Result<(), TritError> {
        if index >= 16 {
            return Err(TritError::InvalidIndex(index));
        }
        let mask = 1u16 << index;
        self.pos &= !mask;
        self.neg &= !mask;
        match value {
            Kleene::True => self.pos |= mask,
            Kleene::False => self.neg |= mask,
            Kleene::Unknown => {}
            Kleene::Trap => {
                self.pos |= mask;
                self.neg |= mask;
            }
        }
        Ok(())
    }

    /// Strict setter that rejects [`Kleene::Trap`] with `Err(TritError::TrapDetected(1 << index))`.
    /// Checks index bounds first (`0..=15`).
    pub fn set_strict(&mut self, index: u8, value: Kleene) -> Result<(), TritError> {
        if index >= 16 {
            return Err(TritError::InvalidIndex(index));
        }
        if matches!(value, Kleene::Trap) {
            return Err(TritError::TrapDetected(1u16 << index));
        }
        self.set(index, value)
    }

    // =========================================================================
    // Branchless Bit-Plane Operations (Canonical Hardware Formulas)
    // =========================================================================

    /// Parallel branchless bit-plane Kleene AND:
    /// `pos = self.pos & other.pos`, `neg = self.neg | other.neg`.
    ///
    /// ### Trap Semantics vs. Scalar `Kleene::and`
    /// - Equivalent to [`Kleene::and`] on all 9 trap-free state combinations.
    /// - Because planes are combined independently without a trap mask override:
    ///   - `Trap (1,1) AND True (1,0)    -> Trap (1,1)`
    ///   - `Trap (1,1) AND Trap (1,1)    -> Trap (1,1)`
    ///   - `Trap (1,1) AND False (0,1)   -> False (0,1)` (trap bit cleared in `pos`)
    ///   - `Trap (1,1) AND Unknown (0,0) -> False (0,1)` (trap bit cleared in `pos`)
    /// - Callers requiring trap rejection before evaluation must use [`Trit16::and_strict`].
    pub const fn and(self, other: Self) -> Self {
        Self {
            pos: self.pos & other.pos,
            neg: self.neg | other.neg,
        }
    }

    /// Parallel branchless bit-plane Kleene OR:
    /// `pos = self.pos | other.pos`, `neg = self.neg & other.neg`.
    ///
    /// ### Trap Semantics vs. Scalar `Kleene::or`
    /// - Equivalent to [`Kleene::or`] on all 9 trap-free state combinations.
    /// - Because planes are combined independently without a trap mask override:
    ///   - `Trap (1,1) OR False (0,1)   -> Trap (1,1)`
    ///   - `Trap (1,1) OR Trap (1,1)    -> Trap (1,1)`
    ///   - `Trap (1,1) OR True (1,0)    -> True (1,0)` (trap bit cleared in `neg`)
    ///   - `Trap (1,1) OR Unknown (0,0) -> True (1,0)` (trap bit cleared in `neg`)
    /// - Callers requiring trap rejection before evaluation must use [`Trit16::or_strict`].
    pub const fn or(self, other: Self) -> Self {
        Self {
            pos: self.pos | other.pos,
            neg: self.neg & other.neg,
        }
    }

    /// Parallel branchless bit-plane Kleene NOT (Negation):
    /// swaps `pos` and `neg` planes.
    ///
    /// Preserves `Trap (1,1)` and `Unknown (0,0)`, and swaps `True (1,0)` <-> `False (0,1)`.
    /// Identical to scalar [`Kleene::not`] across all 4 states.
    pub const fn not(self) -> Self {
        Self {
            pos: self.neg,
            neg: self.pos,
        }
    }

    /// Titan 16-trit Consensus / Collision operation (`COLLIDE`):
    ///
    /// ```text
    /// agree_pos = a.pos & b.pos
    /// agree_neg = a.neg & b.neg
    /// conflict  = (a.pos & b.neg) | (a.neg & b.pos)
    /// pos_out   = agree_pos & !conflict
    /// neg_out   = agree_neg & !conflict
    /// ```
    ///
    /// ### Behavior
    /// - Titan-specific consensus operator (not a standard Kleene operator).
    /// - Outputs `True (1,0)` iff both inputs are `True (1,0)`.
    /// - Outputs `False (0,1)` iff both inputs are `False (0,1)`.
    /// - Outputs `Unknown (0,0)` for all other 14 state combinations, **including**
    ///   `(Trap, Trap)` because `Trap (1,1)` sets `conflict = 1` against itself
    ///   (`a.pos & b.neg == 1`), which masks out both `agree_pos` and `agree_neg`.
    pub const fn collide(self, other: Self) -> Self {
        let agree_pos = self.pos & other.pos;
        let agree_neg = self.neg & other.neg;
        let conflict = (self.pos & other.neg) | (self.neg & other.pos);
        Self {
            pos: agree_pos & !conflict,
            neg: agree_neg & !conflict,
        }
    }

    // =========================================================================
    // Strict Operations (Pre-Operation Trap Rejection)
    // =========================================================================

    /// Strict bit-plane AND: rejects if either operand contains any trap bits,
    /// returning the union of their trap masks in `Err(TritError::TrapDetected(mask))`.
    pub const fn and_strict(self, other: Self) -> Result<Self, TritError> {
        let mask = self.trap_mask() | other.trap_mask();
        if mask != 0 {
            Err(TritError::TrapDetected(mask))
        } else {
            Ok(self.and(other))
        }
    }

    /// Strict bit-plane OR: rejects if either operand contains any trap bits,
    /// returning the union of their trap masks in `Err(TritError::TrapDetected(mask))`.
    pub const fn or_strict(self, other: Self) -> Result<Self, TritError> {
        let mask = self.trap_mask() | other.trap_mask();
        if mask != 0 {
            Err(TritError::TrapDetected(mask))
        } else {
            Ok(self.or(other))
        }
    }

    /// Strict bit-plane NOT: rejects if the operand contains any trap bits,
    /// returning `Err(TritError::TrapDetected(self.trap_mask()))`.
    pub const fn not_strict(self) -> Result<Self, TritError> {
        let mask = self.trap_mask();
        if mask != 0 {
            Err(TritError::TrapDetected(mask))
        } else {
            Ok(self.not())
        }
    }

    /// Strict bit-plane COLLIDE: rejects if either operand contains any trap bits,
    /// returning the union of their trap masks in `Err(TritError::TrapDetected(mask))`.
    pub const fn collide_strict(self, other: Self) -> Result<Self, TritError> {
        let mask = self.trap_mask() | other.trap_mask();
        if mask != 0 {
            Err(TritError::TrapDetected(mask))
        } else {
            Ok(self.collide(other))
        }
    }

    // =========================================================================
    // Interleaved 32-Bit Serialization (Branchless SWAR Morton Coding)
    // =========================================================================

    /// Packs the 16-trit word into the canonical 32-bit interleaved format
    /// (2 bits per trit, trit `i` at bits `(2*i)..=(2*i + 1)`).
    ///
    /// Encoding per 2-bit slot:
    /// - `0b00`: `Unknown` (`0`)
    /// - `0b01`: `True` (`+1`, `pos` at even bit `2*i`)
    /// - `0b10`: `False` (`-1`, `neg` at odd bit `2*i + 1`)
    /// - `0b11`: `Trap` (`pos = 1, neg = 1`)
    pub const fn to_u32_interleaved(self) -> u32 {
        spread16_even(self.pos) | (spread16_even(self.neg) << 1)
    }

    /// Losslessly reconstructs a `Trit16` from its 32-bit interleaved format,
    /// preserving all 4 states including `0b11` (`Trap` -> `pos = 1, neg = 1`).
    pub const fn from_u32_interleaved(val: u32) -> Self {
        Self {
            pos: compact16_even(val),
            neg: compact16_even(val >> 1),
        }
    }

    /// Strictly deserializes a `Trit16` from its 32-bit interleaved format,
    /// rejecting the word if any trit slot contains `0b11` (`Trap`).
    ///
    /// Returns the complete 16-bit mask of all `0b11` slots in
    /// `Err(TritError::TrapDetected(mask))`.
    pub const fn try_from_u32_interleaved(val: u32) -> Result<Self, TritError> {
        let decoded = Self::from_u32_interleaved(val);
        let mask = decoded.trap_mask();
        if mask != 0 {
            Err(TritError::TrapDetected(mask))
        } else {
            Ok(decoded)
        }
    }
}

/// Formats `Trit16` as a 16-character bracketed register string `[t15..t0]`
/// where `+` is `True`, `-` is `False`, `0` is `Unknown`, and `!` is `Trap`.
impl fmt::Display for Trit16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;
        for i in (0..16u8).rev() {
            let ch = match self.get(i) {
                Ok(Kleene::True) => "+",
                Ok(Kleene::False) => "-",
                Ok(Kleene::Unknown) => "0",
                Ok(Kleene::Trap) | Err(_) => "!",
            };
            f.write_str(ch)?;
        }
        f.write_str("]")
    }
}
