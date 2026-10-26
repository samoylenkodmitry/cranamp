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
    @Test public void capturePlayerAndStudio() throws Exception {
        output=new File(InstrumentationRegistry.getInstrumentation().getTargetContext().getExternalFilesDir(null),"store-gallery");
        assertTrue(output.isDirectory() || output.mkdirs());
        try (ActivityScenario<CranposeActivity> scenario=ActivityScenario.launch(CranposeActivity.class)) {
            SystemClock.sleep(5000);
            capture("01-playback");
            final Rect window=new Rect();
            scenario.onActivity(activity -> {
                android.view.View view=activity.findViewById(android.R.id.content);
                int[] location=new int[2];
                view.getLocationOnScreen(location);
                android.view.WindowInsets insets=view.getRootWindowInsets();
                window.set(location[0]+insets.getSystemWindowInsetLeft(),
                        location[1]+insets.getSystemWindowInsetTop(),
                        location[0]+view.getWidth()-insets.getSystemWindowInsetRight(),
                        location[1]+view.getHeight()-insets.getSystemWindowInsetBottom());
            });
            assertTrue("Player content bounds: "+window,window.width()>0 && window.height()>0);
            System.out.println("Player content bounds: "+window);
            float scale=Math.min(window.width()/275f,window.height()/493f);
            tap(window.left+(window.width()-275*scale)/2+262*scale,window.top+95*scale);
            capture("debug-settings");
            click("Open Skin Studio");
            click("Open working copy");
            capture("02-studio-canvas");
            click("Skin");
            capture("03-skin-colours");
            click("States");
            capture("04-states");
        }
    }
}
