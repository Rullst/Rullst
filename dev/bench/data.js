window.BENCHMARK_DATA = {
  "lastUpdate": 1782608626820,
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
          "id": "d34d9e7384ce358950e2a8fba020a003ddd37159",
          "message": "fix(ci): clona repo na raiz para benchmark-action achar arvore git",
          "timestamp": "2026-06-27T21:54:22-03:00",
          "tree_id": "e8cd6b71cec978810b6a6d3c5ea7b338a21acd4a",
          "url": "https://github.com/Rullst/Rullst/commit/d34d9e7384ce358950e2a8fba020a003ddd37159"
        },
        "date": 1782608626415,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 624,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 904,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "html_macro_static",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "html_macro_dynamic",
            "value": 648,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 1842,
            "range": "± 22",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}