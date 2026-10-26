package com.cranamp.app;

import android.accessibilityservice.AccessibilityService;
import android.content.Context;
import android.view.accessibility.AccessibilityNodeInfo;
import androidx.test.core.app.ActivityScenario;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import androidx.test.platform.app.InstrumentationRegistry;
import dev.cranpose.android.CranposeActivity;
import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import org.junit.Test;
import org.junit.runner.RunWith;
import static org.junit.Assert.*;

@RunWith(AndroidJUnit4.class)
public final class StudioCancelReviewTest {
    @Test
    public void cancelledPickerThenEveryStudioControlSet() throws Exception {
        Context context=InstrumentationRegistry.getInstrumentation().getTargetContext();
        File mode=new File(context.getFilesDir(),"studio-review-mode");
        File result=new File(context.getFilesDir(),"studio-review-result");
        Files.deleteIfExists(result.toPath());
        Files.write(mode.toPath(),"cancel-and-review".getBytes(StandardCharsets.UTF_8));
        try (ActivityScenario<CranposeActivity> scenario=ActivityScenario.launch(CranposeActivity.class)) {
            long deadline=android.os.SystemClock.uptimeMillis()+15000;
            boolean picker=false;
            while(android.os.SystemClock.uptimeMillis()<deadline) {
                AccessibilityNodeInfo root=InstrumentationRegistry.getInstrumentation().getUiAutomation().getRootInActiveWindow();
                String pkg=root==null?"":String.valueOf(root.getPackageName());
                picker=pkg.contains("documentsui")||pkg.contains("myfiles");
                if(picker) break;
                android.os.SystemClock.sleep(100);
            }
            assertTrue("The system file picker must open",picker);
            assertTrue(InstrumentationRegistry.getInstrumentation().getUiAutomation()
                .performGlobalAction(AccessibilityService.GLOBAL_ACTION_BACK));
            deadline=android.os.SystemClock.uptimeMillis()+180000;
            String state="";
            while(android.os.SystemClock.uptimeMillis()<deadline) {
                if(result.isFile()) state=new String(Files.readAllBytes(result.toPath()),StandardCharsets.UTF_8);
                if(state.equals("complete")) break;
                android.os.SystemClock.sleep(100);
            }
            assertEquals("All Studio control sets must render after picker cancellation", "complete", state);
        } finally {
            Files.deleteIfExists(mode.toPath());
        }
    }
}
