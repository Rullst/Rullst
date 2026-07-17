window.BENCHMARK_DATA = {
  "lastUpdate": 1784251858686,
  "repoUrl": "https://github.com/Rullst/Rullst",
  "entries": {
    "Benchmark": [
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
          "id": "846ef173d25fdc9d65436fd2755bc71c787d5ead",
          "message": "5.0.0",
          "timestamp": "2026-07-16T22:26:46-03:00",
          "tree_id": "47099e8f533d69946a29d70fd1494186f019261b",
          "url": "https://github.com/Rullst/Rullst/commit/846ef173d25fdc9d65436fd2755bc71c787d5ead"
        },
        "date": 1784251858389,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 741,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 994,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "html_macro_static",
            "value": 7,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "html_macro_dynamic",
            "value": 625,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2275,
            "range": "± 33",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}