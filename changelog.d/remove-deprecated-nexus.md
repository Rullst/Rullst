### Removed

- **Breaking:** `Nexus::build` and `Nexus::into_router`, deprecated in 12.0.0,
  were removed. Both returned a router that answered every request with 503
  when the access policy was missing or invalid. Call `Nexus::try_build()`,
  which returns that error as a `NexusBuildError` at start-up. `cargo rullst
  upgrade` reports `.build()` and `.into_router()` calls in files that name
  `Nexus` (`V13-NEXUS-INFALLIBLE-BUILD-REMOVED`, a review finding, because
  another builder in the file can have a `build` method).
