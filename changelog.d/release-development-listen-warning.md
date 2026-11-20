### Changed

- A `Server` running as development or test on a non-loopback address now logs
  a `warn` event in release builds too, naming `RULLST_ENV` and saying that the
  WAF only reports (development) or is off (test) and that HSTS is not sent
  over plain HTTP. With no `RULLST_ENV`, `APP_ENV` or `[app].env`, the
  environment is development, so a release binary started with `HOST=0.0.0.0`
  previously ran with development defaults without any message. It still
  starts.
