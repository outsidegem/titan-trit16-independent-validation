package com.titan.trit16;

/**
 * Exhaustive JVM/JNI differential and regression test suite for {@code com.titan.trit16}.
 *
 * <p>Executes directly against {@code libtitan_trit16_ffi.so} on ARM64 Linux/Android.
 */
public final class Trit16JniTest {
    private static final Kleene[] ALL_STATES = {
        Kleene.UNKNOWN,
        Kleene.TRUE,
        Kleene.FALSE,
        Kleene.TRAP
    };

    private static void assertTrue(boolean cond, String msg) {
        if (!cond) {
            throw new AssertionError("Assertion failed: " + msg);
        }
    }

    private static void assertEquals(int actual, int expected, String msg) {
        if (actual != expected) {
            throw new AssertionError(
                String.format("%s: expected 0x%X (%d), got 0x%X (%d)", msg, expected, expected, actual, actual)
            );
        }
    }

    private static void assertEqualsObj(Object actual, Object expected, String msg) {
        if (actual == null ? expected != null : !actual.equals(expected)) {
            throw new AssertionError(msg + ": expected <" + expected + ">, got <" + actual + ">");
        }
    }

    public static void main(String[] args) {
        if (args.length > 0 && !args[0].isEmpty()) {
            Trit16Native.loadFromPath(args[0]);
        } else {
            Trit16Native.ensureLoaded();
        }

        testAbiVersionAndAllTrapPreservation();
        System.out.println("[PASS] testAbiVersionAndAllTrapPreservation");

        testExhaustive16StateScalarAndBitPlane();
        System.out.println("[PASS] testExhaustive16StateScalarAndBitPlane");

        testStrictTrapRejectionAndErrorReporting();
        System.out.println("[PASS] testStrictTrapRejectionAndErrorReporting");

        testDeterministicPrng10000Vectors();
        System.out.println("[PASS] testDeterministicPrng10000Vectors");

        testTelemetryGovernorSample();
        System.out.println("[PASS] testTelemetryGovernorSample");

        System.out.println("ALL 5 JVM/JNI DIFFERENTIAL TEST SUITES PASSED.");
    }

    private static void testAbiVersionAndAllTrapPreservation() {
        assertEquals(Trit16Native.abiVersion(), 0x00010000, "ABI version must be 1.0.0 (0x00010000)");

        Trit16 zero = Trit16.zero();
        assertEquals(zero.pos(), 0x0000, "zero.pos");
        assertEquals(zero.neg(), 0x0000, "zero.neg");
        assertTrue(zero.isValid(), "zero must be valid");
        assertTrue(!zero.hasTraps(), "zero must not have traps");
        assertEquals(zero.trapMask(), 0x0000, "zero.trapMask");

        // All-Trap (0xFFFF / 0xFFFFFFFF) must be preserved without sentinel collision
        Trit16 allTrap = Trit16.fromPlanes(0xFFFF, 0xFFFF);
        assertEquals(allTrap.pos(), 0xFFFF, "allTrap.pos");
        assertEquals(allTrap.neg(), 0xFFFF, "allTrap.neg");
        assertTrue(!allTrap.isValid(), "allTrap.isValid must be false");
        assertTrue(allTrap.hasTraps(), "allTrap.hasTraps must be true");
        assertEquals(allTrap.trapMask(), 0xFFFF, "allTrap.trapMask");
        assertEqualsObj(allTrap.and(allTrap), allTrap, "allTrap AND allTrap");
        assertEqualsObj(allTrap.or(allTrap), allTrap, "allTrap OR allTrap");
        assertEqualsObj(allTrap.not(), allTrap, "NOT allTrap");
        assertEqualsObj(allTrap.collide(allTrap), zero, "allTrap COLLIDE allTrap -> zero");
        assertEquals(allTrap.toU32Interleaved(), 0xFFFFFFFF, "allTrap interleaved u32");
        assertEqualsObj(Trit16.fromU32Interleaved(0xFFFFFFFF), allTrap, "fromU32Interleaved(0xFFFFFFFF)");
        assertEqualsObj(zero.toString(), "[0000000000000000]", "zero.toString()");
        assertEqualsObj(allTrap.toString(), "[!!!!!!!!!!!!!!!!]", "allTrap.toString()");
        assertEqualsObj(
            Trit16.fromPlanes(0x00FF, 0xFF00).toString(),
            "[--------++++++++]",
            "sensor bank toString()"
        );

        // Verify Kleene scalar i8 conversions and Display strings
        assertEqualsObj(Kleene.fromI8((byte) 1), Kleene.TRUE, "fromI8(1)");
        assertEqualsObj(Kleene.fromI8((byte) -1), Kleene.FALSE, "fromI8(-1)");
        assertEqualsObj(Kleene.fromI8((byte) 0), Kleene.UNKNOWN, "fromI8(0)");
        assertEqualsObj(Kleene.fromI8((byte) 2), Kleene.TRAP, "fromI8(2)");
        assertEqualsObj(Kleene.tryFromI8((byte) 1), Kleene.TRUE, "tryFromI8(1)");
        assertEqualsObj(Kleene.tryFromI8((byte) -1), Kleene.FALSE, "tryFromI8(-1)");
        assertEqualsObj(Kleene.tryFromI8((byte) 0), Kleene.UNKNOWN, "tryFromI8(0)");
        try {
            Kleene.tryFromI8((byte) 2);
            throw new AssertionError("Expected InvalidValue from tryFromI8(2)");
        } catch (TritException.InvalidValue e) {
            assertEquals(e.getValue(), 2, "tryFromI8(2) invalid value");
        }
        assertEquals(Kleene.TRUE.toI8(), 1, "TRUE.toI8()");
        assertEquals(Kleene.FALSE.toI8(), -1, "FALSE.toI8()");
        assertEquals(Kleene.UNKNOWN.toI8(), 0, "UNKNOWN.toI8()");
        assertEquals(Kleene.TRAP.toI8(), 0, "TRAP.toI8() legacy lossy");
        assertEquals(Kleene.TRUE.tryToI8(), 1, "TRUE.tryToI8()");
        assertEquals(Kleene.FALSE.tryToI8(), -1, "FALSE.tryToI8()");
        assertEquals(Kleene.UNKNOWN.tryToI8(), 0, "UNKNOWN.tryToI8()");
        try {
            Kleene.TRAP.tryToI8();
            throw new AssertionError("Expected TrapDetected from TRAP.tryToI8()");
        } catch (TritException.TrapDetected e) {
            assertEquals(e.getTrapMask(), 1, "TRAP.tryToI8() trapMask");
        }
        assertEqualsObj(Kleene.TRUE.toString(), "+1", "TRUE.toString()");
        assertEqualsObj(Kleene.FALSE.toString(), "-1", "FALSE.toString()");
        assertEqualsObj(Kleene.UNKNOWN.toString(), "0", "UNKNOWN.toString()");
        assertEqualsObj(Kleene.TRAP.toString(), "TRAP", "TRAP.toString()");
    }

