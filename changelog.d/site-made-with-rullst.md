### Added

- The landing page (`https://rullst.win/`, `/pt-br/`) has a "Made with Rullst"
  card listing the maintainer's live applications built with Rullst (Rullst
  Academy, Capoeira Vene, vene.win, the Rullst/Benchmarks results website and
  these pages), each with a link and one line from its own front page, and
  its first screen names the optional packages around the framework (AI,
  payments, mail, messaging, IoT devices, learning platforms and privacy),
  each linked to its documentation. Every page's footer says which Rullst
  version made it. Each landing's footer also shows its page weight ("This
  page: N kB in M requests on a first visit"), measured on the published site
  with `cargo rullst footprint --page-weight` during the daily deployment and
  left out when that measurement fails or does not pass its checks.
