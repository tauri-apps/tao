// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

package app.tauri.tao_mobile;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.Service;
import android.content.Intent;
import android.content.pm.ServiceInfo;
import android.os.Build;
import android.os.IBinder;

// A foreground service that keeps the process alive after all activities are destroyed,
// e.g. when the app is removed from the recents screen.
// Declared in `app/src/{debug,release}/AndroidManifest.xml`.
public class BackgroundTaskService extends Service {
    private static final String CHANNEL_ID = "background-task";

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        Notification.Builder builder;
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            NotificationManager manager = getSystemService(NotificationManager.class);
            manager.createNotificationChannel(
                new NotificationChannel(CHANNEL_ID, "Background task", NotificationManager.IMPORTANCE_LOW));
            builder = new Notification.Builder(this, CHANNEL_ID);
        } else {
            builder = new Notification.Builder(this);
        }
        Notification notification = builder
            .setContentTitle("Tao Mobile")
            .setContentText("Background task running")
            .setSmallIcon(android.R.drawable.ic_popup_sync)
            .build();

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startForeground(1, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE);
        } else {
            startForeground(1, notification);
        }
        return START_NOT_STICKY;
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }
}
