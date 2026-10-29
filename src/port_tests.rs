// The port of the last run again (docs/specs/modules.md, "Tray behaviour").

use super::*;

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("cud-port-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn port_of(s: &tiny_http::Server) -> u16 {
    s.server_addr().to_ip().unwrap().port()
}

#[test]
fn a_restart_gets_the_same_port() {
    let d = dir("same");
    let first = port_of(&listen(&d, 0, true).unwrap());
    assert_eq!(fs::read_to_string(d.join("port")).unwrap(), first.to_string(), "kept");
    // The first server is gone (dropped above): the next run takes its port.
    assert_eq!(port_of(&listen(&d, 0, true).unwrap()), first);
}

#[test]
fn a_taken_port_gives_way_to_a_free_one() {
    let d = dir("taken");
    let other = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let taken = other.local_addr().unwrap().port();
    fs::write(d.join("port"), taken.to_string()).unwrap();
    let got = port_of(&listen(&d, 0, true).unwrap());
    assert_ne!(got, taken);
    assert_eq!(fs::read_to_string(d.join("port")).unwrap(), got.to_string(), "the new one is kept");
}

#[test]
fn a_set_port_is_used_and_not_kept() {
    let d = dir("set");
    let free = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    assert_eq!(port_of(&listen(&d, free, true).unwrap()), free);
    assert!(!d.join("port").exists(), "PORT or the fixed one is not remembered");
    // The demo keeps nothing either.
    listen(&d, 0, false).unwrap();
    assert!(!d.join("port").exists());
}
