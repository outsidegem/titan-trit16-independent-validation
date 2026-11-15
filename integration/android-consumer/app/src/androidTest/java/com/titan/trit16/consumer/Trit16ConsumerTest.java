package com.titan.trit16.consumer;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import androidx.test.ext.junit.runners.AndroidJUnit4;

import com.titan.trit16.Kleene;
import com.titan.trit16.Trit16;
import com.titan.trit16.Trit16Native;

import org.junit.BeforeClass;
import org.junit.Test;
import org.junit.runner.RunWith;

@RunWith(AndroidJUnit4.class)
public final class Trit16ConsumerTest {

    @BeforeClass
    public static void loadNativeLibrary() {
        Trit16Native.ensureLoaded();
    }

    @Test
    public void aarLoadsAndReportsExpectedAbi() {
        assertEquals(0x00010000, Trit16Native.abiVersion());
    }

    @Test
    public void publicApiConstructsAndReadsTrits() {
        Trit16 truth = Trit16.fromPlanes(0x0001, 0x0000);
        Trit16 falsity = Trit16.fromPlanes(0x0000, 0x0001);

        assertEquals(Kleene.TRUE, truth.get(0));
        assertEquals(Kleene.FALSE, falsity.get(0));
        assertTrue(truth.isValid());
        assertFalse(truth.hasTraps());
    }

    @Test
    public void operationsAndSerializationWorkThroughAar() {
        Trit16 truth = Trit16.fromPlanes(0x0001, 0x0000);

        assertEquals(Kleene.TRUE, truth.and(truth).get(0));

        Trit16 restored =
            Trit16.fromU32Interleaved(truth.toU32Interleaved());

        assertEquals(truth, restored);
    }

    @Test
    public void trapStateRemainsObservable() {
        Trit16 trap = Trit16.fromPlanes(0x0004, 0x0004);

        assertTrue(trap.hasTraps());
        assertEquals(0x0004, trap.trapMask());
        assertEquals(Kleene.TRAP, trap.get(2));
    }
}
