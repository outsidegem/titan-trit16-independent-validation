# TITAN TRIT16 SDK — PHASE 4 SPECIFICATION AND IMPLEMENTATION PLAN

**Document ID:** `PHASE_4_SPECIFICATION_AND_IMPLEMENTATION_PLAN.md`
**Baseline Status:** Phase 3.1 Verified & Frozen (`titan_trit16_core` v0.1.0, `titan_trit16_ffi` v0.1.0, ABI v1.0.0 / `0x00010000`)
**Target Environment:** Native ARM64 Android/Linux (`aarch64-unknown-linux-android24`, `rustc 1.98.0`, `clang 21.1.8`, `OpenJDK 21.0.12`, `Gradle 9.8.0` / `Kotlin 2.4.10`)

---

## A. Executive Technical Assessment

A comprehensive read-only investigation of `~/Titan_Trit16_SDK`, the reference repository `~/Titan_Unified_Core`, and the host build environment confirms that the **Phase 2 Rust Core (`core/`)** and **Phase 3.1 C ABI (`ffi/`)** form a mathematically sound, memory-safe, deterministic baseline:
- **Core Logic:** All 16 two-operand state combinations (`00=Unknown`, `01=True`, `10=False`, `11=Trap`) across scalar trap-dominant `Kleene` and branchless bit-plane `Trit16` (`and`, `or`, `not`, `collide`), strict trap-rejecting operations (`*_strict`, `try_from_*`), and lossless 32-bit interleaved serialization are verified (`10/10` test suites passing, `0` warnings).
- **C ABI (`ffi/`):** All 33 exported C symbols, compile-time layout assertions (`sizeof(TitanTrit16) == 4`, `alignof == 2`), pointer non-null/alignment checks, raw-pointer batch validation, and C11/C++17 static/shared linkage are verified (`4/4` test suites passing, `0` warnings).

However, **three integration directories in `~/Titan_Trit16_SDK` (`android/`, `titan-mcp/`, `examples/resource-governor/`) are currently empty**, there is no root Cargo workspace unifying lockfiles, and several Phase 1 design documents contain outdated toolchain assumptions (e.g., assuming `sdkmanager`/`cargo-ndk` and unverified third-party crates are installed). Phase 4 must bridge the frozen Phase 3.1 native baseline to the **Android/Kotlin JNI SDK** and **Core hardening/conformance layer**, establishing a verified foundation for Phase 5 (`titan-mcp`) and Phase 6 (`examples/resource-governor` & cross-layer test vectors).

---

## B. Current Architecture and Verified Capabilities

| Layer | Location | Current State | Verified Capabilities |
|-------|----------|---------------|-----------------------|
| **Rust Core** | [core/](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/core) | **Frozen & Verified** | `Kleene` (4-state trap-dominant scalar), `Trit16` (two-plane `pos`/`neg` bit-sliced vector), lossless & strict constructors, per-trit `get`/`set`/`set_strict`, branchless `and`/`or`/`not`/`collide`, `*_strict` variants, interleaved `u32` serialization (`10/10` tests pass). |
| **C ABI (FFI)** | [ffi/](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/ffi) | **Frozen & Verified** | 33 unmangled C functions in [titan_trit16.h](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/ffi/include/titan_trit16.h), ABI version `0x00010000`, alignment/aliasing checks, zero in-band panic sentinels, C11 & C++17 static/shared compatibility (`4/4` tests pass). |
| **Android SDK** | `android/` | **Empty Directory** | Host has `rustc` targeting `aarch64-unknown-linux-android24`, `jni.h`, `OpenJDK 21`, and `Gradle 9.8.0` (Kotlin `2.4.10`), but no JNI bridge or Kotlin classes exist yet. |
| **MCP Server** | `titan-mcp/` | **Empty Directory** | Planned for Phase 5; requires local-first JSON-RPC 2.0 over stdio/Unix domain socket. |
| **Examples** | `examples/resource-governor/` | **Empty Directory** | Planned monitor-only telemetry homeostasis demo. |
| **Documentation** | [docs/](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/docs) | **Partially Outdated** | `core-spec.md` and `trap-semantics.md` are current; `c-compatibility-audit.md`, `android-binding-decision.md`, `mcp-transport-decision.md`, and `implementation-plan.md` contain pre-Phase-3 draft assumptions. |

