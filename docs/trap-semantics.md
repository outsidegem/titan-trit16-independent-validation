# Trap Semantics (`trap-semantics.md`)

## 1. Trap Representation

A hardware **Trap** occurs when both the positive and negative bit-planes have bit `i` set simultaneously (`pos[i] == 1 && neg[i] == 1`), corresponding to the 2-bit interleaved pattern `0b11` (`T_ERR`).

## 2. Canonical Trap Behavior Across Layers

| Operation Category | Trap Behavior |
|--------------------|---------------|
| **Scalar `Kleene::and` / `Kleene::or`** | **Trap-dominant:** Any combination with `Kleene::Trap` returns `Kleene::Trap`. |
| **Scalar `Kleene::not` & Bit-plane `Trit16::not`** | **Trap-preserving:** Swapping `pos` and `neg` maps `(1,1)` to `(1,1)`. |
| **Bit-plane `Trit16::and`** | **Plane-independent hardware formula (`pos = a.pos & b.pos, neg = a.neg \| b.neg`):** Preserves `Trap` only on `(Trap, True)`, `(True, Trap)`, and `(Trap, Trap)`. Collapses `(Trap, False)`, `(False, Trap)`, `(Trap, Unknown)`, and `(Unknown, Trap)` to `False (0,1)`. |
| **Bit-plane `Trit16::or`** | **Plane-independent hardware formula (`pos = a.pos \| b.pos, neg = a.neg & b.neg`):** Preserves `Trap` only on `(Trap, False)`, `(False, Trap)`, and `(Trap, Trap)`. Collapses `(Trap, True)`, `(True, Trap)`, `(Trap, Unknown)`, and `(Unknown, Trap)` to `True (1,0)`. |
| **Bit-plane `Trit16::collide`** | **Conflict-clearing consensus (`agree & !conflict`):** Because `Trap (1,1)` has both planes set, `a.pos & b.neg` or `a.neg & b.pos` is `1` whenever the other operand has any bit set (including `Trap` with `Trap`), and `agree` is `0` with `Unknown`. Thus all 7 trap-involving pairs collapse to `Unknown (0,0)`. |
| **Strict Operations (`*_strict`, `try_from_*`)** | **Pre-operation trap rejection:** Checks `trap_mask()` prior to evaluation and returns `Err(TritError::TrapDetected(mask))` containing the complete 16-bit mask of trap positions. |
| **Lossless Serialization (`to_u32_interleaved` / `from_u32_interleaved`)** | **Exact 4-state round-trip:** Preserves `0b11` <-> `(pos=1, neg=1)` without loss. |
| **Strict Deserialization (`try_from_u32_interleaved`)** | **Full-mask rejection:** Rejects any `0b11` slot with `Err(TritError::TrapDetected(mask))`. |
