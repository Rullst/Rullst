### Added

- The comparison page (`https://rullst.win/compare/`, `/pt-br/comparar/`)
  has a "Why not Leptos or Dioxus?" section ("Por que não Leptos ou
  Dioxus?"). It says they are UI frameworks whose components render on the
  server and run in the browser as WebAssembly, with server functions, and
  that they ship no ORM, migrations or authentication. They are not in the
  benchmark because their server side runs on Axum (Leptos also on Actix Web),
  which is already the benchmark's baseline. A Leptos or Dioxus interface can
  call a Rullst HTTP API. Facts checked against `leptos 0.8.23` and
  `dioxus 0.7.10` on 19 November 2026.
- The landing page, in both languages, and the README link the Scope and
  limits page.
