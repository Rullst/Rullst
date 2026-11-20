### Fixed

- The Resend and SendPulse offline mocks (empty or `mock_*` API keys) accepted
  messages their live delivery refuses, so a test could pass for mail
  production rejects. They now run the same checks before capturing: a
  missing sender and a Resend schedule beyond 30 days ahead, and for SendPulse
  a future schedule, an empty body, inline CID attachments, unsubscribe
  headers and duplicate attachment names. A test that sent such a message
  through a mock now gets the error live delivery returns.
