### Fixed

- `V13-CSRF-STATIC-PREFIXES` reported `nest_service` and `route_service`
  calls but not the `GET` routes that serve files outside
  `csrf_static_prefixes` and `csrf_static_paths`, which the same CSRF change
  concerns. It now also reports, for review, `.route("/path",
  get_service(..))` with a `ServeDir` or `ServeFile`, and a
  `.route("/path", get(..))` or `routes!` `get("/path" => ..)` whose path ends
  in an extension that 13.0.0-alpha.5 treated as static (`.xml`, `.json`,
  `.txt`, `.css`, `.js`, images, `.wasm`), unless the default settings cover
  it (#826).
