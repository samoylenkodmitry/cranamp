package com.cranamp.app;

import android.app.UiAutomation;
import android.graphics.Bitmap;
import android.graphics.Rect;
import android.os.SystemClock;
import android.view.InputDevice;
import android.view.MotionEvent;
import android.view.accessibility.AccessibilityNodeInfo;
import androidx.test.core.app.ActivityScenario;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import androidx.test.platform.app.InstrumentationRegistry;
import dev.cranpose.android.CranposeActivity;
import java.io.File;
import java.io.FileOutputStream;
import org.junit.Test;
import org.junit.runner.RunWith;
import static org.junit.Assert.*;

/** Captures actual Android screens through the public app controls. */
@RunWith(AndroidJUnit4.class)
public final class StoreGalleryTest {
    private final UiAutomation ui = InstrumentationRegistry.getInstrumentation().getUiAutomation();
    private File output;
    private AccessibilityNodeInfo find(AccessibilityNodeInfo node, String name) {
        if (node == null) return null;
        if (name.contentEquals(node.getText() == null ? "" : node.getText()) ||
            name.contentEquals(node.getContentDescription() == null ? "" : node.getContentDescription())) return node;
        for (int i = 0; i < node.getChildCount(); i++) {
            AccessibilityNodeInfo found = find(node.getChild(i), name);
            if (found != null) return found;
        }
        return null;
    }
    private void tap(float x, float y) {
        long now=SystemClock.uptimeMillis();
        for (int action : new int[]{MotionEvent.ACTION_DOWN, MotionEvent.ACTION_UP}) {
            MotionEvent event=MotionEvent.obtain(now,SystemClock.uptimeMillis(),action,x,y,0);
            event.setSource(InputDevice.SOURCE_TOUCHSCREEN);
            assertTrue(ui.injectInputEvent(event,true));
            event.recycle();
        }
    }
    private void click(String name) {
        long deadline=SystemClock.uptimeMillis()+15000;
        AccessibilityNodeInfo found=null;
        while (found==null && SystemClock.uptimeMillis()<deadline) {
            found=find(ui.getRootInActiveWindow(),name);
            if (found==null) SystemClock.sleep(100);
        }
        assertNotNull("Visible control: "+name,found);
        Rect bounds=new Rect(); found.getBoundsInScreen(bounds);
        tap(bounds.exactCenterX(),bounds.exactCenterY());
        SystemClock.sleep(1500);
    }
    private void capture(String name) throws Exception {
        SystemClock.sleep(2000);
        Bitmap bitmap=ui.takeScreenshot(); assertNotNull(bitmap);
        try (FileOutputStream stream=new FileOutputStream(new File(output,name+".png"))) {
            assertTrue(bitmap.compress(Bitmap.CompressFormat.PNG,100,stream));
        } finally {bitmap.recycle();}
    }
    private boolean visible(String name) {
        return find(ui.getRootInActiveWindow(), name) != null;
    }
    @Test public void capturePlayerAndStudio() throws Exception {
        output=new File(InstrumentationRegistry.getInstrumentation().getTargetContext().getExternalFilesDir(null),"store-gallery");
        assertTrue(output.isDirectory() || output.mkdirs());
        try (ActivityScenario<CranposeActivity> scenario=ActivityScenario.launch(CranposeActivity.class)) {
            SystemClock.sleep(5000);
            // A wide window starts with the whole player and its navigation
            // hidden; the logo shows it.
            if (!visible("Library")) {
                capture("00-wide-player");
                click("Toggle navigation");
            }
            click("Player");
            capture("01-player");
            click("Skin Studio");
            if (visible("Open working copy")) click("Open working copy");
            capture("02-studio-canvas");
            click("Settings");
            capture("04-settings");
            click("Choose skin");
            capture("03-skins");
            click("Back to Settings");
            click("Library");
            capture("05-library");
            click("Player");
        }
    }
}
