use super::*;

fn ok(s: &str) -> bool {
    allowed(s.parse().unwrap())
}

#[test]
fn this_machine_private_networks_and_tailscale() {
    for a in ["127.0.0.1", "::1", "10.0.0.1", "10.255.255.255", "172.16.0.1", "172.31.255.255",
              "192.168.0.1", "192.168.255.254", "100.64.0.1", "100.127.255.255",
              "fd7a:115c:a1e0::1", "fd00::1", "::ffff:192.168.1.5"] {
        assert!(ok(a), "{a} should be allowed");
    }
}

#[test]
fn everything_else_is_refused() {
    for a in ["8.8.8.8", "172.15.255.255", "172.32.0.1", "100.63.255.255", "100.128.0.1",
              "192.169.0.1", "11.0.0.1", "2001:4860::8888", "fe80::1", "fc00::1", "::ffff:8.8.8.8", "0.0.0.0"] {
        assert!(!ok(a), "{a} should be refused");
    }
}
