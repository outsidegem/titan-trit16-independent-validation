# Independent Android AAR Consumer

This standalone Android application consumes the SDK only through
`app/libs/titan-trit16-0.1.0.aar`.

It does not compile or import the SDK's Java, Kotlin, Rust or JNI source.

The integration workflow builds the AAR, copies it into this application's
libs directory, builds this application, and runs instrumentation tests.

The CI emulator is x86_64 API 35. Physical ARM64 Android testing remains
a separate validation step.