---

## C. Outstanding Technical Risks and Deficiencies

1. **R-01 (High — Toolchain Reality vs. Phase 1 Android Plan):**
   - [android-binding-decision.md](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/docs/android-binding-decision.md) assumed `uniffi = "0.23"`, `cargo-ndk`, and Android SDK 34 (`sdkmanager`) were installed. Environment inspection shows `kotlinc` (standalone CLI), `ndk-build`, and `sdkmanager` are **not installed**, whereas `OpenJDK 21`, `javac`, `Gradle 9.8.0` (with Kotlin `2.4.10`), `/usr/include/jni.h`, and a native `aarch64-unknown-linux-android24` Rust toolchain **are installed**.
   - Furthermore, UniFFI requires JNA (`com.sun.jna`) at runtime and adds macro/bindgen dependencies, whereas a direct **JNI bridge** (`Java_com_titan_trit16_Trit16Native_*`) over `titan_trit16_ffi` requires **zero external Rust crates**, zero heap allocations (passing 32-bit/64-bit packed primitives directly in registers), and matches `README.md` (`android: Kotlin/JNI Android SDK`).
2. **R-02 (Medium — Missing Compile-Time `unsafe` Lint Enforcement):**
   - `core/src/lib.rs` contains zero `unsafe` code today, but lacks `#![forbid(unsafe_code)]` to prevent future regressions. `ffi/src/lib.rs` lacks `#![deny(unsafe_op_in_unsafe_fn)]`.
