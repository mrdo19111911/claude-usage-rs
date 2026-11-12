// Who may ask for the feed: this machine, private networks, Tailscale.
use std::net::{IpAddr, Ipv4Addr};

pub(crate) fn allowed(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4_ok(v4),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => v4_ok(v4),
            // ::1, or fd00::/8 (unique local; Tailscale uses fd7a:115c:a1e0::/48).
            None => v6.is_loopback() || v6.octets()[0] == 0xfd,
        },
    }
}

fn v4_ok(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    // 100.64.0.0/10: carrier-grade NAT, the range Tailscale gives its devices.
    ip.is_loopback() || ip.is_private() || (a == 100 && (64..128).contains(&b))
}
