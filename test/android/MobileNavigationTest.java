package com.cranamp.app;

import android.app.UiAutomation;
import android.accessibilityservice.AccessibilityService;
import android.graphics.Bitmap;
import android.graphics.Rect;
import android.os.SystemClock;
import android.os.Bundle;
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

@RunWith(AndroidJUnit4.class)
public final class MobileNavigationTest {
    private final UiAutomation ui = InstrumentationRegistry.getInstrumentation().getUiAutomation();
    private AccessibilityNodeInfo find(AccessibilityNodeInfo node, String label) {
        if (node == null) return null;
        AccessibilityNodeInfo found = label.contentEquals(node.getContentDescription() == null ? "" : node.getContentDescription()) ? node : null;
        for (int i = 0; i < node.getChildCount(); i++) {
            AccessibilityNodeInfo child = find(node.getChild(i), label);
            if (child != null) found = child; // Tabs follow the player controls.
        }
        return found;
    }
    private AccessibilityNodeInfo firstAlbum(AccessibilityNodeInfo node) {
        if (node == null) return null;
        CharSequence label = node.getContentDescription();
        if (label != null && label.toString().startsWith("Open album ")) return node;
        for (int i = 0; i < node.getChildCount(); i++) {
            AccessibilityNodeInfo found = firstAlbum(node.getChild(i));
            if (found != null) return found;
        }
        return null;
    }
    private AccessibilityNodeInfo waitFor(String label) {
        long deadline = SystemClock.uptimeMillis() + 15000;
        AccessibilityNodeInfo found;
        do {
            found = find(ui.getRootInActiveWindow(), label);
            if (found != null) return found;
            SystemClock.sleep(100);
        } while (SystemClock.uptimeMillis() < deadline);
        fail("Missing accessible control: " + label);
        return null;
    }
    private void click(String label) {
        AccessibilityNodeInfo node = waitFor(label);
        assertTrue("Accessible click: " + label, node.performAction(AccessibilityNodeInfo.ACTION_CLICK));
        SystemClock.sleep(500);
    }
    private void capture(String name) throws Exception {
        SystemClock.sleep(1200);
        File folder = new File(InstrumentationRegistry.getInstrumentation().getTargetContext().getExternalFilesDir(null), "mobile-review");
        assertTrue(folder.isDirectory() || folder.mkdirs());
        Bitmap shot = ui.takeScreenshot();
        assertNotNull(shot);
        try (FileOutputStream stream = new FileOutputStream(new File(folder, name + ".png"))) {
            assertTrue(shot.compress(Bitmap.CompressFormat.PNG, 100, stream));
        } finally { shot.recycle(); }
    }
    @Test public void swipeScrollsPlaylist() throws Exception {
        org.junit.Assume.assumeTrue("Pass -e playlistSwipe 1 with a populated playlist",
                "1".equals(InstrumentationRegistry.getArguments().getString("playlistSwipe")));
        try (ActivityScenario<CranposeActivity> scenario = ActivityScenario.launch(CranposeActivity.class)) {
            click("Player");
            AccessibilityNodeInfo scrollbar = waitFor("Playlist scroll position");
            assertNotNull(scrollbar.getRangeInfo());
            float original = scrollbar.getRangeInfo().getCurrent();
            Bundle position = new Bundle();
            position.putFloat(AccessibilityNodeInfo.ACTION_ARGUMENT_PROGRESS_VALUE, 0.0f);
            assertTrue(scrollbar.performAction(AccessibilityNodeInfo.AccessibilityAction.ACTION_SET_PROGRESS.getId(), position));
            try {
                capture("playlist-before-swipe");
                AccessibilityNodeInfo playlist = waitFor("Playlist");
                Rect bounds = new Rect();
                playlist.getBoundsInScreen(bounds);
                int[] display = {0};
                scenario.onActivity(activity -> display[0] = activity.getDisplay().getDisplayId());
                String swipe = "input touchscreen -d " + display[0] + " swipe " + bounds.centerX() + " "
                        + (bounds.top + bounds.height() * 3 / 4) + " " + bounds.centerX() + " "
                        + (bounds.top + bounds.height() / 4) + " 160";
                try (android.os.ParcelFileDescriptor command = ui.executeShellCommand(swipe);
                     java.io.InputStream output = new android.os.ParcelFileDescriptor.AutoCloseInputStream(command)) {
                    StringBuilder result = new StringBuilder();
                    int value;
                    while ((value = output.read()) != -1) result.append((char) value);
                    assertFalse(result.toString(), result.toString().contains("Error"));
                }
                long deadline = SystemClock.uptimeMillis() + 5000;
                float progress;
                do {
                    SystemClock.sleep(100);
                    progress = waitFor("Playlist scroll position").getRangeInfo().getCurrent();
                } while (progress <= 0.0f && SystemClock.uptimeMillis() < deadline);
                assertTrue("A swipe must move the populated playlist", progress > 0.0f);
                capture("playlist-after-swipe");
            } finally {
                position.putFloat(AccessibilityNodeInfo.ACTION_ARGUMENT_PROGRESS_VALUE, original);
                waitFor("Playlist scroll position").performAction(
                        AccessibilityNodeInfo.AccessibilityAction.ACTION_SET_PROGRESS.getId(), position);
            }
        }
    }
    @Test public void tabsSettingsBackAndStudioDraft() throws Exception {
        try (ActivityScenario<CranposeActivity> scenario = ActivityScenario.launch(CranposeActivity.class)) {
            click("Player");
            waitFor("Play");
            for (String label : new String[]{"Player", "Library", "Skin Studio", "Settings"}) {
                AccessibilityNodeInfo tab = waitFor(label);
                Rect bounds = new Rect(); tab.getBoundsInScreen(bounds);
                System.out.println("Navigation bounds " + label + ": " + bounds);
                assertTrue(label + " visible", tab.isVisibleToUser());
            }
            capture("player");
            click("Settings");
            waitFor("Choose skin");
            capture("settings");
            click("Choose skin");
            waitFor("Back to Settings");
            capture("skins");
            assertTrue(ui.performGlobalAction(AccessibilityService.GLOBAL_ACTION_BACK));
            waitFor("Choose skin");
            click("About Cranamp");
            waitFor("Contact support");
            capture("about");
            click("Open source licenses");
            click("Cranamp — Apache License 2.0");
            waitFor("Back to Licenses");
            capture("license");
            assertTrue(ui.performGlobalAction(AccessibilityService.GLOBAL_ACTION_BACK));
            waitFor("Back to About");
            assertTrue(ui.performGlobalAction(AccessibilityService.GLOBAL_ACTION_BACK));
            waitFor("Contact support");
            assertTrue(ui.performGlobalAction(AccessibilityService.GLOBAL_ACTION_BACK));
            waitFor("Choose skin");
            assertTrue(ui.performGlobalAction(AccessibilityService.GLOBAL_ACTION_BACK));
            waitFor("Play");
            click("Skin Studio");
            click("Open working copy");
            waitFor("Canvas");
            capture("studio");
            click("Settings");
            waitFor("Choose skin");
            click("Skin Studio");
            waitFor("Canvas");
            click("Player");
            waitFor("Play");
        }
    }

