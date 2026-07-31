window.BENCHMARK_DATA = {
  "lastUpdate": 1785458895700,
  "repoUrl": "https://github.com/Rullst/Rullst",
  "entries": {
    "Rullst Auth Benchmark": [
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
        "date": 1785457454114,
        "tool": "cargo",
        "benches": [
          {
            "name": "session_crypto/encrypt_session",
            "value": 727,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "session_crypto/decrypt_session",
            "value": 623,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "session_crypto/round_trip_encrypt_decrypt",
            "value": 1371,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "make_login_cookie",
            "value": 1408,
            "range": "± 8",
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
        "date": 1785458895179,
        "tool": "cargo",
        "benches": [
          {
            "name": "session_crypto/encrypt_session",
            "value": 727,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "session_crypto/decrypt_session",
            "value": 636,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "session_crypto/round_trip_encrypt_decrypt",
            "value": 1446,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "make_login_cookie",
            "value": 1436,
            "range": "± 8",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}