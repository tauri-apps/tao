// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

package app.tauri.tao_mobile;

import android.app.Activity;
import android.content.Intent;
import android.os.Build;
import android.os.Bundle;
import android.util.Log;
import android.view.Gravity;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.TextView;

// The minimal activity tao needs: it forwards the lifecycle to Rust
// and implements the methods tao calls through JNI.
public class MainActivity extends Activity {
    private static final String ACTIVITY_ID_KEY = "__taoActivityId";
    // set by the e2e tests to start the background task on launch
    private static final String START_BACKGROUND_TASK_KEY = "startBackgroundTask";
    private static boolean firstActivityCreated = false;

    private int id;
    private TextView status;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        if (savedInstanceState != null && savedInstanceState.containsKey(ACTIVITY_ID_KEY)) {
            // recreated due to a configuration change: keep the ID so tao keeps it attached to its window
            id = savedInstanceState.getInt(ACTIVITY_ID_KEY);
        } else if (getIntent().hasExtra(ACTIVITY_ID_KEY)) {
            // created by tao (`Window::new` with an activity name)
            id = getIntent().getIntExtra(ACTIVITY_ID_KEY, 0);
        } else {
            // created by the system
            id = new java.util.Random().nextInt();
        }

        LinearLayout layout = new LinearLayout(this);
        layout.setOrientation(LinearLayout.VERTICAL);
        layout.setGravity(Gravity.CENTER);
        int padding = (int) (24 * getResources().getDisplayMetrics().density);
        layout.setPadding(padding, padding, padding, padding);

        TextView title = new TextView(this);
        title.setText("Activity " + id);
        title.setTextSize(20);
        title.setGravity(Gravity.CENTER);
        layout.addView(title);

        status = new TextView(this);
        status.setText("no tao window attached");
        status.setGravity(Gravity.CENTER);
        layout.addView(status);

        Button newWindow = new Button(this);
        newWindow.setText("New window");
        newWindow.setOnClickListener(v -> Rust.requestWindow());
        layout.addView(newWindow);

        Button backgroundTask = new Button(this);
        backgroundTask.setText("Start background task");
        backgroundTask.setOnClickListener(v -> startBackgroundTask());
        layout.addView(backgroundTask);

        Button finish = new Button(this);
        finish.setText("Finish activity");
        finish.setOnClickListener(v -> finishAndRemoveTask());
        layout.addView(finish);

        setContentView(layout);

        if (getIntent().getBooleanExtra(START_BACKGROUND_TASK_KEY, false)) {
            startBackgroundTask();
        }

        Rust.onCreate(this);
        // the Rust entry point runs once per process
        if (!firstActivityCreated) {
            firstActivityCreated = true;
            Rust.onFirstActivityCreate();
        }
    }

    @Override
    protected void onStart() {
        super.onStart();
        Rust.onStart(this);
    }

    @Override
    protected void onResume() {
        super.onResume();
        Rust.onResume(this);
    }

    @Override
    protected void onPause() {
        super.onPause();
        Rust.onPause(this);
    }

    @Override
    protected void onStop() {
        super.onStop();
        Rust.onStop(this);
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
        Rust.onDestroy(this);
    }

    @Override
    public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);
        Rust.onWindowFocusChanged(this, hasFocus);
    }

    @Override
    public void onLowMemory() {
        super.onLowMemory();
        Rust.onLowMemory();
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        Rust.onNewIntent(intent);
    }

    @Override
    protected void onSaveInstanceState(Bundle outState) {
        super.onSaveInstanceState(outState);
        outState.putInt(ACTIVITY_ID_KEY, id);
        // e2e/android.sh waits for this before killing the process
        Log.i("tao-mobile", "[e2e] state-saved activity=" + id);
    }

    // keeps the process alive when the app is removed from the recents screen
    private void startBackgroundTask() {
        Intent intent = new Intent(this, BackgroundTaskService.class);
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            startForegroundService(intent);
        } else {
            startService(intent);
        }
    }

    // Called by tao through JNI

    public int getId() {
        return id;
    }

    public Class<?> getAppClass(String name) throws ClassNotFoundException {
        return Class.forName(name);
    }

    public int startActivity(Class<?> cls) {
        Intent intent = new Intent(this, cls);
        int activityId = new java.util.Random().nextInt();
        intent.putExtra(ACTIVITY_ID_KEY, activityId);
        startActivity(intent);
        return activityId;
    }

    // Called by the example through JNI

    public void setStatus(String text) {
        runOnUiThread(() -> status.setText(text));
    }
}
