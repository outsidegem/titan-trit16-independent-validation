package com.titan.trit16;

/**
 * Immutable 16-trit balanced ternary vector backed by two 16-bit bit-planes
 * ({@code pos} in bits {@code 16..31}, {@code neg} in bits {@code 0..15}).
 *
 * <h3>Bit-Plane Encoding</h3>
 * <ul>
 *   <li>{@code 00} ({@code pos=0, neg=0}) = {@link Kleene#UNKNOWN} ({@code 0})</li>
 *   <li>{@code 01} ({@code pos=1, neg=0}) = {@link Kleene#TRUE} ({@code +1})</li>
 *   <li>{@code 10} ({@code pos=0, neg=1}) = {@link Kleene#FALSE} ({@code -1})</li>
 *   <li>{@code 11} ({@code pos=1, neg=1}) = {@link Kleene#TRAP} (hardware fault)</li>
 * </ul>
 *
 * <p>All bit-plane operations ({@link #and}, {@link #or}, {@link #not}, {@link #collide})
 * and strict trap-rejecting operations ({@link #andStrict}, {@link #orStrict},
 * {@link #notStrict}, {@link #collideStrict}) delegate directly to the native Rust
 * {@code titan_trit16_core} implementation via {@link Trit16Native}.
 */
public final class Trit16 {
    private static final Trit16 ZERO_INSTANCE = new Trit16(0);

    private final int rawPlanes;

    private Trit16(int rawPlanes) {
        this.rawPlanes = rawPlanes;
    }

    /**
     * Returns the packed 32-bit plane representation {@code (pos << 16) | neg}.
     */
    public int rawPlanes() {
        return rawPlanes;
    }

    /**
     * Returns an all-{@link Kleene#UNKNOWN} ({@code pos=0, neg=0}) 16-trit word.
     */
    public static Trit16 zero() {
        return ZERO_INSTANCE;
    }

    /**
     * Lossless constructor from raw 16-bit {@code pos} and {@code neg} planes.
     * Preserves all 16-bit patterns including {@code Trap (11)} bits.
     */
    public static Trit16 fromPlanes(int pos, int neg) {
        if (((pos & ~0xFFFF) != 0) || ((neg & ~0xFFFF) != 0)) {
            throw new TritException.InvalidValue(pos | neg);
        }
        return new Trit16(Trit16Native.fromPlanes(pos & 0xFFFF, neg & 0xFFFF));
    }

    /**
     * Strict constructor from 16-bit {@code pos} and {@code neg} planes.
     *
     * @throws TritException.TrapDetected if {@code (pos & neg) != 0}, reporting the exact
     *         16-bit trap bitmask via {@link TritException#getTrapMask()}.
     * @throws TritException.InvalidValue if {@code pos} or {@code neg} is outside {@code 0..0xFFFF}.
     */
    public static Trit16 tryFromPlanes(int pos, int neg) {
        int packed = TritException.unpackPayloadOrThrow(
            Trit16Native.tryFromPlanes(pos, neg),
            pos | neg
        );
        return new Trit16(packed);
    }

    /**
     * Lossless deserialization from a 32-bit interleaved word
     * (trit {@code i} encoded at bit pair {@code (2*i+1, 2*i)} as {@code (neg[i], pos[i])}).
     */
    public static Trit16 fromU32Interleaved(int interleaved) {
        return new Trit16(Trit16Native.fromU32Interleaved(interleaved));
    }

    /**
     * Strict deserialization from a 32-bit interleaved word.
     *
     * @throws TritException.TrapDetected if any trit is {@code Trap (11)}, reporting the full
     *         16-bit trap mask.
     */
    public static Trit16 tryFromU32Interleaved(int interleaved) {
        int packed = TritException.unpackPayloadOrThrow(
            Trit16Native.tryFromU32Interleaved(interleaved),
            interleaved
        );
        return new Trit16(packed);
    }

    /**
     * Returns the 16-bit positive plane ({@code 0x0000..0xFFFF}).
     */
    public int pos() {
        return (rawPlanes >>> 16) & 0xFFFF;
    }

    /**
     * Returns the 16-bit negative plane ({@code 0x0000..0xFFFF}).
     */
    public int neg() {
        return rawPlanes & 0xFFFF;
    }

    /**
     * Returns {@code true} if no trit is in the {@code Trap (11)} state ({@code (pos & neg) == 0}).
     */
    public boolean isValid() {
        return Trit16Native.isValid(rawPlanes);
    }

    /**
     * Returns {@code true} if at least one trit is in the {@code Trap (11)} state.
     */
    public boolean hasTraps() {
        return Trit16Native.hasTraps(rawPlanes);
    }

    /**
     * Returns the 16-bit bitmask of trap positions ({@code pos & neg}).
     */
    public int trapMask() {
        return Trit16Native.trapMask(rawPlanes) & 0xFFFF;
    }

    /**
     * Reads the state of trit {@code index} ({@code 0..15}).
     *
     * @throws TritException.InvalidIndex if {@code index} is outside {@code 0..15}.
     */
    public Kleene get(int index) {
        int code = TritException.unpackPayloadOrThrow(
            Trit16Native.getTrit(rawPlanes, index),
            index
        );
        return Kleene.fromCode(code);
    }

