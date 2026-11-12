// Tests for Address (docs/specs/android.md "The app"). Plain java, no JUnit:
// run with test.ps1; exits 1 when any check fails.

import vn.smartlog.scas.companion.Address;

public class AddressTest {
    static int failed = 0;

    static void eq(String name, String want, String got) {
        boolean ok = want == null ? got == null : want.equals(got);
        if (!ok) {
            failed++;
            System.out.println("FAIL " + name + ": want " + want + ", got " + got);
        }
    }

    static void is(String name, boolean want, boolean got) {
        eq(name, String.valueOf(want), String.valueOf(got));
    }

    public static void main(String[] a) {
        // normalize
        eq("full url kept", "http://127.0.0.1:4777", Address.normalize("http://127.0.0.1:4777"));
        eq("spaces trimmed", "http://127.0.0.1:4777", Address.normalize("  http://127.0.0.1:4777  "));
        eq("port only", "http://127.0.0.1:52001", Address.normalize("52001"));
        eq("colon port", "http://127.0.0.1:52001", Address.normalize(":52001"));
        eq("no scheme", "http://localhost:4777", Address.normalize("localhost:4777"));
        eq("no port -> 4777", "http://127.0.0.1:4777", Address.normalize("127.0.0.1"));
        eq("https kept", "https://pc.example.ts.net:4777", Address.normalize("https://pc.example.ts.net:4777"));
        eq("path dropped", "http://127.0.0.1:4777", Address.normalize("http://127.0.0.1:4777/api/state?x=1"));
        eq("trailing slash", "http://127.0.0.1:4777", Address.normalize("http://127.0.0.1:4777/"));
        eq("scheme case", "http://127.0.0.1:4777", Address.normalize("HTTP://127.0.0.1:4777"));
        eq("empty", null, Address.normalize(""));
        eq("blank", null, Address.normalize("   "));
        eq("null", null, Address.normalize(null));
        eq("file scheme", null, Address.normalize("file:///sdcard/x.html"));
        eq("javascript scheme", null, Address.normalize("javascript:alert(1)"));
        eq("no host", null, Address.normalize("http://:4777"));
        eq("port 0", null, Address.normalize("127.0.0.1:0"));
        eq("port too big", null, Address.normalize("127.0.0.1:70000"));
        eq("port not a number", null, Address.normalize("127.0.0.1:abc"));
        eq("user info refused", null, Address.normalize("http://me@127.0.0.1:4777"));
        // passesHostCheck: SCAS (server.rs) accepts only these two names
        is("127.0.0.1 passes", true, Address.passesHostCheck("http://127.0.0.1:4777"));
        is("localhost passes", true, Address.passesHostCheck("http://localhost:4777"));
        is("LAN refused", false, Address.passesHostCheck("http://192.168.1.5:4777"));
        is("tailnet refused", false, Address.passesHostCheck("https://pc.example.ts.net:4777"));
        is("lookalike refused", false, Address.passesHostCheck("http://127.0.0.1.evil.com:4777"));
        is("garbage refused", false, Address.passesHostCheck("nonsense"));
        // sameOrigin: links elsewhere open in the phone's browser
        is("same origin", true, Address.sameOrigin("http://127.0.0.1:4777", "http://127.0.0.1:4777/help.html"));
        is("other port", false, Address.sameOrigin("http://127.0.0.1:4777", "http://127.0.0.1:4778/"));
        is("other host", false, Address.sameOrigin("http://127.0.0.1:4777", "https://github.com/x"));
        is("callback on same origin", true, Address.sameOrigin("http://localhost:4777", "http://localhost:4777/callback?code=1"));

        if (failed > 0) {
            System.out.println(failed + " check(s) failed");
            System.exit(1);
        }
        System.out.println("AddressTest: all checks passed");
    }
}
