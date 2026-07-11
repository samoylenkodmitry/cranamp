package com.cranamp.app;

import android.app.PendingIntent;
import android.content.ActivityNotFoundException;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.pm.PackageInstaller;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;

import org.json.JSONArray;
import org.json.JSONObject;

import dev.cranpose.android.CranposeActivity;

/**
 * Cranamp's launcher activity. Audio and skin selection go through Cranpose's
 * file picker (provided by the {@link CranposeActivity} base class via
 * JNI), so this class only adds the playlist import/export bridge that is still
 * driven through result files in the bridge directory.
 */
public class CranampActivity extends CranposeActivity {
    static {
        System.loadLibrary("cranamp");
    }

    private static final int REQ_IMPORT_PLAYLIST = 1005;
    private static final int REQ_EXPORT_PLAYLIST = 1006;

    private static final String UPDATE_API_URL =
            "https://api.github.com/repos/samoylenkodmitry/cranamp/releases/latest";
    private static final String UPDATE_INSTALL_ACTION = "com.cranamp.app.UPDATE_INSTALL_RESULT";

    private String pendingExportText = "";

    /** Receives PackageInstaller session results during an in-app update. */
    private final BroadcastReceiver updateInstallReceiver = new BroadcastReceiver() {
        @Override
        public void onReceive(Context context, Intent intent) {
            int status = intent.getIntExtra(
                    PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE);
            if (status == PackageInstaller.STATUS_PENDING_USER_ACTION) {
                // The system needs the user to confirm the install. Launch the
                // confirmation activity it handed us.
                Intent confirm = intent.getParcelableExtra(Intent.EXTRA_INTENT);
                if (confirm != null) {
                    confirm.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                    try {
                        startActivity(confirm);
                    } catch (Exception error) {
                        writeUpdateStatus("state=error\nmessage=" + safeMessage(error));
                    }
                }
            } else if (status == PackageInstaller.STATUS_SUCCESS) {
                writeUpdateStatus("state=installing");
            } else {
                String message = intent.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE);
                writeUpdateStatus(
                        "state=error\nmessage=" + (message == null ? "install failed" : message));
            }
        }
    };

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        ensureBridgeDir();
        IntentFilter filter = new IntentFilter(UPDATE_INSTALL_ACTION);
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(updateInstallReceiver, filter, Context.RECEIVER_NOT_EXPORTED);
        } else {
            registerReceiver(updateInstallReceiver, filter);
        }
    }

    @Override
    protected void onDestroy() {
        try {
            unregisterReceiver(updateInstallReceiver);
        } catch (IllegalArgumentException ignored) {
            // Receiver was never registered; nothing to clean up.
        }
        super.onDestroy();
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

    /** Queries GitHub for the latest release and compares it to the running version. */
    public void cranampCheckUpdate(String currentVersion) {
        final String current = currentVersion == null ? "" : currentVersion;
        new Thread(() -> {
            try {
                writeUpdateStatus("state=checking");
                HttpURLConnection connection =
                        (HttpURLConnection) new URL(UPDATE_API_URL).openConnection();
                connection.setRequestProperty("User-Agent", "Cranamp-Updater");
                connection.setRequestProperty("Accept", "application/vnd.github+json");
                connection.setConnectTimeout(15000);
                connection.setReadTimeout(15000);
                int code = connection.getResponseCode();
                if (code != 200) {
                    writeUpdateStatus("state=error\nmessage=GitHub returned HTTP " + code);
                    return;
                }
                JSONObject release = new JSONObject(readStream(connection.getInputStream()));
                String tag = release.optString("tag_name", "");
                String latest = tag.startsWith("v") ? tag.substring(1) : tag;
                String apkUrl = "";
                JSONArray assets = release.optJSONArray("assets");
                if (assets != null) {
                    for (int i = 0; i < assets.length(); i++) {
                        JSONObject asset = assets.getJSONObject(i);
                        if (asset.optString("name", "").endsWith(".apk")) {
                            apkUrl = asset.optString("browser_download_url", "");
                            break;
                        }
                    }
                }
                if (latest.isEmpty() || apkUrl.isEmpty()) {
                    writeUpdateStatus("state=error\nmessage=No Android asset in latest release");
                } else if (isNewerVersion(latest, current)) {
                    writeUpdateStatus("state=available\nversion=" + latest + "\nurl=" + apkUrl);
                } else {
                    writeUpdateStatus("state=uptodate");
                }
            } catch (Exception error) {
                writeUpdateStatus("state=error\nmessage=" + safeMessage(error));
            }
        }).start();
    }

    /** Downloads the APK at {@code url} and launches the in-place installer. */
    public void cranampInstallUpdate(String url) {
        final String downloadUrl = url == null ? "" : url;
        new Thread(() -> {
            try {
                writeUpdateStatus("state=downloading\npct=0");
                HttpURLConnection connection =
                        (HttpURLConnection) new URL(downloadUrl).openConnection();
                connection.setRequestProperty("User-Agent", "Cranamp-Updater");
                connection.setInstanceFollowRedirects(true);
                connection.setConnectTimeout(15000);
                connection.setReadTimeout(30000);
                int code = connection.getResponseCode();
                if (code != 200) {
                    writeUpdateStatus("state=error\nmessage=Download HTTP " + code);
                    return;
                }
                int total = connection.getContentLength();

                PackageInstaller installer = getPackageManager().getPackageInstaller();
                PackageInstaller.SessionParams params = new PackageInstaller.SessionParams(
                        PackageInstaller.SessionParams.MODE_FULL_INSTALL);
                if (total > 0) {
                    params.setSize(total);
                }
                int sessionId = installer.createSession(params);
                PackageInstaller.Session session = installer.openSession(sessionId);

                try (InputStream input = connection.getInputStream();
                        OutputStream output = session.openWrite("cranamp", 0, total)) {
                    byte[] buffer = new byte[65536];
                    long written = 0;
                    int read;
                    int lastPct = -1;
                    while ((read = input.read(buffer)) >= 0) {
                        output.write(buffer, 0, read);
                        written += read;
                        if (total > 0) {
                            int pct = (int) (written * 100 / total);
                            if (pct != lastPct) {
                                lastPct = pct;
                                writeUpdateStatus("state=downloading\npct=" + pct);
                            }
                        }
                    }
                    session.fsync(output);
                }

                writeUpdateStatus("state=installing");
                Intent intent = new Intent(UPDATE_INSTALL_ACTION).setPackage(getPackageName());
                int flags = PendingIntent.FLAG_UPDATE_CURRENT;
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                    flags |= PendingIntent.FLAG_MUTABLE;
                }
                PendingIntent pending = PendingIntent.getBroadcast(this, sessionId, intent, flags);
                session.commit(pending.getIntentSender());
                session.close();
            } catch (Exception error) {
                writeUpdateStatus("state=error\nmessage=" + safeMessage(error));
            }
        }).start();
    }

    private static boolean isNewerVersion(String latest, String current) {
        int[] remote = parseVersion(latest);
        int[] running = parseVersion(current);
        for (int i = 0; i < 3; i++) {
            if (remote[i] != running[i]) {
                return remote[i] > running[i];
            }
        }
        return false;
    }

    private static int[] parseVersion(String version) {
        int[] parts = new int[]{0, 0, 0};
        String[] split = version.trim().split("\\.");
        for (int i = 0; i < 3 && i < split.length; i++) {
            String digits = split[i].replaceAll("[^0-9].*$", "");
            if (!digits.isEmpty()) {
                try {
                    parts[i] = Integer.parseInt(digits);
                } catch (NumberFormatException ignored) {
                    // Leave the component at 0.
                }
            }
        }
        return parts;
    }

    private static String readStream(InputStream input) throws IOException {
        try (InputStream stream = input) {
            ByteArrayOutputStream buffer = new ByteArrayOutputStream();
            byte[] chunk = new byte[8192];
            int read;
            while ((read = stream.read(chunk)) >= 0) {
                buffer.write(chunk, 0, read);
            }
            return buffer.toString("UTF-8");
        }
    }

    private static String safeMessage(Throwable error) {
        String message = error.getMessage();
        return (message == null ? error.getClass().getSimpleName() : message).replace('\n', ' ');
    }

    private void writeUpdateStatus(String content) {
        writeAtomic("update_status", content);
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