    private static Kleene refScalarAnd(Kleene a, Kleene b) {
        if (a == Kleene.TRAP || b == Kleene.TRAP) return Kleene.TRAP;
        if (a == Kleene.FALSE || b == Kleene.FALSE) return Kleene.FALSE;
        if (a == Kleene.UNKNOWN || b == Kleene.UNKNOWN) return Kleene.UNKNOWN;
        return Kleene.TRUE;
    }

    private static Kleene refScalarOr(Kleene a, Kleene b) {
        if (a == Kleene.TRAP || b == Kleene.TRAP) return Kleene.TRAP;
        if (a == Kleene.TRUE || b == Kleene.TRUE) return Kleene.TRUE;
        if (a == Kleene.UNKNOWN || b == Kleene.UNKNOWN) return Kleene.UNKNOWN;
        return Kleene.FALSE;
    }

    private static Kleene refScalarNot(Kleene a) {
        switch (a) {
            case TRUE: return Kleene.FALSE;
            case FALSE: return Kleene.TRUE;
            case UNKNOWN: return Kleene.UNKNOWN;
            case TRAP:
            default: return Kleene.TRAP;
        }
    }

    private static Kleene refBitPlaneAnd(Kleene a, Kleene b) {
        int ap = (a == Kleene.TRUE || a == Kleene.TRAP) ? 1 : 0;
        int an = (a == Kleene.FALSE || a == Kleene.TRAP) ? 1 : 0;
        int bp = (b == Kleene.TRUE || b == Kleene.TRAP) ? 1 : 0;
        int bn = (b == Kleene.FALSE || b == Kleene.TRAP) ? 1 : 0;
        int rp = ap & bp;
        int rn = an | bn;
        if (rp == 1 && rn == 1) return Kleene.TRAP;
        if (rp == 1) return Kleene.TRUE;
        if (rn == 1) return Kleene.FALSE;
        return Kleene.UNKNOWN;
    }

    private static Kleene refBitPlaneOr(Kleene a, Kleene b) {
        int ap = (a == Kleene.TRUE || a == Kleene.TRAP) ? 1 : 0;
        int an = (a == Kleene.FALSE || a == Kleene.TRAP) ? 1 : 0;
        int bp = (b == Kleene.TRUE || b == Kleene.TRAP) ? 1 : 0;
        int bn = (b == Kleene.FALSE || b == Kleene.TRAP) ? 1 : 0;
        int rp = ap | bp;
        int rn = an & bn;
        if (rp == 1 && rn == 1) return Kleene.TRAP;
        if (rp == 1) return Kleene.TRUE;
        if (rn == 1) return Kleene.FALSE;
        return Kleene.UNKNOWN;
    }

