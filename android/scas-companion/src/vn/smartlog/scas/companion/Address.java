package vn.smartlog.scas.companion;

import java.util.Locale;

/**
 * The address of the SCAS dashboard the app opens (docs/specs/android.md "The app").
 * Pure Java, no Android classes, so test/AddressTest.java runs on a plain JDK.
 */
public final class Address {
    /** SCAS's fixed port outside Windows (main.rs). */
    public static final int DEFAULT_PORT = 4777;

    private Address() {}

    /**
     * What the user typed, as "scheme://host:port", or null when it cannot be a SCAS
     * address. "52001" or ":52001" means 127.0.0.1 on that port; no scheme means http;
     * no port means 4777; the path is dropped.
     */
    public static String normalize(String raw) {
        if (raw == null) return null;
        String s = raw.trim();
        if (s.isEmpty()) return null;
        if (s.matches(":?\\d+")) s = "127.0.0.1:" + s.replace(":", "");
        String[] p = parse(s, false);
        return p == null ? null : p[0] + "://" + p[1] + ":" + p[2];
    }

    /** True when SCAS's Host check (server.rs) lets this address in: 127.0.0.1 or localhost. */
    public static boolean passesHostCheck(String url) {
        String n = normalize(url);
        if (n == null) return false;
        String host = parse(n, true)[1];
        return host.equals("127.0.0.1") || host.equals("localhost");
    }

    /** True when `url` is on the dashboard's own origin; other links open in the browser. */
    public static boolean sameOrigin(String base, String url) {
        String[] a = parse(base == null ? "" : base.trim(), true);
        String[] b = parse(url == null ? "" : url.trim(), true);
        return a != null && b != null && a[0].equals(b[0]) && a[1].equals(b[1]) && a[2].equals(b[2]);
    }

    // {scheme, host, port} or null. `strict`: the scheme must be written, and a missing
    // port is the scheme's own (80/443); otherwise http and 4777 are assumed.
    private static String[] parse(String s, boolean strict) {
        String scheme = "http";
        String rest = s;
        int sep = s.indexOf("://");
        if (sep >= 0) {
            scheme = s.substring(0, sep).toLowerCase(Locale.ROOT);
            rest = s.substring(sep + 3);
        } else if (strict) {
            return null;
        }
        if (!scheme.equals("http") && !scheme.equals("https")) return null;
        int end = rest.length();
        for (char c : new char[] {'/', '?', '#'}) {
            int i = rest.indexOf(c);
            if (i >= 0 && i < end) end = i;
        }
        String auth = rest.substring(0, end);
        if (auth.contains("@")) return null;
        String host = auth;
        String port = null;
        int colon = auth.lastIndexOf(':');
        if (colon >= 0) {
            host = auth.substring(0, colon);
            port = auth.substring(colon + 1);
        }
        host = host.toLowerCase(Locale.ROOT);
        if (host.isEmpty() || !host.matches("[a-z0-9.-]+")) return null;
        int n;
        if (port == null) {
            n = strict ? (scheme.equals("https") ? 443 : 80) : DEFAULT_PORT;
        } else {
            if (!port.matches("\\d{1,5}")) return null;
            n = Integer.parseInt(port);
            if (n < 1 || n > 65535) return null;
        }
        return new String[] {scheme, host, String.valueOf(n)};
    }
}
