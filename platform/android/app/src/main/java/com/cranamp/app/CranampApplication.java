package com.cranamp.app;

import android.app.Activity;
import android.app.Application;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.View;
import android.view.WindowInsetsController;
import java.lang.ref.WeakReference;

/** The system icons sit over Cranamp's full-screen, skin-colored surface. */
public final class CranampApplication extends Application implements Application.ActivityLifecycleCallbacks {
    private static final Handler MAIN = new Handler(Looper.getMainLooper());
    private static WeakReference<Activity> active = new WeakReference<>(null);
    private static boolean darkIcons;
    private static native void nativeInitialize();

    @Override public void onCreate() {
        super.onCreate();
        System.loadLibrary("cranamp");
        nativeInitialize();
        registerActivityLifecycleCallbacks(this);
    }

    public static void setDarkSystemBarIcons(boolean dark) {
        MAIN.post(() -> {
            darkIcons = dark;
            Activity activity = active.get();
            if (activity != null) apply(activity);
        });
    }

    @SuppressWarnings("deprecation")
    private static void apply(Activity activity) {
        if (Build.VERSION.SDK_INT >= 30) {
            WindowInsetsController controller = activity.getWindow().getInsetsController();
            if (controller != null) {
                int mask = WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS
                        | WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS;
                controller.setSystemBarsAppearance(darkIcons ? mask : 0, mask);
            }
        } else {
            View decor = activity.getWindow().getDecorView();
            int mask = View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR | View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR;
            int flags = decor.getSystemUiVisibility();
            decor.setSystemUiVisibility(darkIcons ? flags | mask : flags & ~mask);
        }
    }

    @Override public void onActivityResumed(Activity activity) {
        active = new WeakReference<>(activity);
        apply(activity);
    }
    @Override public void onActivityDestroyed(Activity activity) {
        if (active.get() == activity) active.clear();
    }
    @Override public void onActivityCreated(Activity activity, Bundle state) {}
    @Override public void onActivityStarted(Activity activity) {}
    @Override public void onActivityPaused(Activity activity) {}
    @Override public void onActivityStopped(Activity activity) {}
    @Override public void onActivitySaveInstanceState(Activity activity, Bundle state) {}
}