    /**
     * Returns a new {@link Trit16} with trit {@code index} set to {@code value}
     * (allows setting {@link Kleene#TRAP}).
     *
     * @throws TritException.InvalidIndex if {@code index} is outside {@code 0..15}.
     */
    public Trit16 withTrit(int index, Kleene value) {
        int packed = TritException.unpackPayloadOrThrow(
            Trit16Native.setTrit(rawPlanes, index, value.getCode()),
            index
        );
        return new Trit16(packed);
    }

    /**
     * Returns a new {@link Trit16} with trit {@code index} set to {@code value},
     * rejecting {@link Kleene#TRAP}.
     *
     * @throws TritException.TrapDetected if {@code value == Kleene.TRAP} (with {@code trapMask = 1 << index}).
     * @throws TritException.InvalidIndex if {@code index} is outside {@code 0..15}.
     */
    public Trit16 withTritStrict(int index, Kleene value) {
        int packed = TritException.unpackPayloadOrThrow(
            Trit16Native.setTritStrict(rawPlanes, index, value.getCode()),
            index
        );
        return new Trit16(packed);
    }

    /**
     * Branchless 16-trit parallel bit-plane {@code AND}
     * ({@code pos = a.pos & b.pos}, {@code neg = a.neg | b.neg}).
     */
    public Trit16 and(Trit16 other) {
        return new Trit16(Trit16Native.andPlanes(this.rawPlanes, other.rawPlanes));
    }

    /**
     * Branchless 16-trit parallel bit-plane {@code OR}
     * ({@code pos = a.pos | b.pos}, {@code neg = a.neg & b.neg}).
     */
    public Trit16 or(Trit16 other) {
        return new Trit16(Trit16Native.orPlanes(this.rawPlanes, other.rawPlanes));
    }

    /**
     * Branchless 16-trit parallel bit-plane {@code NOT}
     * (swaps {@code pos} and {@code neg} planes; preserves {@code Trap (11)} and {@code Unknown (00)}).
     */
    public Trit16 not() {
        return new Trit16(Trit16Native.notPlanes(this.rawPlanes));
    }

    /**
     * Branchless 16-trit parallel {@code COLLIDE} consensus operation
     * (outputs {@code True} when both are {@code True}, {@code False} when both are {@code False},
     * and {@code Unknown} on all 14 other pairs including {@code (Trap, Trap)}).
     */
    public Trit16 collide(Trit16 other) {
        return new Trit16(Trit16Native.collidePlanes(this.rawPlanes, other.rawPlanes));
    }

    /**
     * Strict 16-trit {@code AND}. Rejects operands containing any {@code Trap (11)} bits
     * prior to performing the operation, reporting the union trap mask {@code a.trapMask() | b.trapMask()}.
     */
    public Trit16 andStrict(Trit16 other) {
        int packed = TritException.unpackPayloadOrThrow(
            Trit16Native.andStrict(this.rawPlanes, other.rawPlanes),
            0
        );
        return new Trit16(packed);
    }

    /**
     * Strict 16-trit {@code OR}. Rejects operands containing any {@code Trap (11)} bits
     * prior to performing the operation, reporting the union trap mask {@code a.trapMask() | b.trapMask()}.
     */
    public Trit16 orStrict(Trit16 other) {
        int packed = TritException.unpackPayloadOrThrow(
            Trit16Native.orStrict(this.rawPlanes, other.rawPlanes),
            0
        );
        return new Trit16(packed);
    }

    /**
     * Strict 16-trit {@code NOT}. Rejects an operand containing any {@code Trap (11)} bits.
     */
    public Trit16 notStrict() {
        int packed = TritException.unpackPayloadOrThrow(
            Trit16Native.notStrict(this.rawPlanes),
            0
        );
        return new Trit16(packed);
    }

    /**
     * Strict 16-trit {@code COLLIDE}. Rejects operands containing any {@code Trap (11)} bits
     * prior to performing the operation, reporting the union trap mask {@code a.trapMask() | b.trapMask()}.
     */
    public Trit16 collideStrict(Trit16 other) {
        int packed = TritException.unpackPayloadOrThrow(
            Trit16Native.collideStrict(this.rawPlanes, other.rawPlanes),
            0
        );
        return new Trit16(packed);
    }

    /**
     * Losslessly serializes this word into a 32-bit interleaved integer
     * (even bits = {@code pos}, odd bits = {@code neg}).
     */
    public int toU32Interleaved() {
        return Trit16Native.toU32Interleaved(this.rawPlanes);
    }

    @Override
    public boolean equals(Object obj) {
        if (this == obj) {
            return true;
        }
        if (!(obj instanceof Trit16)) {
            return false;
        }
        return this.rawPlanes == ((Trit16) obj).rawPlanes;
    }

    @Override
    public int hashCode() {
        return Integer.hashCode(rawPlanes);
    }

    /**
     * Formats the 16-trit register from MSB (trit 15) to LSB (trit 0) as
     * {@code [t15..t0]} using {@code '+'} ({@code True}), {@code '-'} ({@code False}),
     * {@code '0'} ({@code Unknown}), and {@code '!'} ({@code Trap}).
     */
    @Override
    public String toString() {
        StringBuilder sb = new StringBuilder(18);
        sb.append('[');
        int p = pos();
        int n = neg();
        for (int i = 15; i >= 0; i--) {
            boolean pb = ((p >>> i) & 1) != 0;
            boolean nb = ((n >>> i) & 1) != 0;
            if (pb && nb) {
                sb.append('!');
            } else if (pb) {
                sb.append('+');
            } else if (nb) {
                sb.append('-');
            } else {
                sb.append('0');
            }
        }
        sb.append(']');
        return sb.toString();
    }
}
