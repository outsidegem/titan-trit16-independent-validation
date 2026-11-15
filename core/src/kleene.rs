//! Scalar Kleene 4-state logic (`True`, `False`, `Unknown`, `Trap`).
//!
//! Preserves the exact trap-dominant scalar semantics from
//! `~/Titan_Unified_Core/kleene_logic.rs`.

use core::fmt;
use thiserror::Error;

/// Four-state scalar representation of a single trit in the Titan architecture.
///
/// - `True` (`+1`): Positive assertion (plane bits: `pos = 1, neg = 0`, interleaved `0b01`)
/// - `False` (`-1`): Negative assertion (plane bits: `pos = 0, neg = 1`, interleaved `0b10`)
/// - `Unknown` (`0`): Equilibrium / unknown (plane bits: `pos = 0, neg = 0`, interleaved `0b00`)
/// - `Trap`: Hardware fault / illegal overlap (plane bits: `pos = 1, neg = 1`, interleaved `0b11`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kleene {
    True,
    False,
    Unknown,
    Trap,
}

/// Errors returned by strict scalar `Kleene` conversions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum KleeneError {
    #[error("Invalid integer value {0} for strict Kleene conversion (expected -1, 0, or 1)")]
    InvalidInt(i8),
    #[error("Cannot convert Kleene::Trap to a valid balanced ternary integer (-1, 0, 1)")]
    TrapValue,
}

impl Kleene {
    /// Lossless conversion from `i8` matching original `Kleene::from_i8`:
    /// `1 => True`, `-1 => False`, `0 => Unknown`, any other value `=> Trap`.
    pub const fn from_i8(val: i8) -> Self {
        match val {
            1 => Self::True,
            -1 => Self::False,
            0 => Self::Unknown,
            _ => Self::Trap,
        }
    }

    /// Strict conversion from `i8` that rejects values outside `{-1, 0, 1}` with `Err`.
    pub const fn try_from_i8(val: i8) -> Result<Self, KleeneError> {
        match val {
            1 => Ok(Self::True),
            -1 => Ok(Self::False),
            0 => Ok(Self::Unknown),
            other => Err(KleeneError::InvalidInt(other)),
        }
    }

    /// Legacy conversion to `i8` matching original `Kleene::to_i8`.
    ///
    /// Note: This is lossy for `Kleene::Trap` (which maps to `0`), preserved
    /// only for backward compatibility with legacy telemetry callers. Prefer
    /// [`Kleene::try_to_i8`] when traps must not be conflated with `Unknown`.
    pub const fn to_i8(self) -> i8 {
        match self {
            Self::True => 1,
            Self::False => -1,
            Self::Unknown => 0,
            Self::Trap => 0,
        }
    }

    /// Strict conversion to `i8` that returns `Err(KleeneError::TrapValue)` on `Trap`.
    pub const fn try_to_i8(self) -> Result<i8, KleeneError> {
        match self {
            Self::True => Ok(1),
            Self::False => Ok(-1),
            Self::Unknown => Ok(0),
            Self::Trap => Err(KleeneError::TrapValue),
        }
    }

    /// Scalar Kleene AND (Min) with **trap-dominant** propagation.
    ///
    /// Preserves the exact behavior of `Kleene::and` in `kleene_logic.rs`:
    /// if either operand is `Trap`, returns `Trap` before evaluating `False`.
    ///
    /// Note: This differs from raw bit-plane [`crate::Trit16::and`] when a `Trap`
    /// is combined with `False` or `Unknown` (where raw bit-plane formulas yield `False`).
    pub const fn and(a: Self, b: Self) -> Self {
        match (a, b) {
            (Self::Trap, _) | (_, Self::Trap) => Self::Trap,
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            _ => Self::Unknown,
        }
    }

    /// Scalar Kleene OR (Max) with **trap-dominant** propagation.
    ///
    /// Preserves the exact behavior of `Kleene::or` in `kleene_logic.rs`:
    /// if either operand is `Trap`, returns `Trap` before evaluating `True`.
    ///
    /// Note: This differs from raw bit-plane [`crate::Trit16::or`] when a `Trap`
    /// is combined with `True` or `Unknown` (where raw bit-plane formulas yield `True`).
    pub const fn or(a: Self, b: Self) -> Self {
        match (a, b) {
            (Self::Trap, _) | (_, Self::Trap) => Self::Trap,
            (Self::True, _) | (_, Self::True) => Self::True,
            (Self::False, Self::False) => Self::False,
            _ => Self::Unknown,
        }
    }

    /// Scalar Kleene NOT (Negation).
    ///
    /// Swaps `True` and `False`, preserves `Unknown` and `Trap`.
    /// Matches bit-plane [`crate::Trit16::not`] across all 4 states.
    pub const fn not(a: Self) -> Self {
        match a {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
            Self::Trap => Self::Trap,
        }
    }

    /// Pure, deterministic hardware homeostasis telemetry evaluation.
    ///
    /// Evaluates `Kleene::and(Kleene::and(voltage_ok, thermal_ok), packet_ok)`
    /// matching `Kleene::evaluate_telemetry` in `kleene_logic.rs` without I/O side effects:
    /// - Returns `Kleene::Trap` if any sensor line has a hardware trap (`Trap` dominates `False`).
    /// - Returns `Kleene::False` if no trap exists and at least one sensor line failed (`False`).
    /// - Returns `Kleene::Unknown` if no trap or failure exists and at least one sensor is degraded (`Unknown`).
    /// - Returns `Kleene::True` iff all three telemetry lines are nominal (`True`).
    pub const fn evaluate_telemetry(voltage_ok: Self, thermal_ok: Self, packet_ok: Self) -> Self {
        Self::and(Self::and(voltage_ok, thermal_ok), packet_ok)
    }
}

impl fmt::Display for Kleene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::True => "+1",
            Self::False => "-1",
            Self::Unknown => "0",
            Self::Trap => "TRAP",
        };
        f.write_str(s)
    }
}
