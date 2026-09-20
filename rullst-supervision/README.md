# Rullst Supervision

Unpublished v13 implementation candidate. Optional exam-session observations and
parental application restrictions use explicit scoped authority and bounded local
SQLite state. The application owns authentication, current school membership,
resource authorization and independent guardian/reviewer verification.

`exam` provides visible-session contracts. `parental` provides course/window
policy contracts. `sqlite` selects both and the shared-local durable adapter.
There are no default features or Core dependencies. No camera, microphone,
screen, browsing-history or device-wide control is implemented. Visibility
reports never prove misconduct or affect grades.

The [design and acceptance boundary](../docs/src/supervision.md) records the
supported scope, local generated-LMS/Chromium evidence and remaining archive,
full-workspace and hosted release requirements.
This package is not yet admitted for publication or production use.
