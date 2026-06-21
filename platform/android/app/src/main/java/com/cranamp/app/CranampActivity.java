package com.cranamp.app;

import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;

import dev.cranpose.android.CranposeFilePickerActivity;

/**
 * Cranamp's launcher activity. Audio and skin selection go through Cranpose's
 * file picker (provided by the {@link CranposeFilePickerActivity} base class via
 * JNI), so this class only adds the playlist import/export bridge that is still
 * driven through result files in the bridge directory.
 */
public class CranampActivity extends CranposeFilePickerActivity {
    static {
        System.loadLibrary("cranamp");
    }

    private static final int REQ_IMPORT_PLAYLIST = 1005;
    private static final int REQ_EXPORT_PLAYLIST = 1006;

    private String pendingExportText = "";

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        ensureBridgeDir();
    }

    public String cranampBridgeDirectory() {
        return ensureBridgeDir().getAbsolutePath();
    }

    public void cranampImportPlaylist() {
        runOnUiThread(() -> {
            clearResult("playlist_import");
            Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
            intent.addCategory(Intent.CATEGORY_OPENABLE);
            intent.setType("*/*");
            intent.putExtra(Intent.EXTRA_MIME_TYPES, new String[]{
                    "audio/x-mpegurl",
                    "application/vnd.apple.mpegurl",
                    "application/x-mpegurl",
                    "text/plain"
            });
            intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
            launchIntent(intent, REQ_IMPORT_PLAYLIST, "playlist_import");
        });
    }

    public void cranampExportPlaylist(String playlistText) {
        runOnUiThread(() -> {
            pendingExportText = playlistText == null ? "" : playlistText;
            clearResult("playlist_export");
            Intent intent = new Intent(Intent.ACTION_CREATE_DOCUMENT);
            intent.addCategory(Intent.CATEGORY_OPENABLE);
            intent.setType("audio/x-mpegurl");
            intent.putExtra(Intent.EXTRA_TITLE, "playlist.m3u");
            intent.addFlags(Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
            launchIntent(intent, REQ_EXPORT_PLAYLIST, "playlist_export");
        });
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        // Let the picker base class handle its own request codes first.
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode != REQ_IMPORT_PLAYLIST && requestCode != REQ_EXPORT_PLAYLIST) {
            return;
        }
        if (resultCode != RESULT_OK || data == null) {
            writeCancel(resultNameForRequest(requestCode));
            return;
        }
        try {
            if (requestCode == REQ_IMPORT_PLAYLIST) {
                writePlaylistImport(data.getData());
            } else {
                writePlaylistExport(data.getData());
            }
        } catch (Exception error) {
            writeError(resultNameForRequest(requestCode), error.toString());
        }
    }

    private void launchIntent(Intent intent, int requestCode, String resultName) {
        try {
            startActivityForResult(intent, requestCode);
        } catch (ActivityNotFoundException error) {
            writeError(resultName, "No Android document picker is available");
        }
    }

    private void writePlaylistImport(Uri uri) throws IOException {
        if (uri == null) {
            writeCancel("playlist_import");
            return;
        }
        writeAtomic("playlist_import.m3u", readUriText(uri));
    }

    private void writePlaylistExport(Uri uri) throws IOException {
        if (uri == null) {
            writeCancel("playlist_export");
            return;
        }
        try (OutputStream output = getContentResolver().openOutputStream(uri, "wt")) {
            if (output == null) {
                throw new IOException("Android returned no output stream");
            }
            output.write(pendingExportText.getBytes(StandardCharsets.UTF_8));
        }
        writeAtomic("playlist_export.ok", uri.toString());
    }

    private String readUriText(Uri uri) throws IOException {
        try (InputStream input = getContentResolver().openInputStream(uri)) {
            if (input == null) {
                throw new IOException("Android returned no input stream");
            }
            byte[] buffer = new byte[8192];
            StringBuilder text = new StringBuilder();
            int read;
            while ((read = input.read(buffer)) >= 0) {
                text.append(new String(buffer, 0, read, StandardCharsets.UTF_8));
            }
            return text.toString();
        }
    }

    private File ensureBridgeDir() {
        File baseDir = getFilesDir();
        if (baseDir == null) {
            throw new IllegalStateException("app files directory is unavailable");
        }
        File dir = new File(baseDir, "cranamp_bridge");
        if (dir.isDirectory()) {
            return dir;
        }
        if (dir.exists()) {
            throw new IllegalStateException("expected directory but found file: " + dir);
        }
        if (!dir.mkdirs() && !dir.isDirectory()) {
            throw new IllegalStateException("failed to create " + dir);
        }
        return dir;
    }

    private void clearResult(String name) {
        deleteResultFile(name + ".m3u");
        deleteResultFile(name + ".ok");
        deleteResultFile(name + ".cancel");
        deleteResultFile(name + ".error");
    }

    private void writeCancel(String name) {
        if (name.isEmpty()) {
            return;
        }
        writeAtomic(name + ".cancel", "");
    }

    private void writeError(String name, String error) {
        if (name.isEmpty()) {
            return;
        }
        writeAtomic(name + ".error", error == null ? "Android picker failed" : error);
    }

    private void writeAtomic(String fileName, String text) {
        File dir = ensureBridgeDir();
        File tmp = new File(dir, fileName + ".tmp");
        File out = new File(dir, fileName);
        try (FileOutputStream stream = new FileOutputStream(tmp)) {
            stream.write(text.getBytes(StandardCharsets.UTF_8));
        } catch (IOException error) {
            return;
        }
        if (!tmp.renameTo(out)) {
            deleteResultFile(fileName);
            tmp.renameTo(out);
        }
    }

    private void deleteResultFile(String fileName) {
        File file = new File(ensureBridgeDir(), fileName);
        if (file.isFile()) {
            file.delete();
        }
    }

    private String resultNameForRequest(int requestCode) {
        switch (requestCode) {
            case REQ_IMPORT_PLAYLIST:
                return "playlist_import";
            case REQ_EXPORT_PLAYLIST:
                return "playlist_export";
            default:
                return "";
        }
    }
}
