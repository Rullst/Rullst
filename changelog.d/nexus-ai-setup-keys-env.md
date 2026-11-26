### Fixed

- Nexus's AI setup answer, its chat page and its suggested prompt told the reader to put
  provider keys in `.env`, but `AutoAiConfig` reads them from the process environment only
  (`Server` and `cargo rullst dev` do not load `.env` into the process), so a key written
  there was never used. The text now says to set the variables in the environment of the
  process that runs the application.
