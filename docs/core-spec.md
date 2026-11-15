# Core Specification (`core-spec.md`)

## 1. Reference Architecture & Two-Plane Encoding

Each `Trit16` word stores 16 trits across two 16-bit bit-planes (`pos: u16`, `neg: u16`):

| State | Balanced Ternary | Plane Bits (`pos[i]`, `neg[i]`) | Interleaved 2-Bit Code |
|-------|:----------------:|:-------------------------------:|:----------------------:|
| `Kleene::Unknown` | `0` | `(0, 0)` | `0b00` |
| `Kleene::True` | `+1` | `(1, 0)` | `0b01` |
| `Kleene::False` | `-1` | `(0, 1)` | `0b10` |
| `Kleene::Trap` | Fault | `(1, 1)` | `0b11` |

## 2. Canonical Semantics: Scalar vs. Bit-Plane

Both implementations from `~/Titan_Unified_Core/kleene_logic.rs` and `rules.h` are preserved without altering their underlying formulas:

1. **Scalar `Kleene` (`Kleene::and`, `Kleene::or`, `Kleene::not`)** implements **trap-dominant** 4-valued logic (`(Trap, _) | (_, Trap) => Trap`).
2. **Bit-plane `Trit16` (`Trit16::and`, `Trit16::or`, `Trit16::not`, `Trit16::collide`)** implements the **canonical branchless hardware bit-plane formulas**:
   - `and`: `pos = a.pos & b.pos`, `neg = a.neg | b.neg`
   - `or`: `pos = a.pos | b.pos`, `neg = a.neg & b.neg`
   - `not`: `pos = a.neg`, `neg = a.pos`
   - `collide`: `agree_pos = a.pos & b.pos`, `agree_neg = a.neg & b.neg`, `conflict = (a.pos & b.neg) | (a.neg & b.pos)`, `pos = agree_pos & !conflict`, `neg = agree_neg & !conflict`
3. **Strict bit-plane variants (`and_strict`, `or_strict`, `not_strict`, `collide_strict`)** enforce **pre-operation trap rejection**, returning `Err(TritError::TrapDetected(mask))` with the union of operand trap masks if any operand contains a trap bit.

### 4×4 Truth Tables (`U = Unknown`, `T = True`, `F = False`, `X = Trap`)

#### Scalar `Kleene::and` vs. Bit-Plane `Trit16::and`

| `A \ B` |Scalar `U`|Scalar `T`|Scalar `F`|Scalar `X`| Bit-Plane `U` `(0,0)` | Bit-Plane `T` `(1,0)` | Bit-Plane `F` `(0,1)` | Bit-Plane `X` `(1,1)` |
|---------|:--------:|:--------:|:--------:|:--------:|:---------------------:|:---------------------:|:---------------------:|:---------------------:|
| **`U` `(0,0)`** | `U` | `U` | `F` | `X` | `U` `(0,0)` | `U` `(0,0)` | `F` `(0,1)` | **`F` `(0,1)`** * |
| **`T` `(1,0)`** | `U` | `T` | `F` | `X` | `U` `(0,0)` | `T` `(1,0)` | `F` `(0,1)` | `X` `(1,1)` |
| **`F` `(0,1)`** | `F` | `F` | `F` | `X` | `F` `(0,1)` | `F` `(0,1)` | `F` `(0,1)` | **`F` `(0,1)`** * |
| **`X` `(1,1)`** | `X` | `X` | `X` | `X` | **`F` `(0,1)`** * | `X` `(1,1)` | **`F` `(0,1)`** * | `X` `(1,1)` |

*\* Divergences (`(U, X)`, `(F, X)`, `(X, U)`, `(X, F)`): In raw bit-plane `AND`, `0` on the positive plane clears `pos` while `1` on the negative plane sets `neg`, collapsing `Trap (1,1)` into `False (0,1)`.*

#### Scalar `Kleene::or` vs. Bit-Plane `Trit16::or`

| `A \ B` |Scalar `U`|Scalar `T`|Scalar `F`|Scalar `X`| Bit-Plane `U` `(0,0)` | Bit-Plane `T` `(1,0)` | Bit-Plane `F` `(0,1)` | Bit-Plane `X` `(1,1)` |
|---------|:--------:|:--------:|:--------:|:--------:|:---------------------:|:---------------------:|:---------------------:|:---------------------:|
| **`U` `(0,0)`** | `U` | `T` | `U` | `X` | `U` `(0,0)` | `T` `(1,0)` | `U` `(0,0)` | **`T` `(1,0)`** * |
| **`T` `(1,0)`** | `T` | `T` | `T` | `X` | `T` `(1,0)` | `T` `(1,0)` | `T` `(1,0)` | **`T` `(1,0)`** * |
| **`F` `(0,1)`** | `U` | `T` | `F` | `X` | `U` `(0,0)` | `T` `(1,0)` | `F` `(0,1)` | `X` `(1,1)` |
| **`X` `(1,1)`** | `X` | `X` | `X` | `X` | **`T` `(1,0)`** * | **`T` `(1,0)`** * | `X` `(1,1)` | `X` `(1,1)` |

*\* Divergences (`(U, X)`, `(T, X)`, `(X, U)`, `(X, T)`): In raw bit-plane `OR`, `0` on the negative plane clears `neg` while `1` on the positive plane sets `pos`, collapsing `Trap (1,1)` into `True (1,0)`.*

#### Unary `NOT` and Binary `COLLIDE`

- **`NOT` (Scalar & Bit-Plane identical):** `U -> U`, `T -> F`, `F -> T`, `X -> X`.
- **`COLLIDE` (Titan-specific consensus operator):**
  - `(T, T) -> T`, `(F, F) -> F`.
  - All other 14 pairs—including `(X, X)`—collapse to `U (0,0)` because `X (1,1)` sets `conflict = (a.pos & b.neg) | (a.neg & b.pos) = 1`.

## 3. Public Rust API (`titan_trit16_core`)

- `Kleene`: `from_i8`, `try_from_i8`, `to_i8`, `try_to_i8`, `and`, `or`, `not`
- `Trit16`:
  - Construction & accessors: `zero`, `from_planes`, `try_from_planes`, `pos`, `neg`
  - Validation: `is_valid`, `has_traps`, `trap_mask`
  - Indexing: `get`, `set`, `set_strict`
  - Bit-plane operations: `and`, `or`, `not`, `collide`
  - Strict operations: `and_strict`, `or_strict`, `not_strict`, `collide_strict`
  - Serialization: `to_u32_interleaved`, `from_u32_interleaved`, `try_from_u32_interleaved`
