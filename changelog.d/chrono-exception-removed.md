### Security

- The time-bounded exception for RUSTSEC-2026-0543 (`chrono` unmaintained),
  added on 17 November 2026, is removed: no Rullst crate, generator, example,
  fuzz crate or fixture resolves `chrono` any more, so the supply-chain
  audits check it again with no exception.
