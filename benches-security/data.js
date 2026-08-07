window.BENCHMARK_DATA = {
  "lastUpdate": 1786076731215,
  "repoUrl": "https://github.com/Rullst/Rullst",
  "entries": {
    "Rullst Security Benchmark": [
      {
        "commit": {
          "author": {
            "email": "venelouistyago@gmail.com",
            "name": "Venelouis",
            "username": "venelouis"
          },
          "committer": {
            "email": "venelouistyago@gmail.com",
            "name": "Venelouis",
            "username": "venelouis"
          },
          "distinct": true,
          "id": "6a5e4e9d7ffbe9e81ee7bb5466f84b0cb806cef6",
          "message": "refactor: streamline studio routing, introduce Redis feature, and add architecture documentation",
          "timestamp": "2026-08-07T01:14:34-03:00",
          "tree_id": "07f21b4b693884ecad509c4048403f0e9ad1be20",
          "url": "https://github.com/Rullst/Rullst/commit/6a5e4e9d7ffbe9e81ee7bb5466f84b0cb806cef6"
        },
        "date": 1786076730834,
        "tool": "cargo",
        "benches": [
          {
            "name": "html_sanitizer/sanitize_html_xss",
            "value": 170800,
            "range": "± 42018",
            "unit": "ns/iter"
          },
          {
            "name": "html_sanitizer/sanitize_text_escape",
            "value": 557,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "rbac_guard/authorize_role",
            "value": 14,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "rbac_guard/authorize_owner_or_role",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "vault_secret/vault_secret_new_and_drop",
            "value": 26,
            "range": "± 1",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}