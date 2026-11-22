### Security

- The credential-free loopback access of Studio (`LocalStudioAccess`) and
  Nexus (`NexusAuthPolicy::LoopbackOnly`) trusted any request from a loopback
  peer with a local `Host`, so a reverse proxy or tunnel on the same machine
  (for example nginx's default `proxy_pass http://127.0.0.1:3000`) made every
  remote client local. Both now refuse requests with a forwarding header
  (`Forwarded`, `X-Forwarded-For`, `X-Forwarded-Host`, `X-Forwarded-Proto`,
  `X-Forwarded-Server`, `X-Real-IP`, `Via`, `CF-Connecting-IP`,
  `True-Client-IP`; other names such as `X-Forwarded-Port` or `X-Client-IP`
  are not checked) or over HTTP/1.0, and requests whose `Server` resolved
  staging or production. Building the policy now also fails with
  `LocalAccessRequiresDebugBuild` in a debug build whose process `RULLST_ENV`
  or `APP_ENV` is staging, production or invalid, and
  `NexusAuthPolicy::local_development_or_application` and
  `local_development_or_basic_from_env` then select the application's sign-in
  or Basic Auth. A proxy that adds no forwarding header and speaks HTTP/1.1
  still looks like a local browser; never publish Studio or a loopback Nexus
  through one.
