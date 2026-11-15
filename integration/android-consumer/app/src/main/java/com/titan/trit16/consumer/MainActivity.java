package com.titan.trit16.consumer;

import android.app.Activity;
import android.os.Bundle;
import android.widget.TextView;

import com.titan.trit16.Kleene;
import com.titan.trit16.Trit16;
import com.titan.trit16.Trit16Native;

public final class MainActivity extends Activity {
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        TextView status = new TextView(this);
        status.setPadding(32, 48, 32, 32);

        try {
            Trit16Native.ensureLoaded();

            Trit16 word = Trit16.fromPlanes(0x0001, 0x0000);

            if (word.get(0) != Kleene.TRUE) {
                throw new IllegalStateException(
                    "Unexpected trit value"
                );
            }

            status.setText(
                "TRIT16 external AAR consumer: PASS\n"
                + "ABI: " + Trit16Native.abiVersion() + "\n"
                + "Word: " + word
            );

        } catch (Throwable error) {
            status.setText(
                "TRIT16 external AAR consumer: FAIL\n" + error
            );
        }

        setContentView(status);
    }
}
