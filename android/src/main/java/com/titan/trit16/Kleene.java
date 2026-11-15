package com.titan.trit16;

/**
 * 4-state balanced ternary / Kleene logic scalar value.
 *
 * <p>Scalar {@link #and(Kleene)} and {@link #or(Kleene)} follow canonical
 * <b>trap-dominant</b> semantics ({@code Trap} combined with any operand yields {@code Trap}).
 * For parallel branchless bit-plane operations, see {@link Trit16}.
 */
public enum Kleene {
    /** Unknown / Neutral / Zero ({@code 00}, scalar {@code 0}). */
    UNKNOWN(0, "0"),
    /** True / Positive ({@code 01}, scalar {@code +1}). */
    TRUE(1, "+1"),
    /** False / Negative ({@code 10}, scalar {@code -1}). */
    FALSE(2, "-1"),
    /** Hardware Trap / Sensor Fault ({@code 11}). */
    TRAP(3, "TRAP");

    private final int code;
    private final String symbol;

    Kleene(int code, String symbol) {
        this.code = code;
        this.symbol = symbol;
    }

    public int getCode() {
        return code;
    }

    public static Kleene fromCode(int code) {
        switch (code) {
            case 0:
                return UNKNOWN;
            case 1:
                return TRUE;
            case 2:
                return FALSE;
            case 3:
                return TRAP;
            default:
                throw new TritException.InvalidValue(code);
        }
    }

    /**
     * Lossless conversion from signed byte ({@code -1 -> FALSE}, {@code 0 -> UNKNOWN},
     * {@code +1 -> TRUE}, all other values {@code -> TRAP}).
     */
    public static Kleene fromI8(byte val) {
        return fromCode(Trit16Native.kleeneFromI8(val));
    }

    /**
     * Strict conversion from signed byte ({@code -1, 0, +1} only).
     *
     * @throws TritException.InvalidValue if {@code val} is outside {@code -1..+1}.
     */
    public static Kleene tryFromI8(byte val) {
        int code = TritException.unpackPayloadOrThrow(Trit16Native.kleeneTryFromI8(val), val);
        return fromCode(code);
    }

    /**
     * Lossy conversion to signed byte ({@code TRAP} maps to {@code 0}).
     */
    public byte toI8() {
        int raw = TritException.unpackPayloadOrThrow(Trit16Native.kleeneToI8(this.code), this.code);
        return (byte) raw;
    }

    /**
     * Strict conversion to signed byte.
     *
     * @throws TritException.TrapDetected if this state is {@link #TRAP}.
     */
    public byte tryToI8() {
        int raw = TritException.unpackPayloadOrThrow(Trit16Native.kleeneTryToI8(this.code), this.code);
        return (byte) raw;
    }

    /**
     * Scalar trap-dominant Kleene {@code AND}.
     */
    public Kleene and(Kleene other) {
        int res = TritException.unpackPayloadOrThrow(
            Trit16Native.kleeneAnd(this.code, other.code),
            other.code
        );
        return fromCode(res);
    }

    /**
     * Scalar trap-dominant Kleene {@code OR}.
     */
    public Kleene or(Kleene other) {
        int res = TritException.unpackPayloadOrThrow(
            Trit16Native.kleeneOr(this.code, other.code),
            other.code
        );
        return fromCode(res);
    }

    /**
     * Scalar Kleene {@code NOT} (swaps {@link #TRUE} and {@link #FALSE}; preserves
     * {@link #UNKNOWN} and {@link #TRAP}).
     */
    public Kleene not() {
        int res = TritException.unpackPayloadOrThrow(
            Trit16Native.kleeneNot(this.code),
            this.code
        );
        return fromCode(res);
    }

    /**
     * Pure, side-effect-free ZTPP homeostasis telemetry gate:
     * {@code (voltageOk AND thermalOk) AND packetOk}.
     */
    public static Kleene evaluateTelemetry(Kleene voltageOk, Kleene thermalOk, Kleene packetOk) {
        int res = TritException.unpackPayloadOrThrow(
            Trit16Native.kleeneEvaluateTelemetry(
                voltageOk.code,
                thermalOk.code,
                packetOk.code
            ),
            0
        );
        return fromCode(res);
    }

    @Override
    public String toString() {
        return symbol;
    }
}
