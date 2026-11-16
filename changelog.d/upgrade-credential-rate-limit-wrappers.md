### Fixed

- `cargo rullst upgrade` no longer reports `V13-CREDENTIAL-RATE-LIMIT` at a
  call of the application's own `hash_password` (through `crate`, `self`,
  `super`, or a module or function the file imports from them or declares,
  such as `auth::hash_password` after `use crate::auth;`) when the package
  defines that function with its hashing under `run_password_work` or a
  semaphore permit, even in another module. Other calls (`rullst::auth`,
  `rullst_auth`, names imported from Rullst or another crate, glob imports,
  methods) are still reported, now at every unbounded call of a function
  rather than the first. The finding's message
  says that `// rullst-upgrade: reviewed V13-CREDENTIAL-RATE-LIMIT: <reason>`
  on the line above a call resolves a limit the rule does not follow.
  Reported by the plataforma application.
