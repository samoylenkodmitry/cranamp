package com.cranamp.app;

import android.app.NotificationManager;
import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.Color;
import android.media.MediaMetadata;
import android.media.session.MediaController;
import android.media.session.MediaSession;
import android.os.SystemClock;
import androidx.test.core.app.ActivityScenario;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import androidx.test.platform.app.InstrumentationRegistry;
import dev.cranpose.android.CranposeActivity;
import java.io.ByteArrayOutputStream;
import org.junit.Test;
import org.junit.runner.RunWith;
import static org.junit.Assert.*;

@RunWith(AndroidJUnit4.class)
public final class MediaMetadataTest {
    @Test public void albumCoverReachesSessionAndNotificationAndClearsForNextTrack() throws Exception {
        var instrumentation = InstrumentationRegistry.getInstrumentation();
        Context context = instrumentation.getTargetContext();
        try (ActivityScenario<CranposeActivity> scenario = ActivityScenario.launch(CranposeActivity.class)) {
            Bitmap cover = Bitmap.createBitmap(64, 64, Bitmap.Config.ARGB_8888);
            cover.eraseColor(Color.MAGENTA);
            ByteArrayOutputStream bytes = new ByteArrayOutputStream();
            cover.compress(Bitmap.CompressFormat.PNG, 100, bytes);
            scenario.onActivity(activity -> {
                activity.cranposeMediaSetMetadata("Song", "Artist", "Album", bytes.toByteArray());
                activity.cranposeMediaSessionUpdate(1, 1200, 180000, 1.0f);
            });
            NotificationManager notifications = context.getSystemService(NotificationManager.class);
            MediaController controller = null;
            long deadline = SystemClock.uptimeMillis() + 5000;
            while (controller == null && SystemClock.uptimeMillis() < deadline) {
                for (var posted : notifications.getActiveNotifications()) {
                    MediaSession.Token token = posted.getNotification().extras.getParcelable(android.app.Notification.EXTRA_MEDIA_SESSION);
                    if (token != null) controller = new MediaController(context, token);
                }
                if (controller == null) SystemClock.sleep(50);
            }
            assertNotNull("Cranamp media session", controller);
            MediaMetadata metadata = controller.getMetadata();
            assertEquals("Album", metadata.getString(MediaMetadata.METADATA_KEY_ALBUM));
            Bitmap published = metadata.getBitmap(MediaMetadata.METADATA_KEY_ALBUM_ART);
            assertNotNull("Album cover", published);
            assertEquals(Color.MAGENTA, published.getPixel(32, 32));
            boolean artworkShown = false;
            while (!artworkShown && SystemClock.uptimeMillis() < deadline) {
                for (var notification : notifications.getActiveNotifications()) {
                    artworkShown |= notification.getNotification().getLargeIcon() != null;
                }
                if (!artworkShown) SystemClock.sleep(50);
            }
            assertTrue("Notification has the album artwork", artworkShown);
            scenario.onActivity(activity -> activity.cranposeMediaSetMetadata("Other song", "Other artist", "Other album", new byte[0]));
            assertEquals("Other album", controller.getMetadata().getString(MediaMetadata.METADATA_KEY_ALBUM));
            assertNull("No stale album cover", controller.getMetadata().getBitmap(MediaMetadata.METADATA_KEY_ALBUM_ART));
            scenario.onActivity(activity -> activity.cranposeMediaSessionUpdate(0, 0, -1, 1.0f));
        }
    }
}
