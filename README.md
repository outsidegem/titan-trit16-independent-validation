# Titan Trit16 SDK

A standalone, local-first Rust implementation of Titan's 16-trit bit-plane and 4-state Kleene logic engine, with stable C11/C++17 ABI bindings, an ARM64 Android/JVM JNI SDK (`.aar`), a local-first Model Context Protocol (MCP) JSON-RPC 2.0 server, and cross-interface conformance verification.

## Components

- **[core/](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/core):** Canonical `Trit16` (16-trit two-plane bit-sliced vector) and `Kleene` (4-state scalar) implementation (`#![forbid(unsafe_code)]`).
- **[ffi/](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/ffi):** Stable C11/C++17 ABI (`33` exported `titan_*` symbols in [titan_trit16.h](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/ffi/include/titan_trit16.h)) and `28` zero-allocation JNI entry points (`Java_com_titan_trit16_Trit16Native_*`).
- **[android/](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/android):** Java/Kotlin Android & JVM SDK (`com.titan.trit16`) and packaged [titan-trit16-0.1.0.aar](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/android/build/outputs/aar/titan-trit16-0.1.0.aar) (`arm64-v8a`, zero permissions, non-root).
- **[titan-mcp/](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/titan-mcp):** Local-first Model Context Protocol (MCP) JSON-RPC 2.0 adapter (`titan-mcp-server`) over Stdio, Unix Domain Socket (`--socket`), or localhost TCP (`--tcp`) with a 64 KiB payload guard.
- **[examples/resource-governor/](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/examples/resource-governor):** Monitor-only ZTPP homeostasis telemetry demo (`titan-resource-governor`) and cross-layer conformance test harness verifying [tests/vectors.json](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/tests/vectors.json).
- **[docs/](file:///data/data/com.termux/files/home/Titan_Trit16_SDK/docs):** Architecture, truth tables, C ABI audit, Android JNI binding specification, and MCP transport specification.

## Design Principles

- Deterministic local execution
- Shared Rust core (`#![forbid(unsafe_code)]`)
- Explicit distinction between scalar trap-dominant Kleene ops, branchless bit-plane ops, and strict trap-rejecting ops
- No mandatory network connection
- No EVM or Node.js dependency in the core
- Exhaustive cross-interface conformance tests

## Build & Verification Commands

```bash
# Run all workspace tests (Rust Core, C11/C++17 FFI, JNI, MCP Server, Cross-Layer Vectors)
cargo test --workspace -j 1

# Run JVM/JNI differential tests on ARM64 OpenJDK
java -ea -cp android/build/classes/main:android/build/classes/test \
  com.titan.trit16.Trit16JniTest \
  "$PWD/android/src/main/jniLibs/arm64-v8a/libtitan_trit16_ffi.so"

# Run the monitor-only homeostasis telemetry demo
cargo run --bin titan-resource-governor -j 1

# Run the local-first MCP JSON-RPC 2.0 server
cargo run --bin titan-mcp-server -j 1 -- --stdio
```
