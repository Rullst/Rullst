### Added

- The standalone Studio can show alerts as desktop notifications. On the new
  **Settings** page (`/studio/settings`), enable them and allow them once in
  the browser; every Studio page then reads new notifications every 15
  seconds and shows them with the browser's Notifications API. A project is
  notified at most once per quiet period (10 minutes by default, 1 to
  1,440); alerts in between are counted in its next notification or in a
  summary when the period ends. The setting is kept in the project list.
- Two alert rules in the standalone Studio: **p99 latency** and **Security
  events** (this many security events recorded between two polls, from a
  `rullst.monitor.v2` document). The history keeps p99 latency and the
  security events total; an older history database gains the two columns
  when it opens.
