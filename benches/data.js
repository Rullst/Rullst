window.BENCHMARK_DATA = {
  "lastUpdate": 1784252638761,
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
          "id": "b79bbe8a5ba238ef7bf35711ae97379823da765f",
          "message": "feat: implement subscription billing engine with Stripe provider support and add core feature and queue modules",
          "timestamp": "2026-07-16T22:40:22-03:00",
          "tree_id": "7f83a59f45e824ca2a3c18f0208a7169bf28e9b1",
          "url": "https://github.com/Rullst/Rullst/commit/b79bbe8a5ba238ef7bf35711ae97379823da765f"
        },
        "date": 1784252637986,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 574,
            "range": "± 68",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 764,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "html_macro_static",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "html_macro_dynamic",
            "value": 510,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 1720,
            "range": "± 20",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}