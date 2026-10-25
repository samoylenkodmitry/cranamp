package com.cranamp.app;

import android.view.WindowManager;
import androidx.test.core.app.ActivityScenario;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import dev.cranpose.android.CranposeActivity;
import dev.cranpose.android.CranposeOverlayWindow;
import org.junit.Test;
import org.junit.runner.RunWith;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

@RunWith(AndroidJUnit4.class)
public final class WindowLayoutTest {
    @Test
    public void playerRegistersItsWindowSizeDuringStartup() {
        try (ActivityScenario<CranposeActivity> scenario = ActivityScenario.launch(CranposeActivity.class)) {
            final boolean[] sized = {false};
            for (int attempt = 0; attempt < 50 && !sized[0]; attempt++) {
                scenario.onActivity(activity -> {
                    WindowManager.LayoutParams layout = activity.getWindow().getAttributes();
                    sized[0] = layout.width > 0 && layout.height > 0;
                });
                if (!sized[0]) android.os.SystemClock.sleep(100);
            }
            assertTrue("Player must request its skin dimensions during startup", sized[0]);
        }
    }

    @Test
    public void windowSizeReturnsToSystemControlAfterExplicitRequests() {
        try (ActivityScenario<CranposeActivity> scenario = ActivityScenario.launch(CranposeActivity.class)) {
            scenario.onActivity(activity -> {
                // Start from the system contract used by Studio.
                CranposeOverlayWindow.restoreActivityWindowLayout(activity);
                activity.getWindow().setLayout(WindowManager.LayoutParams.MATCH_PARENT,
                        WindowManager.LayoutParams.MATCH_PARENT);
                for (int pass = 0; pass < 3; pass++) {
                    CranposeOverlayWindow.setActivityWindowLayout(activity, 440, 660);
                    CranposeOverlayWindow.setActivityWindowLayout(activity, 550, 825);
                    assertEquals(550, activity.getWindow().getAttributes().width);
                    assertEquals(825, activity.getWindow().getAttributes().height);
                    CranposeOverlayWindow.restoreActivityWindowLayout(activity);
                    assertEquals(WindowManager.LayoutParams.MATCH_PARENT, activity.getWindow().getAttributes().width);
                    assertEquals(WindowManager.LayoutParams.MATCH_PARENT, activity.getWindow().getAttributes().height);
                    CranposeOverlayWindow.restoreActivityWindowLayout(activity);
                    assertEquals(WindowManager.LayoutParams.MATCH_PARENT, activity.getWindow().getAttributes().width);
                }
            });
        }
    }
}
