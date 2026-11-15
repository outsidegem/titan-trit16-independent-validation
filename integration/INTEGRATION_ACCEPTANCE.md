# TRIT16 Phase A — Integration Acceptance

## Linux C consumer
- [ ] Compiles separately as a C11 project.
- [ ] Includes ffi/include/titan_trit16.h.
- [ ] Links the built libtitan_trit16_ffi shared library.
- [ ] Executes ABI, construction, access and operation checks.
- [ ] Checks serialization, strict Trap rejection and invalid index.

## Android AAR consumer
- [ ] Builds AAR for arm64-v8a and x86_64.
- [ ] Imports AAR as a binary dependency in a separate Android app.
- [ ] Builds without importing SDK source directories.
- [ ] Runs API 35 x86_64 emulator instrumentation tests.
- [ ] Checks JNI loading, ABI, public API and Trap visibility.

## CI
- [ ] External consumer workflow passes.
- [ ] Existing SDK CI remains green.
- [ ] AAR contains manifest, classes.jar and both native libraries.
- [ ] PR diff contains only reviewed Phase A additions.

## Limitations
Passing Phase A does not prove physical ARM64 Android execution,
Linux ARM64 hardware execution, production deployment readiness,
or safety/security certification. Those require separate validation.


## Phase B — Broader Platform Compatibility

- [ ] Linux ARM64 native workspace tests pass.
- [ ] Linux ARM64 independent C11 shared-library consumer passes.
- [ ] Windows x64 MSVC independent C11 DLL consumer passes.
- [ ] Android AAR artifact is published for physical-device testing.
- [ ] Android consumer APK and instrumentation APK are published.
- [ ] Physical Android ARM64 device test passes.

Phase A emulator validation was performed on x86_64.
The AAR contains arm64-v8a and x86_64 native libraries.
Physical ARM64 Android execution remains a separate acceptance item.
