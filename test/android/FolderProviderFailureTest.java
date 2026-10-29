package com.cranamp.app;

import android.content.Context;
import android.content.Intent;
import android.net.Uri;
import android.provider.DocumentsContract;
import androidx.test.core.app.ActivityScenario;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import androidx.test.platform.app.InstrumentationRegistry;
import com.cranamp.app.fixture.FolderProvider;
import dev.cranpose.android.CranposeActivity;
import java.util.concurrent.atomic.AtomicReference;
import org.junit.Test;
import org.junit.runner.RunWith;
import static org.junit.Assert.*;

@RunWith(AndroidJUnit4.class)
public final class FolderProviderFailureTest {
    @Test public void providerFailuresAreNotReportedAsAnEmptyFolder() {
        Context context = InstrumentationRegistry.getInstrumentation().getTargetContext();
        Intent launch = new Intent(context, CranposeActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        try (ActivityScenario<CranposeActivity> scenario = ActivityScenario.launch(launch)) {
            AtomicReference<CranposeActivity> activity = new AtomicReference<>();
            scenario.onActivity(activity::set);
            Uri tree = DocumentsContract.buildTreeDocumentUri(context.getPackageName() + ".folder-test", "root");
            FolderProvider.mode = "transient";
            FolderProvider.queries.set(0);
            assertEquals("A transient null cursor must be retried", "", activity.get().cranposeFolderChildren(tree.toString(), "root"));
            FolderProvider.mode = "null";
            assertNull("A failed listing must not look empty", activity.get().cranposeFolderChildren(tree.toString(), "root"));
            FolderProvider.mode = "loading";
            assertNull("An unfinished listing must not look empty", activity.get().cranposeFolderChildren(tree.toString(), "root"));
        } finally { FolderProvider.mode = "empty"; }
    }
}
