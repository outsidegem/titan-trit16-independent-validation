# Android & JVM Binding Decision (`android-binding-decision.md`)

> [!NOTE]
> Updated in Phase 4 to record the verified register-packed JNI architecture, public Java/Kotlin SDK (`com.titan.trit16`), and `.aar` packaging in `android/`.

## 1. Architecture Evaluation: Direct Register-Packed JNI vs. UniFFI

| Criterion | UniFFI (`0.23+`) | Direct Register-Packed JNI (`ffi/src/jni.rs`) |
|-----------|------------------|-----------------------------------------------|
| **External Rust Dependencies** | Requires `uniffi`, `uniffi_bindgen`, proc-macros | **Zero external crates** (uses standard `extern "system"` ABI) |
| **Runtime JVM Dependencies** | Requires JNA (`com.sun.jna`) in AAR classpath | **Zero external JARs** (standard `System.loadLibrary`) |
| **Call Overhead & Allocation** | Marshals Rust buffers/structs across FFI | **Zero heap allocations** (packs 16-bit planes into `jint`/`jlong` CPU registers) |
| **Offline Host Toolchain** | Requires `uniffi-bindgen` binary + Android NDK | Compiles natively with installed `rustc 1.98.0`, `OpenJDK 21`, and `Gradle 9.8.0` |
| **Phase 3.1 C ABI Preservation** | Separate binding layer | Coexists orthogonally in `libtitan_trit16_ffi.so` alongside all 33 C ABI symbols |

**Decision:** Adopt the **Direct Register-Packed JNI Bridge** (`ffi/src/jni.rs` + `com.titan.trit16.Trit16Native`).

---

## 2. Register-Packed JNI Protocol (`ffi/src/jni.rs`)

Because `Trit16` consists of two 16-bit bit-planes (`pos: u16`, `neg: u16`), every 16-trit word fits losslessly in a single 32-bit signed Java `int` (`(pos << 16) | neg`), including the all-`Trap` pattern (`0xFFFFFFFF`).

For fallible and strict operations (`tryFromPlanes`, `getTrit`, `setTrit`, `setTritStrict`, `andStrict`, `orStrict`, `notStrict`, `collideStrict`, `tryFromU32Interleaved`, and `kleene*`), the JNI layer returns a single 64-bit `jlong` (`long`) envelope:
- **Bits `0..=31` (`u32`):** Result payload (packed `(pos << 16) | neg` or `Kleene` code)
- **Bits `32..=47` (`u16`):** 16-bit `trap_mask` (`pos & neg`) when status is `TRAP_DETECTED`
- **Bits `48..=55` (`u8`):** `TitanError` discriminant (`0 = OK`, `2 = TRAP_DETECTED`, `3 = INVALID_INDEX`, `4 = INVALID_VALUE`, `255 = PANIC`)

This eliminates all JNI pointer dereferences (`JNIEnv*` is never dereferenced), prevents JNI local-reference leaks, catches any potential unwind via `std::panic::catch_unwind`, and preserves all $2^{32}$ valid `Trit16` bit patterns without in-band sentinel collisions.

---

## 3. Exported JNI Symbols (28 Functions)

`libtitan_trit16_ffi.so` exports 28 `Java_com_titan_trit16_Trit16Native_*` symbols alongside the 33 frozen Phase 3.1 C ABI symbols:
- **Metadata & Construction:** `abiVersion`, `fromPlanes`, `tryFromPlanes`, `fromU32Interleaved`, `tryFromU32Interleaved`, `toU32Interleaved`
- **Validation & Per-Trit Access:** `isValid`, `hasTraps`, `trapMask`, `getTrit`, `setTrit`, `setTritStrict`
- **Bit-Plane Logical Operations:** `andPlanes`, `orPlanes`, `notPlanes`, `collidePlanes`
- **Strict Trap-Rejecting Logical Operations:** `andStrict`, `orStrict`, `notStrict`, `collideStrict`
- **Scalar Kleene & Telemetry Operations:** `kleeneFromI8`, `kleeneTryFromI8`, `kleeneToI8`, `kleeneTryToI8`, `kleeneAnd`, `kleeneOr`, `kleeneNot`, `kleeneEvaluateTelemetry`

---

## 4. Public Java & Kotlin SDK (`com.titan.trit16`)

1. **[Trit16Native.java](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/android/src/main/java/com/titan/trit16/Trit16Native.java):** Low-level JNI declarations and thread-safe library loader (`ensureLoaded()`, `loadFromPath(String)`).
2. **[Kleene.java](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/android/src/main/java/com/titan/trit16/Kleene.java):** 4-state scalar enum (`UNKNOWN(0)`, `TRUE(1)`, `FALSE(2)`, `TRAP(3)`) with trap-dominant `and`, `or`, `not`, `evaluateTelemetry`, and `fromI8`/`tryFromI8`/`toI8`/`tryToI8`.
3. **[Trit16.java](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/android/src/main/java/com/titan/trit16/Trit16.java):** Immutable 16-trit vector wrapping `rawPlanes`, exposing both branchless bit-plane operations (`and`, `or`, `not`, `collide`) and strict trap-rejecting operations (`andStrict`, `orStrict`, `notStrict`, `collideStrict`).
4. **[TritError.java](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/android/src/main/java/com/titan/trit16/TritError.java) & [TritException.java](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/android/src/main/java/com/titan/trit16/TritException.java):** Typed exception hierarchy (`TritException.TrapDetected` with 16-bit `getTrapMask()`, `TritException.InvalidIndex`, `TritException.InvalidValue`).
5. **[Trit16Extensions.kt](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/android/src/main/kotlin/com/titan/trit16/Trit16Extensions.kt):** Kotlin `UShort`/`UInt` extensions, infix operators, and `Result<Trit16>` wrappers (`andStrictResult`, `orStrictResult`, `notStrictResult`, `collideStrictResult`).
6. **[TelemetryGovernorSample.java](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/android/src/main/java/com/titan/trit16/TelemetryGovernorSample.java):** Non-root homeostasis evaluation module combining scalar telemetry gating with 16-trit sensor bank collision.

---

## 5. Reproducible Build & AAR Packaging

1. Build the release shared library (`arm64-v8a`):
   ```bash
   cd ffi && cargo build --release -j 1
   cp target/release/libtitan_trit16_ffi.so ../android/src/main/jniLibs/arm64-v8a/
   ```
2. Compile classes, run the 5 JVM/JNI differential test suites, and package `titan-trit16-0.1.0.aar`:
   ```bash
   javac -Xlint:all -Werror -d android/build/classes/main android/src/main/java/com/titan/trit16/*.java
   javac -Xlint:all -Werror -cp android/build/classes/main -d android/build/classes/test android/src/test/java/com/titan/trit16/*.java
   java -ea -cp android/build/classes/main:android/build/classes/test com.titan.trit16.Trit16JniTest android/src/main/jniLibs/arm64-v8a/libtitan_trit16_ffi.so
   ```
3. Resulting `.aar` layout (`android/build/outputs/aar/titan-trit16-0.1.0.aar`):
   - `AndroidManifest.xml` (`package="com.titan.trit16"`, zero permissions, no root requirement)
   - `classes.jar` (`com/titan/trit16/*.class`)
   - `R.txt`
   - `jni/arm64-v8a/libtitan_trit16_ffi.so`
