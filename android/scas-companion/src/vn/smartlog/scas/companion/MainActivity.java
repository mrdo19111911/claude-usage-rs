package vn.smartlog.scas.companion;

import android.app.Activity;
import android.content.Intent;
import android.content.SharedPreferences;
import android.graphics.Color;
import android.net.Uri;
import android.os.Bundle;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.webkit.WebResourceError;
import android.webkit.WebResourceRequest;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.TextView;

/**
 * One screen (docs/specs/android.md): an address box, then the PC's SCAS dashboard in a
 * WebView. Holds no tokens; the only thing saved is the address.
 */
public class MainActivity extends Activity {
    private static final String PREFS = "scas";
    private static final String KEY_URL = "url";
    private static final String HINT_NOT_LOCAL =
        "SCAS only answers 127.0.0.1 / localhost. Over USB run on the PC: adb reverse tcp:PORT tcp:PORT";

    private WebView web;
    private String base;

    @Override
    protected void onCreate(Bundle saved) {
        super.onCreate(saved);
        String url = getSharedPreferences(PREFS, MODE_PRIVATE).getString(KEY_URL, null);
        if (url == null) showAddress("http://127.0.0.1:" + Address.DEFAULT_PORT, null);
        else showDashboard(url);
    }

    private void showAddress(String current, String error) {
        web = null;
        LinearLayout box = new LinearLayout(this);
        box.setOrientation(LinearLayout.VERTICAL);
        box.setGravity(Gravity.CENTER_VERTICAL);
        box.setPadding(48, 48, 48, 48);
        box.setBackgroundColor(Color.parseColor("#0d1117"));
        box.setFitsSystemWindows(true); // targetSdk 35 draws edge to edge
        TextView title = text("SCAS dashboard address", 20, "#e6edf3");
        TextView note = text(error != null ? error : "The PC running SCAS, e.g. http://127.0.0.1:4777", 14,
            error != null ? "#f85149" : "#8b949e");
        EditText field = new EditText(this);
        field.setText(current);
        field.setSingleLine(true);
        field.setTextColor(Color.parseColor("#e6edf3"));
        Button open = new Button(this);
        open.setText("Open");
        open.setOnClickListener(v -> {
            String n = Address.normalize(field.getText().toString());
            if (n == null) {
                note.setText("Not an http(s) address with a host and a port.");
                note.setTextColor(Color.parseColor("#f85149"));
                return;
            }
            getSharedPreferences(PREFS, MODE_PRIVATE).edit().putString(KEY_URL, n).apply();
            showDashboard(n);
        });
        box.addView(title);
        box.addView(note);
        box.addView(field);
        box.addView(open);
        setContentView(box);
    }

    private TextView text(String s, int sp, String color) {
        TextView t = new TextView(this);
        t.setText(s);
        t.setTextSize(sp);
        t.setTextColor(Color.parseColor(color));
        t.setPadding(0, 0, 0, 24);
        return t;
    }

    private void showDashboard(String url) {
        base = url;
        web = new WebView(this);
        web.setLayoutParams(new ViewGroup.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.MATCH_PARENT));
        web.setBackgroundColor(Color.parseColor("#0d1117"));
        web.setFitsSystemWindows(true);
        WebSettings s = web.getSettings();
        s.setJavaScriptEnabled(true);
        s.setDomStorageEnabled(true);
        s.setAllowFileAccess(false);
        web.setWebViewClient(new Client());
        setContentView(web);
        web.loadUrl(url + "/");
    }

    private final class Client extends WebViewClient {
        @Override
        public boolean shouldOverrideUrlLoading(WebView v, WebResourceRequest rq) {
            String u = rq.getUrl().toString();
            if (Address.sameOrigin(base, u)) return false;
            try {
                startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse(u)));
            } catch (Exception ignored) {
                // no app for this link: stay on the dashboard
            }
            return true;
        }

        @Override
        public void onReceivedError(WebView v, WebResourceRequest rq, WebResourceError e) {
            if (!rq.isForMainFrame()) return;
            String why = "Could not open " + base + ": " + e.getDescription();
            if (!Address.passesHostCheck(base)) why += "\n" + HINT_NOT_LOCAL;
            showAddress(base, why);
        }
    }

    @Override
    public void onBackPressed() {
        if (web != null && web.canGoBack()) web.goBack();
        else if (web != null) showAddress(base, Address.passesHostCheck(base) ? null : HINT_NOT_LOCAL);
        else super.onBackPressed();
    }
}
