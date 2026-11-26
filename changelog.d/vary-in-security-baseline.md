### Changed

- With the access log off (`RULLST_ACCESS_LOG=off`), `Server` now adds
  `Vary: HX-Request` to the HTML responses of the application's routes inside
  its security baseline middleware instead of in a second middleware around
  the routes, so every request runs one middleware fewer. Responses keep the
  same headers in the same order. A debug build in Development, where the
  error console sits between the two, and PII masking keep the separate
  middleware; with the access log on, the access log middleware still adds
  the header as before.
