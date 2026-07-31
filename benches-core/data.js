window.BENCHMARK_DATA = {
  "lastUpdate": 1785458811976,
  "repoUrl": "https://github.com/Rullst/Rullst",
  "entries": {
    "Rullst Core Primitives Benchmark": [
      {
        "commit": {
          "author": {
            "email": "venelouistyago@gmail.com",
            "name": "venelouis",
            "username": "venelouis"
          },
          "committer": {
            "email": "venelouistyago@gmail.com",
            "name": "venelouis",
            "username": "venelouis"
          },
          "distinct": true,
          "id": "940d213fcdbfb254abe46f095ddc9cc3015ef354",
          "message": "ci: add GitHub Actions workflows for DAST, Kani formal verification, and Miri UB detection",
          "timestamp": "2026-07-30T21:13:48-03:00",
          "tree_id": "28bcab87dc439d0fd8df6bf6f6b428795f102f3b",
          "url": "https://github.com/Rullst/Rullst/commit/940d213fcdbfb254abe46f095ddc9cc3015ef354"
        },
        "date": 1785457362168,
        "tool": "cargo",
        "benches": [
          {
            "name": "html_escape/clean_input_no_escape",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "html_escape/malicious_input_full_escape",
            "value": 206,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "html_escape/realistic_partial_escape",
            "value": 69,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "mask_pii/email_field",
            "value": 159,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "mask_pii/credit_card_field",
            "value": 279,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "mask_pii/phone_field",
            "value": 956,
            "range": "± 55",
            "unit": "ns/iter"
          },
          {
            "name": "mask_pii/safe_field_no_pii",
            "value": 183,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "generate_csrf_token_32_chars",
            "value": 125,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "venelouistyago@gmail.com",
            "name": "venelouis",
            "username": "venelouis"
          },
          "committer": {
            "email": "venelouistyago@gmail.com",
            "name": "venelouis",
            "username": "venelouis"
          },
          "distinct": true,
          "id": "aa0db2996e91a959a689e5eebc920f20afc5e4c3",
          "message": "feat: add project documentation, CI workflows, and initial ORM/auth benchmarking infrastructure",
          "timestamp": "2026-07-30T21:38:47-03:00",
          "tree_id": "e3bc94fa6f951e72090b5386cddb937f6d84e51c",
          "url": "https://github.com/Rullst/Rullst/commit/aa0db2996e91a959a689e5eebc920f20afc5e4c3"
        },
        "date": 1785458811450,
        "tool": "cargo",
        "benches": [
          {
            "name": "html_escape/clean_input_no_escape",
            "value": 22,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "html_escape/malicious_input_full_escape",
            "value": 214,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "html_escape/realistic_partial_escape",
            "value": 69,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "mask_pii/email_field",
            "value": 159,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "mask_pii/credit_card_field",
            "value": 282,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "mask_pii/phone_field",
            "value": 957,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "mask_pii/safe_field_no_pii",
            "value": 168,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "generate_csrf_token_32_chars",
            "value": 124,
            "range": "± 1",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}