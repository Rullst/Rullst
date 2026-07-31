window.BENCHMARK_DATA = {
  "lastUpdate": 1785457454454,
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
      }
    ]
  }
}