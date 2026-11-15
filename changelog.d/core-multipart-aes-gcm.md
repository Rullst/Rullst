### Changed

- With `storage-multipart`, Core encrypts the multipart recovery records
  (`MultipartCheckpoint`) with the RustCrypto `aes-gcm` crate instead of
  `ring`, so `rullst-core` no longer depends on `ring` with any feature. The
  `mp1.` format, its associated data and `MultipartKey` are unchanged:
  checkpoints written by earlier releases still open with the same key, as
  tests with records written by the `ring` code and with NIST and Wycheproof
  AES-256-GCM vectors check. The key schedule is now cleared when the last
  clone of a `MultipartKey` is dropped.
