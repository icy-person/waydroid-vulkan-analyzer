package com.example.waydroidvulkan;

import android.app.Activity;
import android.os.Bundle;
import android.graphics.Typeface;
import android.view.Gravity;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

public final class MainActivity extends Activity {
    static {
        System.loadLibrary("waydroid_vulkan_analyzer");
    }

    private TextView report;

    private static native String getVulkanReport();

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(24, 24, 24, 24);

        TextView title = new TextView(this);
        title.setText("Waydroid Vulkan Analyzer");
        title.setTextSize(22);
        title.setTypeface(Typeface.DEFAULT, Typeface.BOLD);
        title.setGravity(Gravity.CENTER_HORIZONTAL);
        root.addView(title, new LinearLayout.LayoutParams(-1, -2));

        Button refresh = new Button(this);
        refresh.setText("Scan Vulkan");
        root.addView(refresh, new LinearLayout.LayoutParams(-1, -2));

        report = new TextView(this);
        report.setTextSize(12);
        report.setTypeface(Typeface.MONOSPACE);
        report.setTextIsSelectable(true);
        report.setPadding(8, 16, 8, 16);

        ScrollView scroll = new ScrollView(this);
        scroll.addView(report, new ScrollView.LayoutParams(-1, -2));
        root.addView(scroll, new LinearLayout.LayoutParams(-1, 0, 1));

        setContentView(root);

        refresh.setOnClickListener(v -> scan());
        scan();
    }

    private void scan() {
        try {
            report.setText(getVulkanReport());
        } catch (Throwable t) {
            report.setText("JNI/Vulkan error:\n\n" + t);
        }
    }
}
