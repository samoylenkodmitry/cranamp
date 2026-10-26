package com.cranamp.app;

import android.content.Context;
import android.content.UriPermission;
import android.graphics.Rect;
import android.os.SystemClock;
import android.provider.OpenableColumns;
import android.view.InputDevice;
import android.view.MotionEvent;
import android.view.accessibility.AccessibilityNodeInfo;
import androidx.test.core.app.ActivityScenario;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import androidx.test.platform.app.InstrumentationRegistry;
import dev.cranpose.android.CranposeActivity;
import java.io.InputStream;
import java.util.List;
import org.junit.Test;
import org.junit.runner.RunWith;
import static org.junit.Assert.*;

/** Uses a generated WAV in Downloads and the real system document picker. */
@RunWith(AndroidJUnit4.class)
public final class FileAccessPersistenceTest {
    @Test public void pickingAudioRetainsReadAccess() throws Exception {
        Context context = InstrumentationRegistry.getInstrumentation().getTargetContext();
        for (UriPermission grant : context.getContentResolver().getPersistedUriPermissions()) {
            if (isFixture(context, grant)) context.getContentResolver().releasePersistableUriPermission(
                    grant.getUri(), android.content.Intent.FLAG_GRANT_READ_URI_PERMISSION);
        }
        try (ActivityScenario<CranposeActivity> scenario = ActivityScenario.launch(CranposeActivity.class)) {
            SystemClock.sleep(3000);
            final float[] eject = new float[2];
            scenario.onActivity(activity -> {
                android.view.View view = activity.findViewById(android.R.id.content);
                int[] location = new int[2];
                view.getLocationOnScreen(location);
                android.view.WindowInsets insets = view.getRootWindowInsets();
                float width = view.getWidth() - insets.getSystemWindowInsetLeft() - insets.getSystemWindowInsetRight();
                float height = view.getHeight() - insets.getSystemWindowInsetTop() - insets.getSystemWindowInsetBottom();
                float scale = Math.min(width / 275f, height / 493f);
                eject[0] = location[0] + insets.getSystemWindowInsetLeft() + (width - 275 * scale) / 2 + 147 * scale;
                eject[1] = location[1] + insets.getSystemWindowInsetTop() + 96 * scale;
            });
            tap(eject[0], eject[1]);
            clickPickerItem("Show roots");
            clickPickerItem("Downloads");
            clickPickerItem("Cranamp playback");
            long deadline = SystemClock.uptimeMillis() + 15000;
            while (context.getContentResolver().getPersistedUriPermissions().isEmpty()
                    && SystemClock.uptimeMillis() < deadline) SystemClock.sleep(100);
            assertReadableGrant(context);
        }
    }

    @Test public void selectedFileIsReadableAfterProcessRestart() throws Exception {
        assertReadableGrant(InstrumentationRegistry.getInstrumentation().getTargetContext());
    }

    private static void clickPickerItem(String text) throws Exception {
        var ui = InstrumentationRegistry.getInstrumentation().getUiAutomation();
        long deadline = SystemClock.uptimeMillis() + 20000;
        AccessibilityNodeInfo item = null;
        while (item == null && SystemClock.uptimeMillis() < deadline) {
            AccessibilityNodeInfo root = ui.getRootInActiveWindow();
            if (root != null && String.valueOf(root.getPackageName()).contains("documentsui")) {
                List<AccessibilityNodeInfo> found = root.findAccessibilityNodeInfosByText(text);
                for (AccessibilityNodeInfo candidate : found) {
                    String description = String.valueOf(candidate.getContentDescription());
                    if (!description.startsWith("Preview the file")) item = candidate;
                }
            }
            if (item == null) SystemClock.sleep(100);
        }
        assertNotNull("Document picker item: " + text, item);
        Rect bounds = new Rect();
        item.getBoundsInScreen(bounds);
        assertFalse("Visible picker item: " + text, bounds.isEmpty());
        AccessibilityNodeInfo clickable = item;
        while (clickable != null && !clickable.isClickable()) clickable = clickable.getParent();
        if (clickable != null) {
            assertTrue("Select picker item: " + text,
                    clickable.performAction(AccessibilityNodeInfo.ACTION_CLICK));
        } else {
            tap(bounds.exactCenterX(), bounds.exactCenterY());
        }
        ui.waitForIdle(500, 5000);
    }

    private static void tap(float x, float y) {
        var ui = InstrumentationRegistry.getInstrumentation().getUiAutomation();
        long down = SystemClock.uptimeMillis();
        for (int action : new int[]{MotionEvent.ACTION_DOWN, MotionEvent.ACTION_UP}) {
            MotionEvent event = MotionEvent.obtain(down, SystemClock.uptimeMillis(), action, x, y, 0);
            event.setSource(InputDevice.SOURCE_TOUCHSCREEN);
            assertTrue(ui.injectInputEvent(event, true));
            event.recycle();
            if (action == MotionEvent.ACTION_DOWN) SystemClock.sleep(100);
        }
    }

    private static void assertReadableGrant(Context context) throws Exception {
        boolean readable = false;
        for (UriPermission grant : context.getContentResolver().getPersistedUriPermissions()) {
            if (!grant.isReadPermission() || !isFixture(context, grant)) continue;
            try (InputStream stream = context.getContentResolver().openInputStream(grant.getUri())) {
                assertNotNull(stream);
                assertTrue("The selected document must remain readable", stream.read() >= 0);
                readable = true;
            }
        }
        assertTrue("File selection must retain a read grant across process restarts", readable);
    }

    private static boolean isFixture(Context context, UriPermission grant) {
        if (!grant.getUri().toString().contains("/document/")) return false;
        try (var cursor = context.getContentResolver().query(grant.getUri(),
                new String[]{OpenableColumns.DISPLAY_NAME}, null, null, null)) {
            return cursor != null && cursor.moveToFirst()
                    && "Cranamp playback check.wav".equals(cursor.getString(0));
        }
    }
}
