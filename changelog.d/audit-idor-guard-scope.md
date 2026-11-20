### Changed

- `cargo rullst audit --idor` now prints that its guard evidence is crate-wide:
  one `RbacGuard::authorize_owner_or_role` call anywhere in a crate satisfies
  every `owner` route of that crate (likewise for `role` and `admin`), so each
  handler still needs its own check.
