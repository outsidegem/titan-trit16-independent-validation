# C Compatibility & ABI Audit (`c-compatibility-audit.md`)

> [!NOTE]
> Updated in Phase 4 to record the frozen Phase 3.1 C ABI ([titan_trit16.h](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/ffi/include/titan_trit16.h)) alongside the line-by-line comparison between `rules.h` (`Titan_Unified_Core`), `titan_trit16_core`, and `titan_trit16_ffi`.

## 1. Binary Layout & Compile-Time Assertions

| Aspect | Reference C (`rules.h`) | Rust Core (`core`) | Stable C ABI (`ffi/include/titan_trit16.h`) |
|--------|-------------------------|--------------------|---------------------------------------------|
| **Word Type** | `typedef union { uint32_t raw; struct { uint16_t neg; uint16_t pos; } plane; } titan16_word_t;` | `pub struct Trit16 { pos: u16, neg: u16 }` (private fields) | `typedef struct TitanTrit16 { uint16_t neg; uint16_t pos; } TitanTrit16;` |
| **Size & Alignment** | 4 bytes, 2-byte aligned | 4 bytes, 2-byte aligned | Enforced via `_Static_assert`: `sizeof == 4`, `alignof == 2`, `offsetof(neg) == 0`, `offsetof(pos) == 2` |
| **Enum Representation** | Implicit C `int` / `#define` masks | `Kleene`, `TritError` | Fixed-width `int32_t` across ABI boundary (`sizeof(TitanKleene) == 4`, `sizeof(TitanError) == 4`) |

---

## 2. Construction, Validation & Trap Detection

| Operation | Reference C (`rules.h`) | Rust Core (`Trit16`) | C ABI (`titan_trit16_*`) |
|-----------|-------------------------|----------------------|--------------------------|
| **Zero Word** | `{ .raw = 0 }` | `Trit16::zero()` | `titan_trit16_zero()` |
| **Lossless Planes** | `titan16_make(pos, neg)` | `Trit16::from_planes(pos, neg)` | `titan_trit16_from_planes(pos, neg)` |
| **Strict Planes** | *(not provided)* | `Trit16::try_from_planes(pos, neg)` | `titan_trit16_try_from_planes(pos, neg, &out, &trap_mask)` |
| **Validity Check** | `titan16_is_valid(w)` (`(pos & neg) == 0`) | `w.is_valid()` / `w.has_traps()` | `titan_trit16_is_valid(w)` / `titan_trit16_has_traps(w)` |
| **Trap Mask** | `titan16_detect_traps(w)` (`pos & neg`) | `w.trap_mask()` | `titan_trit16_trap_mask(w)` |
| **Batch Validation** | *(not provided)* | Iterator over `trap_mask()` | `titan_trit16_validate_batch(words, len, out_masks, &out_trap_count)` (aliasing-safe raw pointer loop) |

---

## 3. Per-Trit Access & Mutation

| Operation | Reference C (`rules.h`) | Rust Core (`Trit16`) | C ABI (`titan_trit16_*`) |
|-----------|-------------------------|----------------------|--------------------------|
| **Read Trit** | `titan16_get_trit(w, idx)` returns `0` on `Trap` (lossy) and `0` on `idx >= 16` | `w.get(idx)` returns `Result<Kleene, TritError>` (preserves `Kleene::Trap`) | `titan_trit16_get(w, idx, &out_state)` (`TITAN_ERR_INVALID_INDEX` on `idx >= 16`) |
| **Write Trit (Lossless)** | `titan16_set_trit(&w, idx, val)` (cannot set `Trap`) | `w.set(idx, val)` (can set `Kleene::Trap`) | `titan_trit16_set(&w, idx, val)` |
| **Write Trit (Strict)** | *(not provided)* | `w.set_strict(idx, val)` (rejects `Kleene::Trap`) | `titan_trit16_set_strict(&w, idx, val)` |

---

## 4. Logical Operations (Bit-Plane vs. Strict vs. Scalar)

| Operation | Reference C (`rules.h`) | Rust Core (`Trit16` / `Kleene`) | C ABI (`titan_trit16_*` / `titan_kleene_*`) |
|-----------|-------------------------|---------------------------------|---------------------------------------------|
| **Bit-Plane AND** | `pos = a.pos & b.pos; neg = a.neg \| b.neg;` | `a.and(b)` | `titan_trit16_and(a, b)` |
| **Bit-Plane OR** | `pos = a.pos \| b.pos; neg = a.neg & b.neg;` | `a.or(b)` | `titan_trit16_or(a, b)` |
| **Bit-Plane NOT** | `pos = a.neg; neg = a.pos;` | `a.not()` | `titan_trit16_not(a)` |
| **Bit-Plane COLLIDE** | Consensus (`~conflict`, clears `(Trap, Trap)` to `Unknown`) | `a.collide(b)` | `titan_trit16_collide(a, b)` |
| **Strict Bit-Plane Ops** | *(not provided)* | `and_strict`, `or_strict`, `not_strict`, `collide_strict` | `titan_trit16_and_strict`, `or_strict`, `not_strict`, `collide_strict` (reports union trap mask `(a.pos & a.neg) \| (b.pos & b.neg)`) |
| **Scalar Kleene Ops** | In `kleene_logic.rs` (trap-dominant) | `Kleene::and`, `or`, `not`, `evaluate_telemetry` | `titan_kleene_and`, `titan_kleene_or`, `titan_kleene_not` |

---

## 5. Interleaved 32-Bit Serialization

| Operation | Reference C (`rules.h`) | Rust Core (`Trit16`) | C ABI (`titan_trit16_*`) |
|-----------|-------------------------|----------------------|--------------------------|
| **Serialize (`u32`)** | `titan16_to_interleaved(w)` (16-step loop) | `w.to_u32_interleaved()` (4-stage SWAR branchless Morton bit-dilation, bit-identical across all $2^{32}$ inputs) | `titan_trit16_to_u32_interleaved(w)` |
| **Deserialize (Lossless)** | `titan16_from_interleaved(raw)` (**bug in C**: drops `T_ERR = 0b11` trap bits!) | `Trit16::from_u32_interleaved(raw)` (preserves all `11` trap bits losslessly via SWAR compaction) | `titan_trit16_from_u32_interleaved(raw)` |
| **Deserialize (Strict)** | *(not provided)* | `Trit16::try_from_u32_interleaved(raw)` (reports full 16-bit trap mask) | `titan_trit16_try_from_u32_interleaved(raw, &out, &trap_mask)` |

---

## 6. FFI Memory Safety & Error Semantics Summary (Phase 3.1.2 Baseline)

1. **Zero In-Band Panic Sentinels:** All $2^{32}$ `TitanTrit16` bit patterns (including all-`Trap` `{ neg: 0xFFFF, pos: 0xFFFF }` and interleaved `0xFFFFFFFF`) are valid data values. Value-returning functions execute mathematically infallible `const fn` bitwise operations directly without reserving `0xFFFF` as a sentinel.
2. **Pointer & Alignment Validation:** Every pointer parameter is checked for both `.is_null()` and `.is_aligned()` before dereferencing, returning `TITAN_ERR_NULL_POINTER (1)` on failure.
3. **Unwind Protection:** Every fallible FFI entry point wraps execution in `std::panic::catch_unwind`, returning `TITAN_ERR_PANIC (255)` if an internal panic occurs under `panic = "unwind"`.
4. **Exported Symbol Table:** Verified 33 `titan_*` C ABI symbols + 28 `Java_com_titan_trit16_Trit16Native_*` JNI symbols in `libtitan_trit16_ffi.so`.
