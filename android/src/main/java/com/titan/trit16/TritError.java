package com.titan.trit16;

/**
 * Error codes matching {@code TitanError} in {@code titan_trit16_ffi}.
 */
public enum TritError {
    OK(0, "OK"),
    NULL_POINTER(1, "Null or misaligned pointer argument"),
    TRAP_DETECTED(2, "Hardware trap bits detected"),
    INVALID_INDEX(3, "Trit index out of range (must be 0..=15)"),
    INVALID_VALUE(4, "Invalid Kleene discriminant or integer value"),
    INVALID_LENGTH(5, "Invalid batch length"),
    PANIC(255, "Internal panic caught at FFI boundary");

    private final int code;
    private final String description;

    TritError(int code, String description) {
        this.code = code;
        this.description = description;
    }

    public int getCode() {
        return code;
    }

    public String getDescription() {
        return description;
    }

    public static TritError fromCode(int code) {
        switch (code) {
            case 0:
                return OK;
            case 1:
                return NULL_POINTER;
            case 2:
                return TRAP_DETECTED;
            case 3:
                return INVALID_INDEX;
            case 4:
                return INVALID_VALUE;
            case 5:
                return INVALID_LENGTH;
            case 255:
            default:
                return PANIC;
        }
    }
}