3. **R-03 (Medium — Loop-Based Interleaved Serialization Overhead):**
   - `Trit16::to_u32_interleaved` and `from_u32_interleaved` in [trit16.rs](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/core/src/trit16.rs#L276-L315) iterate 16 times with branches. Because `pos[i]` maps to even bits (`2*i`, mask `0x5555_5555`) and `neg[i]` maps to odd bits (`2*i + 1`, mask `0xAAAA_AAAA`), 4-stage SWAR Morton bit-interleaving/deinterleaving can execute branchlessly in ~10 bitwise operations while producing bit-identical outputs across all $2^{32}$ inputs.
4. **R-04 (Medium — Missing ZTPP / Telemetry & Batch Primitives in Core):**
   - `kleene_logic.rs` in `~/Titan_Unified_Core` includes `Kleene::evaluate_telemetry(voltage_ok, thermal_ok, packet_ok)` (needed by `examples/resource-governor`), and `rules.h` / `main.c` include 16-trit human-readable formatting (`[+ - 0 !]`) and 4-trit sensory packet encoding/collision (`encode_titan_state`, `titan_validate_packet`, `titan2_collide`). Currently `core` lacks a pure (side-effect-free) `evaluate_telemetry` helper and `Display` formatting.
5. **R-05 (Low — Split Cargo Lockfiles & Stale Documentation):**
   - `core/Cargo.lock` and `ffi/Cargo.lock` are separate instead of unified under a root Cargo workspace, and `docs/c-compatibility-audit.md` still references pre-Phase-3 symbol names (`CTrit16`, `titan16_make`).

---

## D. Recommended Phase 4 Objectives (Ranked by Priority)

### Priority 1 (Mandatory — Android/JVM Native JNI & Kotlin SDK in `android/`)
1. **JNI Native Bridge:** Expose JNI symbols (`Java_com_titan_trit16_Trit16Native_*`) backed directly by `titan_trit16_core` / `titan_trit16_ffi`, compiled natively to `arm64-v8a` (`libtitan_trit16_jni.so` or integrated into `libtitan_trit16_ffi.so` without altering the 33 C ABI symbols).
2. **Kotlin/JVM Public API (`com.titan.trit16`):**
   - `enum class Kleene(val code: Int)` (`UNKNOWN(0)`, `TRUE(1)`, `FALSE(2)`, `TRAP(3)`)
   - `@JvmInline value class Trit16` or immutable `class Trit16` wrapping `(pos: UShort, neg: UShort)`
   - Sealed/typed exception `TritException` (`TrapDetected(val trapMask: Int)`, `InvalidIndex(val index: Int)`, `InvalidValue(val value: Int)`) and Kotlin `Result<Trit16>` strict wrappers (`tryFromPlanes`, `andStrict`, `orStrict`, `notStrict`, `collideStrict`, `tryFromU32Interleaved`).
3. **Host JVM/Kotlin Verification & AAR Packaging:**
   - Verify runtime execution against `libtitan_trit16_ffi.so` on the host ARM64 JVM (`OpenJDK 21` / `Gradle 9.8.0`) across all 16 state combinations, strict trap masks, and serialization round-trips.
   - Package standard Android AAR layout (`AndroidManifest.xml`, `classes.jar`, `jni/arm64-v8a/libtitan_trit16_ffi.so`, `R.txt`) with zero network permissions and no root requirement.

### Priority 2 (Mandatory — Core & FFI Hardening, Determinism & ZTPP Readiness)
1. **Compile-Time Safety Guards:** Add `#![forbid(unsafe_code)]` to `core/src/lib.rs` and `#![deny(unsafe_op_in_unsafe_fn)]` to `ffi/src/lib.rs`.
2. **SWAR Branchless Interleaved Serialization:** Optimize `Trit16::to_u32_interleaved` and `from_u32_interleaved` using branchless Morton bit-dilation/compaction, verified exhaustively against the loop reference over all $2^{16}$ plane values (`0x0000..=0xFFFF`).
3. **Pure Telemetry & Formatting Helpers in Core:**
   - Add side-effect-free `Kleene::evaluate_telemetry(voltage_ok: Kleene, thermal_ok: Kleene, packet_ok: Kleene) -> Kleene` (`Kleene::and(Kleene::and(voltage_ok, thermal_ok), packet_ok)`).
   - Implement `core::fmt::Display` for `Kleene` (`"0"`, `"+1"`, `"-1"`, `"TRAP"`) and `Trit16` (16-character `[+ - 0 !]` register format preserving traps distinctly as `'!'` rather than `0`).

### Priority 3 (Mandatory — Property/Exhaustive Testing & Documentation Synchronization)
1. **Exhaustive & Deterministic Property Tests:** Add exhaustive $2^{16}$ plane serialization tests and deterministic PRNG (xorshift64) multi-thousand-vector differential tests between Rust Core, C ABI, and JVM/JNI.
2. **Documentation Synchronization:** Update `docs/c-compatibility-audit.md` and `docs/android-binding-decision.md` to match the verified Phase 3.1 C ABI and Phase 4 JNI architecture.

### Priority 4 (Optional / Deferrable to Phase 5–6)
1. **Root Cargo Workspace (`Cargo.toml`):** Unify `core` and `ffi` under a root workspace.
2. **4-Trit Sensory Packet (`SensoryPacket8`) Module:** Optional Rust/C helper for `rules.h` 8-bit sensory packets (`MASK_LOGIC`, `MASK_INTENSITY`, `MASK_CONFIDENCE`, `MASK_CONTEXT`, `titan2_collide`).

---

## E. Detailed Implementation Sequence

1. **Step 4.1 — Core Safety & SWAR Serialization Verification (Backward-Compatible):**
   - Add `#![forbid(unsafe_code)]` to `core/src/lib.rs`.
   - Implement `Kleene::evaluate_telemetry`, `Display` for `Kleene`/`Trit16`, and branchless SWAR `to_u32_interleaved` / `from_u32_interleaved`.
   - Add exhaustive $2^{16}$ plane serialization equivalence test in `core/tests/trit16_tests.rs`.
   - Run `cargo test -j 1` in `core/` (must pass with 0 warnings).
2. **Step 4.2 — JNI Native Bridge (`ffi/` or `android/jni/`):**
   - Add `#![deny(unsafe_op_in_unsafe_fn)]` to `ffi/src/lib.rs`.
   - Implement JNI entry points (`Java_com_titan_trit16_Trit16Native_*`) calling `titan_trit16_core` directly, preserving all 33 existing C ABI symbols untouched.
   - Run `cargo test -j 1` in `ffi/` and verify all 33 C ABI symbols + JNI symbols via `nm -D`.
3. **Step 4.3 — Android Kotlin/Java SDK, Host JVM Tests & AAR Assembly (`android/`):**
   - Create `android/src/main/java/com/titan/trit16/` and `android/src/main/kotlin/com/titan/trit16/` providing the public API (`Trit16`, `Kleene`, `TritException`, `Trit16Native`).
   - Compile and run the JVM/Kotlin cross-language differential test suite against `libtitan_trit16_ffi.so` (`arm64-v8a`).
   - Assemble the verifiable `.aar` archive (`titan-trit16-0.1.0.aar`) containing `AndroidManifest.xml` (zero permissions), `classes.jar`, and `jni/arm64-v8a/libtitan_trit16_ffi.so`.
4. **Step 4.4 — Documentation Synchronization:**
   - Update `docs/android-binding-decision.md` and `docs/c-compatibility-audit.md` with actual build artifacts and symbol tables.

---

## F. Files and Modules Requiring Creation or Modification

| File Path | Action | Purpose |
|-----------|--------|---------|
| `core/src/lib.rs` | Modify | Add `#![forbid(unsafe_code)]`. |
| `core/src/kleene.rs` | Modify | Add `Kleene::evaluate_telemetry` (pure) and `fmt::Display`. |
| `core/src/trit16.rs` | Modify | Add SWAR branchless interleaving and `fmt::Display`. |
| `core/tests/trit16_tests.rs` | Modify | Add exhaustive $2^{16}$ SWAR serialization & telemetry tests. |
| `ffi/src/lib.rs` | Modify | Add `#![deny(unsafe_op_in_unsafe_fn)]` and `jni` module (or separate `ffi/src/jni.rs`) exporting `Java_com_titan_trit16_Trit16Native_*`. |
| `android/AndroidManifest.xml` | Create | Minimal manifest (`package="com.titan.trit16"`, zero permissions). |
| `android/build.gradle.kts` | Create | Reproducible Gradle/JVM build & AAR packaging configuration. |
| `android/src/main/.../com/titan/trit16/*` | Create | Public Kotlin/Java SDK classes (`Kleene`, `Trit16`, `TritException`, `Trit16Native`). |
| `android/src/test/.../com/titan/trit16/*` | Create | Exhaustive JVM/JNI differential test suite. |
| `docs/android-binding-decision.md` | Modify | Document verified JNI vs. UniFFI decision and AAR layout. |
| `docs/c-compatibility-audit.md` | Modify | Align symbol names with frozen Phase 3.1 `TitanTrit16` ABI. |

---

## G. Proposed API or ABI Changes and Compatibility Implications

- **Phase 2 Rust Core API:** **100% Backward-Compatible (Additive Only).** All existing types, methods, signatures, and truth tables remain unchanged. `Kleene::evaluate_telemetry` and `fmt::Display` are purely additive; SWAR serialization is an internal algorithmic optimization with identical input/output behavior across all $2^{32}$ values.
- **Phase 3.1 C ABI (`titan_trit16.h` & 33 symbols):** **ZERO Changes.** `titan_trit16.h` and all 33 exported C symbols remain frozen at ABI version `1.0.0` (`0x00010000`).
- **JNI Symbols:** Exported under the standard `Java_com_titan_trit16_Trit16Native_*` namespace; orthogonal to the 33 `titan_*` C ABI symbols.

---

## H. Required New Tests and Measurable Acceptance Criteria

1. **Exhaustive $2^{16}$ Serialization Verification (`core`):** Verify SWAR `to_u32_interleaved` and `from_u32_interleaved` match bit-by-bit across all $65,536$ single-plane patterns and mixed trap patterns.
2. **JNI Cross-Language Differential Suite (`android`):**
   - Load `libtitan_trit16_ffi.so` via `System.load` in `OpenJDK 21` on ARM64.
   - Verify all 16 `(a, b)` state combinations for `and`, `or`, `not`, `collide` (bit-plane) and scalar `Kleene`.
   - Verify strict operations throw/return `TritException.TrapDetected` with exact 16-bit `trapMask`.
   - Verify all-Trap (`0xFFFF` / `0xFFFFFFFF`) round-trip preservation.
3. **AAR Structural Verification:** Inspect packaged `.aar` via `unzip -l` / `jar tf` to confirm presence of `AndroidManifest.xml` (with zero `<uses-permission>` tags), `classes.jar`, and `jni/arm64-v8a/libtitan_trit16_ffi.so`.
4. **Zero Warnings Gate:** `cargo test -j 1` in `core/` and `ffi/`, `clang -Wall -Wextra -Wpedantic -Werror` (C11 & C++17), and JVM compilation must complete with `0` warnings and `0` failures.

---

## I. Regression Prevention and Rollback Strategy

- **Compile-Time Guards:** `#![forbid(unsafe_code)]` in `core`, compile-time layout assertions in `ffi` and `titan_trit16.h`, and symbol-count verification (`nm -D` checking all 33 C ABI symbols).
- **Differential Locking:** Existing 10 `core` tests and 4 `ffi` test suites must run unmodified before and after each step.
- **Rollback Strategy:** All Phase 4 work is isolated to additive methods in `core`, an orthogonal `jni` module in `ffi`, and new files in `android/`. If any step fails verification, changes in `android/` or the JNI module can be reverted cleanly without affecting the Phase 3.1 C ABI or Phase 2 Core.

---

## J. Dependencies, Resource Requirements, and Build Considerations

- **Zero New External Rust Crates:** Both `core` (depends only on `thiserror = "2"`) and `ffi` (depends only on `titan_trit16_core` and `std`) require **no new crates.io dependencies** for JNI (standard C JNI types `JNIEnv`, `jclass`, `jint`, `jlong`, `jboolean` are thin `#[repr(C)]` pointers/primitives).
- **Resource-Conscious Execution:** All Cargo builds and test runs must use `-j 1` (`CARGO_BUILD_JOBS=1`) to prevent memory exhaustion on constrained ARM64 mobile hardware.
- **Offline/Local Build Capability:** Using standard JNI + `javac`/`jar`/Gradle with pre-installed `OpenJDK 21` avoids network downloads of Android SDK platforms or Maven dependencies.

---

## K. Explicit Separation of Mandatory Work vs. Optional Improvements

| Category | Scope Items |
|----------|-------------|
| **Mandatory (Phase 4 Core Scope)** | 1. `#![forbid(unsafe_code)]` in `core` and `#![deny(unsafe_op_in_unsafe_fn)]` in `ffi`.<br>2. Pure `Kleene::evaluate_telemetry` and `Display` implementations in `core`.<br>3. SWAR branchless `to_u32_interleaved` / `from_u32_interleaved` + exhaustive $2^{16}$ test.<br>4. Native ARM64 JNI bridge + Kotlin/JVM SDK in `android/` calling the Rust core.<br>5. Host JVM/JNI 16-state differential test suite & verified `.aar` package (`arm64-v8a`).<br>6. Update `docs/android-binding-decision.md` and `docs/c-compatibility-audit.md`. |
| **Optional (Deferrable to Phase 5/6)** | 1. Root Cargo workspace `Cargo.toml` (can be added alongside `titan-mcp` in Phase 5).<br>2. 4-trit sensory byte packet (`SensoryPacket8`) helpers from `rules.h`.<br>3. Multi-word batch slice operations (`and_batch`, `or_batch`, `collide_batch`) in `core`. |
