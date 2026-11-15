### Added

- The standalone Studio can reach monitored projects and webhooks through a
  proxy. The **Proxy** section of its Settings page selects direct
  connections (the default, as before), the `HTTPS_PROXY` and `NO_PROXY`
  variables of its environment, or an explicit `http://` or `https://` proxy
  URL with a `NO_PROXY`-style exception list. Only `https://` connections use
  the proxy, as a `CONNECT` tunnel, so it never sees a token; loopback hosts
  and plain `http://` (loopback only) stay direct, and a proxy that cannot be
  used blocks every connection instead of connecting directly.
