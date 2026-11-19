### Changed

- **Breaking:** `rullst-labs` renamed two public enum variants that carried the
  word Experimental, which was never a maturity label: the profile whose
  signed receipts `complete` and `reconcile_cleanup` accept,
  `ExecutionProfile::LinuxExperimental`, is now `ExecutionProfile::PinnedRunner`
  (the operator pins its tools and receipt key; the crate never checked that
  the runner is Linux), and the evidence of a graded or rejected signed
  receipt, `ResultEvidence::Experimental`, is now
  `ResultEvidence::SignedReceipt`. Only the Rust names changed: both are still
  serialized as `LinuxExperimental` and `Experimental`, so profile digests,
  store bindings, job records and receipt bindings are byte-identical, and
  stores created by an earlier release open and verify unchanged. Rename the
  paths in code; `cargo rullst upgrade` reports them
  (`V13-LABS-VARIANTS-RENAMED`). A JSON or TOML configuration that names the
  profile keeps `LinuxExperimental`.
