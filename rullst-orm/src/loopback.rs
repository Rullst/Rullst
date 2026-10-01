//! Loopback host detection shared by the network adapters' local-only modes.

use std::net::IpAddr;

/// Reports whether a URL host names the local machine: `localhost` or a
/// loopback IP address. `Url::host_str` keeps the brackets of an IPv6
/// literal (`[::1]`), so they are removed before parsing the address.
pub(crate) fn is_loopback_host(host: &str) -> bool {
    let host = host
        .strip_prefix('[')
        .and_then(|address| address.strip_suffix(']'))
        .unwrap_or(host);
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

#[cfg(test)]
mod tests {
    use super::is_loopback_host;

    #[test]
    fn bracketed_ipv6_and_ipv4_loopback_hosts_are_local() {
        for host in [
            "localhost",
            "LOCALHOST",
            "127.0.0.1",
            "127.0.0.2",
            "[::1]",
            "::1",
        ] {
            assert!(is_loopback_host(host), "{host}");
        }
        for host in [
            "example.com",
            "10.0.0.1",
            "[2001:db8::1]",
            "[::1",
            "::1]",
            "",
        ] {
            assert!(!is_loopback_host(host), "{host}");
        }
    }
}