    private static Kleene refCollide(Kleene a, Kleene b) {
        if (a == Kleene.TRUE && b == Kleene.TRUE) return Kleene.TRUE;
        if (a == Kleene.FALSE && b == Kleene.FALSE) return Kleene.FALSE;
        return Kleene.UNKNOWN;
    }

    private static void testExhaustive16StateScalarAndBitPlane() {
        // Pack all 16 (a, b) state pairs into one 16-trit word
        Trit16 wordA = Trit16.zero();
        Trit16 wordB = Trit16.zero();
        int idx = 0;
        for (Kleene a : ALL_STATES) {
            assertEqualsObj(a.not(), refScalarNot(a), "Scalar NOT " + a);
            for (Kleene b : ALL_STATES) {
                assertEqualsObj(a.and(b), refScalarAnd(a, b), "Scalar AND (" + a + "," + b + ")");
                assertEqualsObj(a.or(b), refScalarOr(a, b), "Scalar OR (" + a + "," + b + ")");
                wordA = wordA.withTrit(idx, a);
                wordB = wordB.withTrit(idx, b);
                idx++;
            }
        }

        Trit16 resAnd = wordA.and(wordB);
        Trit16 resOr = wordA.or(wordB);
        Trit16 resNot = wordA.not();
        Trit16 resCol = wordA.collide(wordB);

        idx = 0;
        for (Kleene a : ALL_STATES) {
            for (Kleene b : ALL_STATES) {
                assertEqualsObj(resAnd.get(idx), refBitPlaneAnd(a, b), "Bit-plane AND at " + idx);
                assertEqualsObj(resOr.get(idx), refBitPlaneOr(a, b), "Bit-plane OR at " + idx);
                assertEqualsObj(resNot.get(idx), refScalarNot(a), "Bit-plane NOT at " + idx);
                assertEqualsObj(resCol.get(idx), refCollide(a, b), "Bit-plane COLLIDE at " + idx);
                idx++;
            }
        }

        // Explicitly verify the critical semantic cases from Phase 2 audit
        // Trap AND Unknown -> Scalar: TRAP, Bit-Plane: FALSE
        assertEqualsObj(Kleene.TRAP.and(Kleene.UNKNOWN), Kleene.TRAP, "Scalar Trap AND Unknown");
        assertEqualsObj(
            Trit16.zero().withTrit(0, Kleene.TRAP).and(Trit16.zero().withTrit(0, Kleene.UNKNOWN)).get(0),
            Kleene.FALSE,
            "Bit-plane Trap AND Unknown"
        );
        // Trap OR True -> Scalar: TRAP, Bit-Plane: TRUE
        assertEqualsObj(Kleene.TRAP.or(Kleene.TRUE), Kleene.TRAP, "Scalar Trap OR True");
        assertEqualsObj(
            Trit16.zero().withTrit(0, Kleene.TRAP).or(Trit16.zero().withTrit(0, Kleene.TRUE)).get(0),
            Kleene.TRUE,
            "Bit-plane Trap OR True"
        );
        // Trap COLLIDE Trap -> UNKNOWN
        assertEqualsObj(
            Trit16.zero().withTrit(0, Kleene.TRAP).collide(Trit16.zero().withTrit(0, Kleene.TRAP)).get(0),
            Kleene.UNKNOWN,
            "Bit-plane Trap COLLIDE Trap"
        );
    }

