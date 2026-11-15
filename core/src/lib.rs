//! Titan Trit16 Core Library
//!
//! Deterministic, local-first, heap-free implementation of Titan's scalar
//! `Kleene` 4-state logic and 16-trit two-plane (`Trit16`) bit-sliced engine.

#![forbid(unsafe_code)]

mod kleene;
mod trit16;

pub use kleene::{Kleene, KleeneError};
pub use trit16::{Trit16, TritError};
