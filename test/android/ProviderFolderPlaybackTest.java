package com.cranamp.app;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.media.session.MediaController;
import android.media.session.MediaSessionManager;
import android.media.session.PlaybackState;
import android.net.Uri;
import android.os.SystemClock;
import androidx.test.core.app.ActivityScenario;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import androidx.test.platform.app.InstrumentationRegistry;
import dev.cranpose.android.CranposeActivity;
import org.junit.Test;
import org.junit.runner.RunWith;
import static org.junit.Assert.*;

/** Uses the real document provider through a previously granted folder URI. */
@RunWith(AndroidJUnit4.class)
public final class ProviderFolderPlaybackTest {
    @Test public void selectedProviderFolderStartsPlayback() throws Exception {
        Instrumentation instrumentation = InstrumentationRegistry.getInstrumentation();
        Context context = instrumentation.getTargetContext();
        String folder = InstrumentationRegistry.getArguments().getString("folderUri");
        assertNotNull("Pass the URI of a folder already granted to this app", folder);
        Intent result = new Intent().setData(Uri.parse(folder))
                .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
        Instrumentation.ActivityMonitor picker = instrumentation.addMonitor(
                new IntentFilter(Intent.ACTION_OPEN_DOCUMENT_TREE),
                new Instrumentation.ActivityResult(Activity.RESULT_OK, result), true);
        instrumentation.getUiAutomation().adoptShellPermissionIdentity(
                "android.permission.MEDIA_CONTENT_CONTROL");
        Intent launch = new Intent(context, CranposeActivity.class)
                .setAction(Intent.ACTION_MAIN)
                .putExtra("open-picker", "folder")
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        long started = SystemClock.elapsedRealtime();
        try (ActivityScenario<CranposeActivity> scenario = ActivityScenario.launch(launch)) {
            MediaSessionManager sessions = context.getSystemService(MediaSessionManager.class);
            MediaController player = null;
            long firstPlaying = 0;
            long position = 0;
            while (SystemClock.elapsedRealtime() - started < 30000) {
                for (MediaController session : sessions.getActiveSessions(null)) {
                    if (!context.getPackageName().equals(session.getPackageName())) continue;
                    PlaybackState state = session.getPlaybackState();
                    if (state != null && state.getState() == PlaybackState.STATE_PLAYING) {
                        if (firstPlaying == 0) firstPlaying = SystemClock.elapsedRealtime();
                        player = session;
                        position = state.getPosition();
                    }
                }
                if (player != null && position >= 2000) break;
                SystemClock.sleep(100);
            }
            assertTrue("The folder picker must receive the provider selection", picker.getHits() > 0);
            assertNotNull("Provider audio must begin before a full folder scan", player);
            assertTrue("Decoded playback must advance beyond two seconds", position >= 2000);
            System.out.println("Provider playback started after " + (firstPlaying - started)
                    + " ms; decoded position " + position + " ms");
            player.getTransportControls().stop();
        } finally {
            instrumentation.removeMonitor(picker);
            instrumentation.getUiAutomation().dropShellPermissionIdentity();
        }
    }
}