    private static void testStrictTrapRejectionAndErrorReporting() {
        // tryFromPlanes with traps at bits 1 and 12 (0x1002)
        try {
            Trit16.tryFromPlanes(0x1002, 0x1002);
            throw new AssertionError("Expected TrapDetected from tryFromPlanes");
        } catch (TritException.TrapDetected e) {
            assertEquals(e.getTrapMask(), 0x1002, "tryFromPlanes trapMask");
        }

        // andStrict / orStrict / collideStrict union trap mask
        Trit16 a = Trit16.fromPlanes(0x0004, 0x0004); // trap at bit 2
        Trit16 b = Trit16.fromPlanes(0x0200, 0x0200); // trap at bit 9
        try {
            a.andStrict(b);
            throw new AssertionError("Expected TrapDetected from andStrict");
        } catch (TritException.TrapDetected e) {
            assertEquals(e.getTrapMask(), 0x0204, "andStrict union trapMask");
        }
        try {
            a.orStrict(b);
            throw new AssertionError("Expected TrapDetected from orStrict");
        } catch (TritException.TrapDetected e) {
            assertEquals(e.getTrapMask(), 0x0204, "orStrict union trapMask");
        }
        try {
            a.collideStrict(b);
            throw new AssertionError("Expected TrapDetected from collideStrict");
        } catch (TritException.TrapDetected e) {
            assertEquals(e.getTrapMask(), 0x0204, "collideStrict union trapMask");
        }
        try {
            a.notStrict();
            throw new AssertionError("Expected TrapDetected from notStrict");
        } catch (TritException.TrapDetected e) {
            assertEquals(e.getTrapMask(), 0x0004, "notStrict trapMask");
        }

        // withTritStrict rejecting TRAP at index 7
        try {
            Trit16.zero().withTritStrict(7, Kleene.TRAP);
            throw new AssertionError("Expected TrapDetected from withTritStrict");
        } catch (TritException.TrapDetected e) {
            assertEquals(e.getTrapMask(), 1 << 7, "withTritStrict trapMask");
        }

        // Invalid index 16
        try {
            Trit16.zero().get(16);
            throw new AssertionError("Expected InvalidIndex for get(16)");
        } catch (TritException.InvalidIndex e) {
            assertEquals(e.getIndex(), 16, "InvalidIndex value");
        }

        // tryFromU32Interleaved reporting full trap mask
        int interleavedWithTraps = Trit16.fromPlanes(0x8001, 0x8001).toU32Interleaved();
        try {
            Trit16.tryFromU32Interleaved(interleavedWithTraps);
            throw new AssertionError("Expected TrapDetected from tryFromU32Interleaved");
        } catch (TritException.TrapDetected e) {
            assertEquals(e.getTrapMask(), 0x8001, "tryFromU32Interleaved full trapMask");
        }
    }

    private static void testDeterministicPrng10000Vectors() {
        long state = 0x5449_5441_4E5F_3136L; // "TITAN_16" seed
        for (int iter = 0; iter < 10_000; iter++) {
            state ^= (state << 13);
            state ^= (state >>> 7);
            state ^= (state << 17);
            int p1 = (int) (state & 0xFFFFL);
            int n1 = (int) ((state >>> 16) & 0xFFFFL);
            int p2 = (int) ((state >>> 32) & 0xFFFFL);
            int n2 = (int) ((state >>> 48) & 0xFFFFL);

            Trit16 w1 = Trit16.fromPlanes(p1, n1);
            Trit16 w2 = Trit16.fromPlanes(p2, n2);

            // Round-trip interleaved serialization
            int enc1 = w1.toU32Interleaved();
            Trit16 dec1 = Trit16.fromU32Interleaved(enc1);
            assertEqualsObj(dec1, w1, "Interleaved round-trip at iter " + iter);

            // Bit-plane invariants
            assertEquals(w1.and(w2).pos(), p1 & p2, "AND pos");
            assertEquals(w1.and(w2).neg(), n1 | n2, "AND neg");
            assertEquals(w1.or(w2).pos(), p1 | p2, "OR pos");
            assertEquals(w1.or(w2).neg(), n1 & n2, "OR neg");
            assertEquals(w1.not().pos(), n1, "NOT pos");
            assertEquals(w1.not().neg(), p1, "NOT neg");
            assertEqualsObj(w1.not().not(), w1, "Double negation");
        }
    }

    private static void testTelemetryGovernorSample() {
        Trit16 bankA = Trit16.fromPlanes(0x00FF, 0xFF00);
        Trit16 bankB = Trit16.fromPlanes(0x000F, 0xF000);
        TelemetryGovernorSample.EvaluationReport cleanReport =
            TelemetryGovernorSample.evaluateSnapshot(
                Kleene.TRUE,
                Kleene.TRUE,
                Kleene.TRUE,
                bankA,
                bankB
            );
        assertEqualsObj(cleanReport.scalarHealth, Kleene.TRUE, "Clean scalar health");
        assertTrue(cleanReport.strictValid, "Clean banks must pass strict validation");
        assertEquals(cleanReport.combinedTrapMask, 0, "Clean trap mask");
        assertEquals(cleanReport.consensusWord.pos(), 0x000F, "Consensus pos");
        assertEquals(cleanReport.consensusWord.neg(), 0xF000, "Consensus neg");

        Trit16 faultBank = bankB.withTrit(3, Kleene.TRAP);
        TelemetryGovernorSample.EvaluationReport faultReport =
            TelemetryGovernorSample.evaluateSnapshot(
                Kleene.TRUE,
                Kleene.TRAP,
                Kleene.FALSE,
                bankA,
                faultBank
            );
        assertEqualsObj(faultReport.scalarHealth, Kleene.TRAP, "Trap-dominant scalar health");
        assertTrue(!faultReport.strictValid, "Fault bank must trigger strict fallback");
        assertEquals(faultReport.combinedTrapMask, 1 << 3, "Fault trap mask at bit 3");
    }
}
