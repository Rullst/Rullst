window.BENCHMARK_DATA = {
  "lastUpdate": 1785457212645,
  "repoUrl": "https://github.com/Rullst/Rullst",
  "entries": {
    "Rullst ORM Benchmark": [
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
        "date": 1785457212282,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 11,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 14,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 46,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 466,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 362,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 200,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2064047,
            "range": "± 86278",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 99648,
            "range": "± 1193",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 104469,
            "range": "± 1547",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 98592,
            "range": "± 2469",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 106511,
            "range": "± 1860",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 98633,
            "range": "± 1995",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 107917,
            "range": "± 2735",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 152450,
            "range": "± 1900",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 246175,
            "range": "± 2853",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}