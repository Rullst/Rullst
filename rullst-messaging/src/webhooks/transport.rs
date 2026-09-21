use super::*;
use reqwest::{Client, Url};
use std::net::{IpAddr, SocketAddr};

pub(super) struct PreparedRequest {
    client: Client,
    url: Url,
}
#[derive(Debug, Clone, Copy)]
pub(super) struct HttpOutcome {
    pub(super) status: u16,
    pub(super) retry_after: Option<Duration>,
}

pub(super) fn public_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(a == 0
                || a == 10
                || a == 127
                || a >= 224
                || (a == 100 && (64..=127).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192
                    && ((b == 0 && (c == 0 || c == 2)) || (b == 88 && c == 99) || b == 168))
                || (a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
                || (a == 203 && b == 0 && c == 113))
        }
        IpAddr::V6(ip) => {
            let octets = ip.octets();
            // Conservatively admit ordinary global unicast only. Deny IETF special
            // ranges, documentation, 6to4 and address-translation/mapped mechanisms.
            octets[0] & 0xe0 == 0x20
                && !(octets[0] == 0x20
                    && octets[1] == 0x01
                    && (octets[2] < 2 || (octets[2] == 0x0d && octets[3] == 0xb8)))
                && !(octets[0] == 0x20 && octets[1] == 0x02)
                && !(octets[0] == 0x3f && octets[1] == 0xff && octets[2] & 0xf0 == 0)
        }
    }
}
fn validate_addresses(addresses: &[SocketAddr], loopback: bool) -> Result<()> {
    if addresses.is_empty()
        || addresses.len() > 16
        || addresses.iter().any(|address| {
            if loopback {
                !address.ip().is_loopback()
            } else {
                !public_address(address.ip())
            }
        })
    {
        return Err(WebhookError::DestinationDenied);
    }
    Ok(())
}
impl PreparedRequest {
    pub(super) async fn resolve(destination: &WebhookDestination) -> Result<Self> {
        let host = destination
            .url
            .host_str()
            .ok_or(WebhookError::DestinationDenied)?
            .trim_matches(['[', ']']);
        let port = destination
            .url
            .port_or_known_default()
            .ok_or(WebhookError::DestinationDenied)?;
        let addresses = if let Ok(ip) = host.parse::<IpAddr>() {
            vec![SocketAddr::new(ip, port)]
        } else {
            tokio::time::timeout(
                Duration::from_secs(3),
                tokio::net::lookup_host((host, port)),
            )
            .await
            .map_err(|_| WebhookError::Timeout)?
            .map_err(|_| WebhookError::Resolution)?
            .take(17)
            .collect::<Vec<_>>()
        };
        validate_addresses(&addresses, destination.loopback)?;
        let mut builder = Client::builder()
            .no_proxy()
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .redirect(reqwest::redirect::Policy::none())
            .https_only(!destination.loopback)
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(3))
            .pool_max_idle_per_host(0)
            .resolve_to_addrs(host, &addresses);
        if let Some(pem) = &destination.test_root {
            if !destination.loopback {
                return Err(WebhookError::Configuration);
            }
            builder = builder.add_root_certificate(
                reqwest::Certificate::from_pem(pem).map_err(|_| WebhookError::Configuration)?,
            );
        }
        let client = builder.build().map_err(|_| WebhookError::Configuration)?;
        Ok(Self {
            client,
            url: destination.url.clone(),
        })
    }
    pub(super) async fn send(
        self,
        signature: &WebhookSignature,
        body: &[u8],
        timeout: Duration,
    ) -> Result<HttpOutcome> {
        let mut signature_header =
            reqwest::header::HeaderValue::from_str(&signature.signature_header())
                .map_err(|_| WebhookError::InvalidSignature)?;
        signature_header.set_sensitive(true);
        let response = self
            .client
            .post(self.url)
            .timeout(timeout)
            .header("content-type", "application/json")
            .header("rullst-webhook-id", signature.delivery_id())
            .header("rullst-webhook-type", signature.event_kind())
            .header(
                "rullst-webhook-timestamp",
                signature.timestamp_seconds().to_string(),
            )
            .header("rullst-webhook-key-id", signature.key_id())
            .header("rullst-webhook-signature", signature_header)
            .body(body.to_vec())
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    WebhookError::Timeout
                } else {
                    WebhookError::Transport
                }
            })?;
        // Receiver bodies and arbitrary headers are never buffered, persisted or logged.
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .filter(|v| v.len() <= 4)
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|v| *v <= 3600)
            .map(Duration::from_secs);
        Ok(HttpOutcome {
            status: response.status().as_u16(),
            retry_after,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_special_transition_and_mixed_dns_addresses_are_denied() {
        for value in [
            "0.0.0.0",
            "10.0.0.1",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.169.254",
            "172.16.0.1",
            "192.0.0.1",
            "192.0.2.1",
            "192.88.99.1",
            "192.168.0.1",
            "198.18.0.1",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "255.255.255.255",
            "::",
            "::1",
            "::ffff:127.0.0.1",
            "64:ff9b::a00:1",
            "fc00::1",
            "fe80::1",
            "ff02::1",
            "2001::1",
            "2001:20::1",
            "2001:db8::1",
            "2002:7f00:1::1",
            "3fff::1",
        ] {
            assert!(!public_address(value.parse().unwrap()), "{value}");
        }
        for value in ["1.1.1.1", "8.8.8.8", "2606:4700:4700::1111"] {
            assert!(public_address(value.parse().unwrap()));
        }
        let public: SocketAddr = "1.1.1.1:443".parse().unwrap();
        let private = "127.0.0.1:443".parse().unwrap();
        assert!(validate_addresses(&[public, private], false).is_err());
        assert!(validate_addresses(&[public], true).is_err());
        assert!(validate_addresses(&[private], true).is_ok());
        assert!(validate_addresses(&[], false).is_err());
        assert!(validate_addresses(&[public; 17], false).is_err());
    }
}
