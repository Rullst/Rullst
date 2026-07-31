window.BENCHMARK_DATA = {
  "lastUpdate": 1785459547243,
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
        "date": 1785458665488,
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
            "value": 461,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 359,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 206,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2038649,
            "range": "± 46559",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 98479,
            "range": "± 1388",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 102190,
            "range": "± 1257",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 97512,
            "range": "± 2869",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105224,
            "range": "± 1603",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 95378,
            "range": "± 2107",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 105622,
            "range": "± 1114",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 152348,
            "range": "± 3633",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 246656,
            "range": "± 3578",
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
          "id": "edbb6a2ea6970d419d0c187c9644800e5cd4755b",
          "message": "feat: add landing page template, benchmark scaffold, GitHub Pages workflow, and footer dedication",
          "timestamp": "2026-07-30T21:54:41-03:00",
          "tree_id": "9d403da5db172f7c61af25a683450b2e6a7e4273",
          "url": "https://github.com/Rullst/Rullst/commit/edbb6a2ea6970d419d0c187c9644800e5cd4755b"
        },
        "date": 1785459546699,
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
            "value": 45,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 461,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 362,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 197,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2007569,
            "range": "± 33932",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 100362,
            "range": "± 952",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 105669,
            "range": "± 1301",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 99530,
            "range": "± 2837",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 106633,
            "range": "± 1457",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 98039,
            "range": "± 2945",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 108089,
            "range": "± 2036",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154916,
            "range": "± 2221",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 244487,
            "range": "± 2256",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}