    @Test public void librarySearchSheetsAndSelection() throws Exception {
        try (ActivityScenario<CranposeActivity> scenario = ActivityScenario.launch(CranposeActivity.class)) {
            click("Library");
            waitFor("Search your playlist");
            capture("library");
            click("Library Albums tab");
            capture("library-albums");
            AccessibilityNodeInfo album = firstAlbum(ui.getRootInActiveWindow());
            if (album != null) {
                assertTrue(album.performAction(AccessibilityNodeInfo.ACTION_CLICK));
                waitFor("Back to albums");
                capture("library-album-tracks");
                click("Back to albums");
            }
            click("Library Tracks tab");
            click("Add music");
            waitFor("Choose a music folder");
            capture("library-add");
            click("Cancel");
            click("Sort playlist");
            waitFor("Track title");
            capture("library-sort");
            click("Cancel");
            click("Select");
            waitFor("Select results");
            capture("library-selection");
            assertTrue(ui.performGlobalAction(AccessibilityService.GLOBAL_ACTION_BACK));
            waitFor("Sort playlist");
            click("Library options");
            waitFor("Import a playlist");
            click("Cancel");
            AccessibilityNodeInfo search = waitFor("Search your playlist");
            Bundle input = new Bundle();
            input.putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, "cranamp-library-no-match-62983");
            assertTrue(search.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, input));
            waitFor("Clear search");
            capture("library-search");
            click("Finish search");
            click("Settings");
            waitFor("Choose skin");
            capture("settings");
            click("Choose skin");
            waitFor("Find a skin");
            capture("skins");
            click("Back to Settings");
            click("Library");
            waitFor("Clear search");
            click("Clear search");
            click("Player");
            waitFor("Play");
        }
    }
}
