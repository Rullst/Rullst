### Changed

- Building the local Studio boundary (`LocalStudioAccess::protect_router`) or
  an explicit loopback Nexus policy in a debug build whose environment is
  staging, production or invalid, or whose `.env` or `Rullst.toml` cannot be
  read, now fails with the new
  `StudioBuildError::LocalAccessRequiresDevelopmentEnvironment` or
  `NexusBuildError::LocalAccessRequiresDevelopmentEnvironment`, whose message
  names the environment, instead of `LocalAccessRequiresDebugBuild`, which
  now means a release build only. Code that matched
  `LocalAccessRequiresDebugBuild` to detect the environment case needs to
  match the new variant ([#842](https://github.com/Rullst/Rullst/issues/842)).
