window.BENCHMARK_DATA = {
  "lastUpdate": 1786076882762,
  "repoUrl": "https://github.com/Rullst/Rullst",
  "entries": {
    "Rullst Capital Benchmark": [
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
        "date": 1786076882405,
        "tool": "cargo",
        "benches": [
          {
            "name": "capital_subscription/parse_status_active",
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "capital_subscription/parse_status_past_due",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "capital_subscription/status_as_str",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}