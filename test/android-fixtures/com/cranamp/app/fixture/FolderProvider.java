package com.cranamp.app.fixture;

import android.database.Cursor;
import android.database.MatrixCursor;
import android.os.Bundle;
import android.os.CancellationSignal;
import android.os.ParcelFileDescriptor;
import android.provider.DocumentsContract;
import android.provider.DocumentsProvider;
import java.io.FileNotFoundException;
import java.util.concurrent.atomic.AtomicInteger;

/** Supplies directory listing failures without reading or creating audio. */
public final class FolderProvider extends DocumentsProvider {
    public static volatile String mode = "empty";
    public static final AtomicInteger queries = new AtomicInteger();
    @Override public boolean onCreate() { return true; }
    @Override public Cursor queryRoots(String[] projection) { return new MatrixCursor(new String[0]); }
    @Override public Cursor queryDocument(String id, String[] projection) { return new MatrixCursor(new String[0]); }
    @Override public Cursor queryChildDocuments(String id, String[] projection, String sortOrder) {
        int attempt = queries.incrementAndGet();
        if (mode.equals("null") || (mode.equals("transient") && attempt < 4)) return null;
        MatrixCursor cursor = new MatrixCursor(projection);
        if (mode.equals("loading")) {
            Bundle extras = new Bundle();
            extras.putBoolean(DocumentsContract.EXTRA_LOADING, true);
            cursor.setExtras(extras);
        }
        return cursor;
    }
    @Override public ParcelFileDescriptor openDocument(String id, String mode, CancellationSignal signal)
            throws FileNotFoundException { throw new FileNotFoundException("No audio in this fixture"); }
}
