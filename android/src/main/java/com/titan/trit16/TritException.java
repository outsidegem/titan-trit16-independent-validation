package com.titan.trit16;

/**
 * Exception thrown by strict Titan Trit16 operations when a hardware trap state
 * ({@code 11}), invalid trit index, or invalid state representation is encountered.
 */
public class TritException extends RuntimeException {
    private static final long serialVersionUID = 1L;

    private final TritError error;
    private final int trapMask;

    public TritException(TritError error, int trapMask, String message) {
        super(message);
        this.error = error;
        this.trapMask = trapMask & 0xFFFF;
    }

    public TritError getError() {
        return error;
    }

    /**
     * Returns the 16-bit bitmask of trap positions ({@code pos & neg}) when
     * {@link #getError()} is {@link TritError#TRAP_DETECTED}, or {@code 0} otherwise.
     */
    public int getTrapMask() {
        return trapMask;
    }

    /**
     * Thrown when a strict operation encounters one or more {@code Trap (11)} trits.
     */
    public static final class TrapDetected extends TritException {
        private static final long serialVersionUID = 1L;

        public TrapDetected(int trapMask) {
            super(
                TritError.TRAP_DETECTED,
                trapMask,
                String.format("Hardware trap detected at bitmask 0x%04X", trapMask & 0xFFFF)
            );
        }
    }

    /**
     * Thrown when a trit index is outside {@code 0..15}.
     */
    public static final class InvalidIndex extends TritException {
        private static final long serialVersionUID = 1L;

        private final int index;

        public InvalidIndex(int index) {
            super(
                TritError.INVALID_INDEX,
                0,
                "Trit index out of range: " + index + " (expected 0..15)"
            );
            this.index = index;
        }

        public int getIndex() {
            return index;
        }
    }

    /**
     * Thrown when an integer or plane value is outside the valid representation domain.
     */
    public static final class InvalidValue extends TritException {
        private static final long serialVersionUID = 1L;

        private final int value;

        public InvalidValue(int value) {
            super(
                TritError.INVALID_VALUE,
                0,
                "Invalid Kleene or plane value: " + value
            );
            this.value = value;
        }

        public int getValue() {
            return value;
        }
    }

    static int unpackPayloadOrThrow(long envelope, int contextValue) {
        int status = (int) ((envelope >>> 48) & 0xFFL);
        if (status == 0) {
            return (int) (envelope & 0xFFFFFFFFL);
        }
        int mask = (int) ((envelope >>> 32) & 0xFFFFL);
        TritError err = TritError.fromCode(status);
        switch (err) {
            case TRAP_DETECTED:
                throw new TrapDetected(mask);
            case INVALID_INDEX:
                throw new InvalidIndex(contextValue);
            case INVALID_VALUE:
                throw new InvalidValue(contextValue);
            default:
                throw new TritException(err, mask, err.getDescription());
        }
    }
}
