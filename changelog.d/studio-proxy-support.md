### Added

- The standalone Studio can reach monitored projects and webhooks through a
  proxy. The **Proxy** section of its Settings page selects direct
  connections (the default, as before), the proxy variables of its
  environment, or an explicit `http://` or `https://` proxy URL with a
  `NO_PROXY`-style exception list. The variables are chosen by the target's
  scheme, as curl and reqwest choose them: `HTTPS_PROXY` (else `ALL_PROXY`)
  for `https://` and `HTTP_PROXY` (else `ALL_PROXY`) for `http://`, upper case
  before lower case, except for the hosts of `NO_PROXY`; a proxy that needs a
  password comes from them and is never stored. Loopback hosts are always
  direct and Studio sends `http://` only to loopback, so only `https://`
  connections use the proxy, as a `CONNECT` tunnel that never sees a token.
  SOCKS proxies are not supported: a `socks5://` or `socks5h://` value, like
  a malformed one, blocks every connection instead of connecting directly,
  and the Settings page names the variable or setting.
