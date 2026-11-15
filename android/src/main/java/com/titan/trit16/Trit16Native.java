package com.titan.trit16;

/**
 * Low-level JNI bridge to {@code libtitan_trit16_ffi.so}.
 *
 * <p>All entry points pass register-packed 32-bit ({@code int}) or 64-bit ({@code long})
 * primitives without JNI pointer dereferences or heap allocations.
 *
 * <h3>Packed {@code long} Result Envelope for Fallible Operations</h3>
 * <ul>
 *   <li>Bits {@code 0..31}: Result payload (packed {@code (pos << 16) | neg} or {@code Kleene} code)</li>
 *   <li>Bits {@code 32..47}: 16-bit trap bitmask ({@code 0x0000..0xFFFF}) when status is {@code TRAP_DETECTED}</li>
 *   <li>Bits {@code 48..55}: {@link TritError} code ({@code 0 = OK}, {@code 2 = TRAP_DETECTED},
 *       {@code 3 = INVALID_INDEX}, {@code 4 = INVALID_VALUE}, {@code 255 = PANIC})</li>
 * </ul>
 */
public final class Trit16Native {
    private static volatile boolean loaded = false;

    private Trit16Native() {}

    /**
     * Loads {@code libtitan_trit16_ffi.so} from {@code java.library.path} if not already loaded.
     */
    public static synchronized void ensureLoaded() {
        if (!loaded) {
            System.loadLibrary("titan_trit16_ffi");
            loaded = true;
        }
    }

    /**
     * Loads {@code libtitan_trit16_ffi.so} from an explicit absolute path.
     *
     * @param absolutePath Absolute path to the shared library file.
     */
    public static synchronized void loadFromPath(String absolutePath) {
        if (!loaded) {
            System.load(absolutePath);
            loaded = true;
        }
    }

    public static native int abiVersion();

    public static native int fromPlanes(int pos, int neg);

    public static native long tryFromPlanes(int pos, int neg);

    public static native boolean isValid(int rawPlanes);

    public static native boolean hasTraps(int rawPlanes);

    public static native int trapMask(int rawPlanes);

    public static native long getTrit(int rawPlanes, int index);

    public static native long setTrit(int rawPlanes, int index, int kleeneCode);

    public static native long setTritStrict(int rawPlanes, int index, int kleeneCode);

    public static native int andPlanes(int aPlanes, int bPlanes);

    public static native int orPlanes(int aPlanes, int bPlanes);

    public static native int notPlanes(int aPlanes);

    public static native int collidePlanes(int aPlanes, int bPlanes);

    public static native long andStrict(int aPlanes, int bPlanes);

    public static native long orStrict(int aPlanes, int bPlanes);

    public static native long notStrict(int aPlanes);

    public static native long collideStrict(int aPlanes, int bPlanes);

    public static native int toU32Interleaved(int rawPlanes);

    public static native int fromU32Interleaved(int interleaved);

    public static native long tryFromU32Interleaved(int interleaved);

    public static native int kleeneFromI8(byte val);

    public static native long kleeneTryFromI8(byte val);

    public static native long kleeneToI8(int code);

    public static native long kleeneTryToI8(int code);

    public static native long kleeneAnd(int aCode, int bCode);

    public static native long kleeneOr(int aCode, int bCode);

    public static native long kleeneNot(int aCode);

    public static native long kleeneEvaluateTelemetry(int vCode, int tCode, int pCode);
}
