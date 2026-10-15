//! The startup line that states where the server listens.

use std::net::SocketAddr;

/// The line `Server` prints once it is bound: the listen address and, when it
/// covers every network interface (`0.0.0.0` or `::`, the staging and
/// production default) while a trusted reverse proxy is configured, a hint to
/// bind loopback so clients cannot bypass that proxy.
pub(super) fn listen_notice(address: SocketAddr, behind_trusted_proxy: bool) -> String {
    let mut line = format!("Rullst framework serving on http://{address}");
    if address.ip().is_unspecified() {
        line.push_str(" (every network interface");
        if behind_trusted_proxy {
            line.push_str(
                "; a trusted reverse proxy is configured: if it runs on this host, set HOST=127.0.0.1 so clients reach the application only through it",
            );
        }
        line.push(')');
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_names_the_address_and_hints_at_loopback_behind_a_proxy() {
        let loopback = SocketAddr::from(([127, 0, 0, 1], 3000));
        for proxied in [false, true] {
            assert_eq!(
                listen_notice(loopback, proxied),
                "Rullst framework serving on http://127.0.0.1:3000"
            );
        }
        let lan = SocketAddr::from(([192, 168, 1, 5], 8080));
        assert_eq!(
            listen_notice(lan, true),
            "Rullst framework serving on http://192.168.1.5:8080"
        );
        let every = SocketAddr::from(([0, 0, 0, 0], 3000));
        assert_eq!(
            listen_notice(every, false),
            "Rullst framework serving on http://0.0.0.0:3000 (every network interface)"
        );
        assert_eq!(
            listen_notice(every, true),
            "Rullst framework serving on http://0.0.0.0:3000 (every network interface; a trusted reverse proxy is configured: if it runs on this host, set HOST=127.0.0.1 so clients reach the application only through it)"
        );
        let every_v6: SocketAddr = "[::]:3000".parse().unwrap_or(every);
        assert_eq!(
            listen_notice(every_v6, false),
            "Rullst framework serving on http://[::]:3000 (every network interface)"
        );
    }
}
