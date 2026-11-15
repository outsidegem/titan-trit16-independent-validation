package com.titan.trit16;

import androidx.test.ext.junit.runners.AndroidJUnit4;
import org.junit.Test;
import org.junit.runner.RunWith;

@RunWith(AndroidJUnit4.class)
public final class Trit16JniInstrumentedTest {

    @Test
    public void runExistingJniDifferentialSuite() {
        Trit16JniTest.main(new String[0]);
    }
}
