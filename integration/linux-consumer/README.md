# Independent Linux C Consumer

This is a separate C11 consumer. It includes the SDK public header and links
the compiled shared library. It does not compile SDK source into the app.

From the repository root:

```bash
cargo build --release -p titan_trit16_ffi

cmake -S integration/linux-consumer \
  -B build/trit16-linux-consumer \
  -DTITAN_TRIT16_LIB_DIR="$PWD/target/release"

cmake --build build/trit16-linux-consumer --parallel

LD_LIBRARY_PATH="$PWD/target/release${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
  ctest --test-dir build/trit16-linux-consumer --output-on-failure
```

Checks ABI version, construction, access, operations, serialization,
strict Trap rejection and invalid-index handling.
