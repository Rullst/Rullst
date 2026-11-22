### Changed

- The privacy notice of `https://rullst.win/` (and its summary on the
  landing, in both languages) names Cloudflare, the content delivery and
  security service in front of GitHub Pages: it receives every request to
  rullst.win and, unless those features are turned off for the domain, can
  add its own email-obfuscation and bot-detection scripts to the pages and
  asks browsers to send it network-error reports.
- The landing and comparison pages state what their evidence supports: the
  fail-closed check catches unreviewed fallbacks that match its known
  patterns; without `RULLST_ENV=production` (or staging) an application runs
  as development, where the WAF only logs; rc.1 also changes what a working
  alpha.6 deployment does (with a link to the new checklist); the memory
  proof point shows the benchmark's Memory score (a tie with Axum in the
  current run) and that the RAM after the logins is in no score; the
  results of the run measured on 20–21 November 2026 carry a note that an
  automatic Rullst/Benchmarks run would not publish them; Stripe is
  described as used for real purchases in the maintainer's production SaaS,
  with no recorded acceptance run; and the comparison page says when its
  rows were last checked, that every starter mounts CSRF protection, and
  which of the seven GitHub contributors are bots or an AI agent's account.
