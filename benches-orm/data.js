window.BENCHMARK_DATA = {
  "lastUpdate": 1786246460038,
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
          "id": "b19612b56bc0d6123246aad6068b2602fe5f75e9",
          "message": "fix: update Matrix DB Tests CI badge URL in README",
          "timestamp": "2026-07-30T22:21:40-03:00",
          "tree_id": "3ba8d910cccbef9e3c65a2f9fde6c58d51441445",
          "url": "https://github.com/Rullst/Rullst/commit/b19612b56bc0d6123246aad6068b2602fe5f75e9"
        },
        "date": 1785461159881,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 9,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 320,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 217,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 127,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 3905004,
            "range": "± 9798594",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 60935,
            "range": "± 1308",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 66293,
            "range": "± 1902",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 65586,
            "range": "± 2650",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 71710,
            "range": "± 1426",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 63665,
            "range": "± 1569",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 70703,
            "range": "± 2606",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 131686,
            "range": "± 12075",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 212537,
            "range": "± 21022",
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
          "id": "635d3f9d45e3e4d1b0cb56c2abbd5b591223cc37",
          "message": "feat: implement Nexus admin panel framework, authentication module, and GitHub Pages CI/CD workflow",
          "timestamp": "2026-07-30T22:38:04-03:00",
          "tree_id": "85427af62e589bbd3956c095b92201c53f3cddc5",
          "url": "https://github.com/Rullst/Rullst/commit/635d3f9d45e3e4d1b0cb56c2abbd5b591223cc37"
        },
        "date": 1785462146728,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 471,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 363,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 199,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2004412,
            "range": "± 62825",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 98536,
            "range": "± 1043",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 104190,
            "range": "± 1118",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 96977,
            "range": "± 3818",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 106368,
            "range": "± 2815",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 94464,
            "range": "± 3370",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 106797,
            "range": "± 1570",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 151746,
            "range": "± 2539",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 240769,
            "range": "± 3305",
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
          "id": "b199e916ab5738bc74e262aef77833a48357642d",
          "message": "feat: implement authentication utilities and encrypted session management services",
          "timestamp": "2026-07-30T22:56:03-03:00",
          "tree_id": "6558f418ada77fe896f65b85ab6e0e2f1531af6f",
          "url": "https://github.com/Rullst/Rullst/commit/b199e916ab5738bc74e262aef77833a48357642d"
        },
        "date": 1785463224514,
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
            "value": 357,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 195,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1938766,
            "range": "± 197952",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 99991,
            "range": "± 1219",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 105463,
            "range": "± 1170",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 98283,
            "range": "± 1958",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 106756,
            "range": "± 1432",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 97757,
            "range": "± 1985",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 107273,
            "range": "± 1774",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 152259,
            "range": "± 2005",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 248411,
            "range": "± 2791",
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
          "id": "5bc2b02a839bfc81fc11ef756efbbf35d31b6682",
          "message": "feat: add CodeQL static analysis workflow for Rust project",
          "timestamp": "2026-07-30T23:07:25-03:00",
          "tree_id": "c36fb3bdbfc9c190e2ec29f0ca9b015c87a6228f",
          "url": "https://github.com/Rullst/Rullst/commit/5bc2b02a839bfc81fc11ef756efbbf35d31b6682"
        },
        "date": 1785463924093,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 7,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 38,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 427,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 294,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 180,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2423097,
            "range": "± 6197275",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 61647,
            "range": "± 1597",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 69166,
            "range": "± 1728",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 65731,
            "range": "± 1105",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 75595,
            "range": "± 1032",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 67567,
            "range": "± 990",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 75995,
            "range": "± 1236",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153208,
            "range": "± 4654",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 271644,
            "range": "± 14258",
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
          "id": "283a69f32489084940c9f8e251c88556f4c8445c",
          "message": "feat: add abstract HttpClient client interface and implement GitHub CI workflows for security and verification tools",
          "timestamp": "2026-07-30T23:27:34-03:00",
          "tree_id": "6308cbb27bb74aca27269bdca8dde3c3da063023",
          "url": "https://github.com/Rullst/Rullst/commit/283a69f32489084940c9f8e251c88556f4c8445c"
        },
        "date": 1785465133691,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 7,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 13,
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
            "value": 440,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 302,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 184,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2485232,
            "range": "± 658303",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 65961,
            "range": "± 1266",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 71151,
            "range": "± 1074",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 68326,
            "range": "± 1320",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 78496,
            "range": "± 1759",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 70423,
            "range": "± 1566",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 78811,
            "range": "± 1751",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 157266,
            "range": "± 6332",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 278948,
            "range": "± 10746",
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
          "id": "f940d3e86f6924c1b827ea69307c7df72a833b4b",
          "message": "chore: initialize comprehensive CI/CD pipeline with GitHub Actions workflows",
          "timestamp": "2026-07-31T13:53:20-03:00",
          "tree_id": "8a58fdf8891b79cd5ddb99a10e01986bfe4c0fda",
          "url": "https://github.com/Rullst/Rullst/commit/f940d3e86f6924c1b827ea69307c7df72a833b4b"
        },
        "date": 1785517082569,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 46,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 520,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 392,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 200,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1520444,
            "range": "± 78304",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 81193,
            "range": "± 2025",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 85674,
            "range": "± 14343",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 82283,
            "range": "± 9230",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 94783,
            "range": "± 10795",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 81921,
            "range": "± 1248",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 91558,
            "range": "± 1116",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155797,
            "range": "± 6980",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 255096,
            "range": "± 9102",
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
          "id": "8d17cea2e5f4481ab630bb09cdb8217917a692e8",
          "message": "ci: add TangleGuard architecture linter workflow and ignore RUSTSEC-2026-0221 vulnerability",
          "timestamp": "2026-07-31T14:06:16-03:00",
          "tree_id": "6790ff28d5562025eb794bacb0811815205af4c5",
          "url": "https://github.com/Rullst/Rullst/commit/8d17cea2e5f4481ab630bb09cdb8217917a692e8"
        },
        "date": 1785517844020,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 9,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 37,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 399,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 300,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 150,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2095424,
            "range": "± 579230",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 63030,
            "range": "± 754",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 66067,
            "range": "± 439",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 61825,
            "range": "± 928",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 69623,
            "range": "± 565",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 62775,
            "range": "± 691",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 70260,
            "range": "± 565",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 119247,
            "range": "± 1251",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 201214,
            "range": "± 1243",
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
          "id": "9fc37e2b3ccdf4d63c2609b6441d6199dbf7fa3d",
          "message": "feat: integrate TangleGuard architecture linting via GitHub Actions workflow",
          "timestamp": "2026-07-31T15:56:19-03:00",
          "tree_id": "6db0f69d3a44c5f24b336aee6989161931e00aa9",
          "url": "https://github.com/Rullst/Rullst/commit/9fc37e2b3ccdf4d63c2609b6441d6199dbf7fa3d"
        },
        "date": 1785524472511,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 7,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 40,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 403,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 269,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 168,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 4605244,
            "range": "± 10736884",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 71859,
            "range": "± 3014",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 66684,
            "range": "± 3912",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 61871,
            "range": "± 2560",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 70142,
            "range": "± 4707",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 64437,
            "range": "± 3603",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 72031,
            "range": "± 4393",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 148712,
            "range": "± 6549",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 259171,
            "range": "± 10273",
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
          "id": "b87bb872b01466e5f5be401e32e8d4dc281570a1",
          "message": "feat: add UI components including update checker, interactive spinner, and CLI dashboard",
          "timestamp": "2026-07-31T16:14:11-03:00",
          "tree_id": "699b80b2b65295ab36a8af6847c00202c5910cfd",
          "url": "https://github.com/Rullst/Rullst/commit/b87bb872b01466e5f5be401e32e8d4dc281570a1"
        },
        "date": 1785525512443,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 462,
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
            "value": 198,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2063885,
            "range": "± 66561",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 101109,
            "range": "± 1267",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 105818,
            "range": "± 891",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 99814,
            "range": "± 1965",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 107219,
            "range": "± 1413",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 99785,
            "range": "± 2324",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 107714,
            "range": "± 1262",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 156092,
            "range": "± 3485",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 245973,
            "range": "± 3335",
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
          "id": "209b11bf867a17f8cf1b4723ae4a19babc836ad8",
          "message": "feat: implement hybrid hot-reload server with AST-based logic diffing and TUI dashboard support",
          "timestamp": "2026-07-31T16:39:02-03:00",
          "tree_id": "33725ba0603d757972a766407350eee9021ed1d7",
          "url": "https://github.com/Rullst/Rullst/commit/209b11bf867a17f8cf1b4723ae4a19babc836ad8"
        },
        "date": 1785527017477,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 15,
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
            "value": 486,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 400,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 191,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1527378,
            "range": "± 73206",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 76860,
            "range": "± 1033",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 80955,
            "range": "± 803",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 76789,
            "range": "± 4590",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 86306,
            "range": "± 3426",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 77853,
            "range": "± 2899",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 86385,
            "range": "± 10766",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153961,
            "range": "± 1677",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 259412,
            "range": "± 2793",
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
          "id": "04715f3adf7ea141261a483008061a9e8da5b2a1",
          "message": "chore: update Cargo.lock and ignore RUSTSEC-2026-0221 in OSV scanner configuration",
          "timestamp": "2026-07-31T17:13:19-03:00",
          "tree_id": "5935b5d090831970e12f3c832def812af2f30b98",
          "url": "https://github.com/Rullst/Rullst/commit/04715f3adf7ea141261a483008061a9e8da5b2a1"
        },
        "date": 1785529202980,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 515,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 402,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 191,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1987715,
            "range": "± 110275",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 99668,
            "range": "± 1180",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 104414,
            "range": "± 1530",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 97691,
            "range": "± 1960",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 106393,
            "range": "± 1971",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 96905,
            "range": "± 2085",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 106084,
            "range": "± 1734",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 151498,
            "range": "± 1876",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 247284,
            "range": "± 4285",
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
          "id": "f850e5f7b3e317c0bf72ebe34ef6d127032c1b24",
          "message": "chore: add OSV-scanner configuration to ignore known non-critical or upstream-blocked vulnerabilities",
          "timestamp": "2026-07-31T17:30:22-03:00",
          "tree_id": "60991324e5aca59d885b1eef513ffbece9f87766",
          "url": "https://github.com/Rullst/Rullst/commit/f850e5f7b3e317c0bf72ebe34ef6d127032c1b24"
        },
        "date": 1785530101001,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 13,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 49,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 518,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 399,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 199,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1433751,
            "range": "± 25887",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 76538,
            "range": "± 1070",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 81371,
            "range": "± 703",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 76946,
            "range": "± 1203",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 85146,
            "range": "± 865",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 76870,
            "range": "± 829",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 85602,
            "range": "± 2255",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153550,
            "range": "± 2238",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 258824,
            "range": "± 4578",
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
          "id": "cf5971699b08026b303bf82555fa3ced05b8af71",
          "message": "ci: add OWASP ZAP baseline scan workflow for automated security testing",
          "timestamp": "2026-07-31T18:08:10-03:00",
          "tree_id": "b6c2535e63f4881d1fb246c086fe8b7163e5b9e1",
          "url": "https://github.com/Rullst/Rullst/commit/cf5971699b08026b303bf82555fa3ced05b8af71"
        },
        "date": 1785532369957,
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
            "value": 366,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 192,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1829656,
            "range": "± 72888",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 97774,
            "range": "± 1293",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 102136,
            "range": "± 1077",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 96732,
            "range": "± 2320",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 106238,
            "range": "± 1380",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 95127,
            "range": "± 2289",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 105359,
            "range": "± 1430",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153715,
            "range": "± 2906",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 249662,
            "range": "± 4316",
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
          "id": "88777cbda50bd4cf79e8847235f91d8038729008",
          "message": "feat: add CI workflow for automated OWASP ZAP baseline security scanning",
          "timestamp": "2026-07-31T18:38:45-03:00",
          "tree_id": "d9375b816d591d2f4e988b1862aec71cf9682c63",
          "url": "https://github.com/Rullst/Rullst/commit/88777cbda50bd4cf79e8847235f91d8038729008"
        },
        "date": 1785534198459,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 468,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 372,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 199,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2103557,
            "range": "± 145475",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 101851,
            "range": "± 1241",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 108161,
            "range": "± 3183",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 100383,
            "range": "± 2644",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 108758,
            "range": "± 2441",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 100174,
            "range": "± 2201",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 109482,
            "range": "± 2343",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155118,
            "range": "± 1516",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 248228,
            "range": "± 3429",
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
          "id": "ff9d926a13cefa8b713e2f8b039255cf82d951a4",
          "message": "ci: implement comprehensive GitHub Actions CI and release automation workflows",
          "timestamp": "2026-07-31T18:58:26-03:00",
          "tree_id": "cd790abaabea8ae42961dc906e1c0df714dc8a27",
          "url": "https://github.com/Rullst/Rullst/commit/ff9d926a13cefa8b713e2f8b039255cf82d951a4"
        },
        "date": 1785535374604,
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
            "value": 473,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 373,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 199,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1891514,
            "range": "± 176096",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 90390,
            "range": "± 1053",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 95633,
            "range": "± 1138",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 88294,
            "range": "± 3323",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 97450,
            "range": "± 2407",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 86875,
            "range": "± 2929",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 98386,
            "range": "± 1149",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 141704,
            "range": "± 2825",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 235785,
            "range": "± 3538",
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
          "id": "b5462397a354ee20ca184f9ba7e5d8d345b2ae65",
          "message": "feat: add OWASP ZAP baseline scan workflow for dynamic security testing",
          "timestamp": "2026-07-31T19:10:20-03:00",
          "tree_id": "cf83606c1474d05e2472d93b968e0f29899ea1b2",
          "url": "https://github.com/Rullst/Rullst/commit/b5462397a354ee20ca184f9ba7e5d8d345b2ae65"
        },
        "date": 1785536084965,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 10,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 29,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 316,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 226,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 128,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2617139,
            "range": "± 17244619",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 60011,
            "range": "± 1076",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 63912,
            "range": "± 1383",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 63610,
            "range": "± 2660",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 72493,
            "range": "± 1659",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 63622,
            "range": "± 1253",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 72044,
            "range": "± 1595",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 131423,
            "range": "± 4305",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 221931,
            "range": "± 14181",
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
          "id": "e5b5f48042389ef5fcccc14360bc8939d07a7ae4",
          "message": "feat: implement HTML sanitization utilities and add automated workflows for fuzzing, mutation, and architecture testing.",
          "timestamp": "2026-08-02T02:28:51-03:00",
          "tree_id": "57ba3d4c508561e23192d9dd5b8284d9be1771f0",
          "url": "https://github.com/Rullst/Rullst/commit/e5b5f48042389ef5fcccc14360bc8939d07a7ae4"
        },
        "date": 1785648806531,
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
            "value": 467,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 374,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 199,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2322658,
            "range": "± 283545",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 121708,
            "range": "± 11853",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 110563,
            "range": "± 10441",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 103672,
            "range": "± 7341",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 110516,
            "range": "± 7425",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 111728,
            "range": "± 9570",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 120395,
            "range": "± 9667",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 164689,
            "range": "± 17546",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 297981,
            "range": "± 35298",
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
          "id": "f3597045da8503c8ad71cecc7c765d9391d034a0",
          "message": "ci: add manual workflow triggers for mutation testing, fuzzing, Kani verification, and Miri UB detection",
          "timestamp": "2026-08-02T02:40:51-03:00",
          "tree_id": "152d7e40ea5b82bfcbdb614d3f623d330e50e1e7",
          "url": "https://github.com/Rullst/Rullst/commit/f3597045da8503c8ad71cecc7c765d9391d034a0"
        },
        "date": 1785649518151,
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
            "value": 482,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 367,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 191,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2621900,
            "range": "± 315583",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 92435,
            "range": "± 1518",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 95301,
            "range": "± 1351",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 90539,
            "range": "± 3150",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 99473,
            "range": "± 3322",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 91171,
            "range": "± 2470",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 100144,
            "range": "± 2293",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 141406,
            "range": "± 3542",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 243501,
            "range": "± 3385",
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
          "id": "e4ffeb159587a5259d45f868c116c37e268b0879",
          "message": "feat: implement Rullst Live component system and project inspection tooling",
          "timestamp": "2026-08-02T12:11:29-03:00",
          "tree_id": "b15d22b28c1ffcfb3fc4809ee2d2ad43b31c7077",
          "url": "https://github.com/Rullst/Rullst/commit/e4ffeb159587a5259d45f868c116c37e268b0879"
        },
        "date": 1785683755267,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 463,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 369,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 195,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1807460,
            "range": "± 64977",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 98381,
            "range": "± 1094",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 102988,
            "range": "± 983",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 97550,
            "range": "± 1879",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 104772,
            "range": "± 1413",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 95177,
            "range": "± 2033",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 105638,
            "range": "± 1574",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 151624,
            "range": "± 1124",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 249167,
            "range": "± 7029",
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
          "id": "100ce2d6e618b42aba40d27c118d4f88b17fd6c5",
          "message": "feat: add CLI inspection tools, define package extension interface, and expand project documentation",
          "timestamp": "2026-08-02T12:29:20-03:00",
          "tree_id": "e75adec43bcb7c0f0e2ca1aca23e940886acb012",
          "url": "https://github.com/Rullst/Rullst/commit/100ce2d6e618b42aba40d27c118d4f88b17fd6c5"
        },
        "date": 1785684826999,
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
            "value": 479,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 365,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 192,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2012720,
            "range": "± 108942",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 97643,
            "range": "± 1405",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 100902,
            "range": "± 787",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 96236,
            "range": "± 2161",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 104262,
            "range": "± 1521",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 94600,
            "range": "± 3308",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 104560,
            "range": "± 2658",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 151651,
            "range": "± 2407",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 244824,
            "range": "± 2300",
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
          "id": "84a46d8ab7311ba7104fca60f82a24775914a444",
          "message": "feat: add AuthCallback extractor with CSRF verification and integrate matrix test suites for MySQL and Postgres",
          "timestamp": "2026-08-02T13:16:22-03:00",
          "tree_id": "9c2882769c27317e23c35414b55ea5d6afe873b2",
          "url": "https://github.com/Rullst/Rullst/commit/84a46d8ab7311ba7104fca60f82a24775914a444"
        },
        "date": 1785687675826,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 474,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 369,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 190,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2069935,
            "range": "± 134737",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 100498,
            "range": "± 1891",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 104715,
            "range": "± 1664",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 99724,
            "range": "± 1897",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 108583,
            "range": "± 2165",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 98055,
            "range": "± 2072",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 106734,
            "range": "± 1560",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153465,
            "range": "± 4189",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 250977,
            "range": "± 4651",
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
          "id": "2e43d594b28bb4b06153e80b1a2adff4f9fabe41",
          "message": "feat: add inspection tool for project routes, models, and schema definitions",
          "timestamp": "2026-08-02T20:04:39-03:00",
          "tree_id": "b15a2f235fc193dd71f0efa4dc680a6bd08a8fa7",
          "url": "https://github.com/Rullst/Rullst/commit/2e43d594b28bb4b06153e80b1a2adff4f9fabe41"
        },
        "date": 1785712471121,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 473,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 378,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 190,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2125486,
            "range": "± 106392",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 98052,
            "range": "± 1995",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 103439,
            "range": "± 1212",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 96364,
            "range": "± 2048",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 104391,
            "range": "± 3042",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 93478,
            "range": "± 3165",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 104514,
            "range": "± 1272",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 151461,
            "range": "± 1313",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 244223,
            "range": "± 2223",
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
          "id": "223b6357c38e0ac737e51aee4879eec9af6dcfd9",
          "message": "feat: implement dynamic database table browser with schema-agnostic querying and automated GitHub scorecard analysis.",
          "timestamp": "2026-08-02T20:33:39-03:00",
          "tree_id": "a5e848641b5019cc32623bc23b564cba8a891ef5",
          "url": "https://github.com/Rullst/Rullst/commit/223b6357c38e0ac737e51aee4879eec9af6dcfd9"
        },
        "date": 1785713923598,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 474,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 372,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 190,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1947076,
            "range": "± 85031",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 101044,
            "range": "± 2068",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 106051,
            "range": "± 1895",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 98081,
            "range": "± 2838",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 106001,
            "range": "± 2268",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 99436,
            "range": "± 2329",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 107297,
            "range": "± 2664",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154374,
            "range": "± 2985",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 249681,
            "range": "± 2284",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "4fb11e9b7a3cbf13a13efcb299e92dbb99471c0d",
          "message": "chore: update GitHub Actions dependencies and refactor artisan studio export and db imports",
          "timestamp": "2026-08-03T11:51:28-03:00",
          "tree_id": "b047f6f984e4822bb1c3b5c13c20cd626639473b",
          "url": "https://github.com/Rullst/Rullst/commit/4fb11e9b7a3cbf13a13efcb299e92dbb99471c0d"
        },
        "date": 1785768994476,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 467,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 375,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 190,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1828584,
            "range": "± 85127",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 99242,
            "range": "± 1597",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 103758,
            "range": "± 1400",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 96949,
            "range": "± 2327",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 104980,
            "range": "± 2517",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 95211,
            "range": "± 2622",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 105467,
            "range": "± 3979",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153960,
            "range": "± 2468",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 249832,
            "range": "± 2675",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "a474a7c20d6792cd0efd0ba6be6ab1ed9374d532",
          "message": "refactor: cleanup code formatting and simplify environment variable retrieval logic",
          "timestamp": "2026-08-03T12:20:37-03:00",
          "tree_id": "2ef7cde966adb15e90023b53c1ccbcf9f6eae223",
          "url": "https://github.com/Rullst/Rullst/commit/a474a7c20d6792cd0efd0ba6be6ab1ed9374d532"
        },
        "date": 1785771618876,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 13,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 15,
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
            "value": 493,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 400,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 193,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1495311,
            "range": "± 76531",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 76184,
            "range": "± 1116",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 80968,
            "range": "± 1524",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 76909,
            "range": "± 919",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 85394,
            "range": "± 1187",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 76606,
            "range": "± 1078",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 86231,
            "range": "± 1168",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153960,
            "range": "± 5036",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 257894,
            "range": "± 9790",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "36833cbb8c4e0218de8b9d9ad11dff828e7e4a61",
          "message": "feat: add comprehensive documentation suite and implement hot-reload UI triggers for logic and template changes.",
          "timestamp": "2026-08-03T22:31:55-03:00",
          "tree_id": "e4ef6a2ecac07c7deba1ef49e6f66fe08d23243b",
          "url": "https://github.com/Rullst/Rullst/commit/36833cbb8c4e0218de8b9d9ad11dff828e7e4a61"
        },
        "date": 1785807419088,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 471,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 358,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 190,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1789193,
            "range": "± 87485",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 97596,
            "range": "± 2070",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 103537,
            "range": "± 2496",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 97284,
            "range": "± 2699",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105198,
            "range": "± 1786",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 95461,
            "range": "± 3391",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 105097,
            "range": "± 2344",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155213,
            "range": "± 2366",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 248511,
            "range": "± 1759",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "b7140b7e339b68c1857a177b9e9771cbe129f21f",
          "message": "refactor: apply consistent code formatting and style improvements across nexus, orm, and studio modules",
          "timestamp": "2026-08-03T23:55:23-03:00",
          "tree_id": "8ab4d7700f816fa28072773137a99bac0c019e92",
          "url": "https://github.com/Rullst/Rullst/commit/b7140b7e339b68c1857a177b9e9771cbe129f21f"
        },
        "date": 1785812425250,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 15,
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
            "value": 534,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 381,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 194,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1544778,
            "range": "± 91676",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 79710,
            "range": "± 1026",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 84783,
            "range": "± 2043",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 79639,
            "range": "± 9566",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 88577,
            "range": "± 743",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 80267,
            "range": "± 844",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 89257,
            "range": "± 720",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 156317,
            "range": "± 1724",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 261743,
            "range": "± 2158",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "404fcb9b3fa804dbdcef3d7da0f71190335bb128",
          "message": "feat: expand roadmap with IoT, Aerospace, and enhanced Security/Quantum milestones",
          "timestamp": "2026-08-04T00:10:22-03:00",
          "tree_id": "27b3df669941733ffe7aaf58af2d251a54c132f1",
          "url": "https://github.com/Rullst/Rullst/commit/404fcb9b3fa804dbdcef3d7da0f71190335bb128"
        },
        "date": 1785813316877,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 472,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 375,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 189,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1933846,
            "range": "± 163318",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 96636,
            "range": "± 974",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 102325,
            "range": "± 1153",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 96572,
            "range": "± 1844",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 104232,
            "range": "± 1131",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 92998,
            "range": "± 2717",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 104847,
            "range": "± 1233",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 150827,
            "range": "± 1163",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 241862,
            "range": "± 3466",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "e2a4d1373fff4f4d3268a02d8899c0ca7868c49b",
          "message": "feat: implement rullst-iot and rullst-security crates with hardware-focused modules and CI/CD pipelines",
          "timestamp": "2026-08-04T15:04:05-03:00",
          "tree_id": "22c09cd1efb8fc18ec14f7248365a2135032ac79",
          "url": "https://github.com/Rullst/Rullst/commit/e2a4d1373fff4f4d3268a02d8899c0ca7868c49b"
        },
        "date": 1785867002202,
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
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 470,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 365,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 204,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1872381,
            "range": "± 99734",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 96965,
            "range": "± 1301",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 102060,
            "range": "± 1610",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 96355,
            "range": "± 4283",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 104157,
            "range": "± 2964",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 95059,
            "range": "± 2512",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 104530,
            "range": "± 1935",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 149434,
            "range": "± 1578",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 244727,
            "range": "± 7653",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "aaf7f370fc953cfcba41f04dde1e2decdf5b1728",
          "message": "style: apply consistent rustfmt code formatting across the codebase and reorder crate modules",
          "timestamp": "2026-08-04T15:51:27-03:00",
          "tree_id": "dfe1617f4c9d4a595e74b03af024a01952519742",
          "url": "https://github.com/Rullst/Rullst/commit/aaf7f370fc953cfcba41f04dde1e2decdf5b1728"
        },
        "date": 1785869787693,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 469,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 359,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 193,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2013650,
            "range": "± 80081",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 103066,
            "range": "± 1320",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 107064,
            "range": "± 930",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 101693,
            "range": "± 2496",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 108393,
            "range": "± 1134",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 100100,
            "range": "± 2465",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 109616,
            "range": "± 1132",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155058,
            "range": "± 1726",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 253140,
            "range": "± 3892",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "c439130254f3a4dc78d81e6c144fe32c3f3548d1",
          "message": "refactor: optimize I2C frame buffer allocation using resize instead of manual loop",
          "timestamp": "2026-08-04T15:53:21-03:00",
          "tree_id": "f87423ea5bf17c187e48eff7a45aaa1d5425964d",
          "url": "https://github.com/Rullst/Rullst/commit/c439130254f3a4dc78d81e6c144fe32c3f3548d1"
        },
        "date": 1785869929799,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 9,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 41,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 430,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 329,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 174,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1492408,
            "range": "± 65370",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 69474,
            "range": "± 1545",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 73838,
            "range": "± 1430",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 67519,
            "range": "± 1125",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 79312,
            "range": "± 4330",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 69166,
            "range": "± 1199",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 79807,
            "range": "± 1722",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153218,
            "range": "± 4094",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 265592,
            "range": "± 9664",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "f3e8ae62e1da83078163ba65b39558d0fc65498d",
          "message": "chore: pin GitHub Actions, update dependencies, ignore new vulnerability, and perform maintenance cleanups",
          "timestamp": "2026-08-04T16:15:30-03:00",
          "tree_id": "d33543cf998102ca06a9834bbea7fd428c43112b",
          "url": "https://github.com/Rullst/Rullst/commit/f3e8ae62e1da83078163ba65b39558d0fc65498d"
        },
        "date": 1785871237165,
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
            "value": 458,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 361,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 189,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1975427,
            "range": "± 171752",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 101557,
            "range": "± 1327",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 105817,
            "range": "± 1171",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 100242,
            "range": "± 1829",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 109030,
            "range": "± 1863",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 100755,
            "range": "± 1984",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 108668,
            "range": "± 1521",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154600,
            "range": "± 4834",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 252051,
            "range": "± 4226",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "7004aecbf98363bc245a29379c68e91cb97a925e",
          "message": "feat: implement major framework expansion with k8s/Paas deployment, gRPC support, dependency injection, Rullst Radar telemetry, and comprehensive documentation.",
          "timestamp": "2026-08-05T17:00:39-03:00",
          "tree_id": "36ed9ceb03d69ed3a2b2f6c177bfa8511265da89",
          "url": "https://github.com/Rullst/Rullst/commit/7004aecbf98363bc245a29379c68e91cb97a925e"
        },
        "date": 1785960338103,
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
            "value": 52,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 463,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 360,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 190,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1797939,
            "range": "± 70260",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 96514,
            "range": "± 1604",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 100343,
            "range": "± 1576",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 95729,
            "range": "± 1830",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 103665,
            "range": "± 1528",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 94085,
            "range": "± 3100",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 103462,
            "range": "± 2369",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 149884,
            "range": "± 2710",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 243696,
            "range": "± 2559",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "eb0bb605c7635c1f5095d4ba47c431091c248079",
          "message": "chore: reorganize roadmap into pillar-based milestones with target releases",
          "timestamp": "2026-08-05T17:32:31-03:00",
          "tree_id": "1758c244218b0aae799cfe030be51df9fb13db74",
          "url": "https://github.com/Rullst/Rullst/commit/eb0bb605c7635c1f5095d4ba47c431091c248079"
        },
        "date": 1785962241708,
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
            "value": 471,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 360,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 190,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2003587,
            "range": "± 102111",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 102187,
            "range": "± 2159",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 107435,
            "range": "± 1255",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 101784,
            "range": "± 1872",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 109008,
            "range": "± 2373",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 101114,
            "range": "± 1870",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 109926,
            "range": "± 1801",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 156944,
            "range": "± 2252",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 249193,
            "range": "± 2746",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "0083a3d6694138a34319981db4a7925a81d96e35",
          "message": "refactor: reformat codebase style and consolidate module declarations",
          "timestamp": "2026-08-05T19:04:00-03:00",
          "tree_id": "ff7a2617c66994a2144572153dc0ffe83d77e374",
          "url": "https://github.com/Rullst/Rullst/commit/0083a3d6694138a34319981db4a7925a81d96e35"
        },
        "date": 1785967749105,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 15,
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
            "value": 523,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 379,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 195,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1430988,
            "range": "± 69493",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 75445,
            "range": "± 937",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 80414,
            "range": "± 807",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 76933,
            "range": "± 861",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 84230,
            "range": "± 563",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 75865,
            "range": "± 821",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 84673,
            "range": "± 792",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153078,
            "range": "± 1450",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 257229,
            "range": "± 3145",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "c73d7fa3106142f71130c26dc637ed46935ae1bc",
          "message": "feat: implement nexus ai chat system with schema-aware query generation and llm configuration guide",
          "timestamp": "2026-08-06T02:17:42-03:00",
          "tree_id": "7e8a5714c00359ce9a2ccd626e61231c1324268c",
          "url": "https://github.com/Rullst/Rullst/commit/c73d7fa3106142f71130c26dc637ed46935ae1bc"
        },
        "date": 1785993821149,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 46,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 492,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 393,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 195,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1467926,
            "range": "± 64717",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 74876,
            "range": "± 1131",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 80469,
            "range": "± 734",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 76438,
            "range": "± 725",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 84605,
            "range": "± 1365",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 76889,
            "range": "± 993",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 85902,
            "range": "± 1539",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 151572,
            "range": "± 1715",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 258675,
            "range": "± 2992",
            "unit": "ns/iter"
          }
        ]
      },
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
        "date": 1786076400997,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 473,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 368,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 189,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1928667,
            "range": "± 113559",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 98317,
            "range": "± 1573",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 103410,
            "range": "± 822",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 97694,
            "range": "± 2106",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 104140,
            "range": "± 1212",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 94837,
            "range": "± 2687",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 104453,
            "range": "± 2781",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 150294,
            "range": "± 2175",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 240837,
            "range": "± 2194",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "6e5a80038c93e05af0071dd0593ba1e1b6a21129",
          "message": "feat: add comprehensive security modules including MFA, SIEM, and rate limiting while updating Studio UI assets",
          "timestamp": "2026-08-07T15:48:21-03:00",
          "tree_id": "0687c289b85e6fc0327bae751212c315602025e3",
          "url": "https://github.com/Rullst/Rullst/commit/6e5a80038c93e05af0071dd0593ba1e1b6a21129"
        },
        "date": 1786128796431,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 9,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 30,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 319,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 228,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 127,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 4024676,
            "range": "± 10665752",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 63602,
            "range": "± 2395",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 62803,
            "range": "± 2420",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 63306,
            "range": "± 1452",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 71088,
            "range": "± 2014",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 61414,
            "range": "± 1582",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 69332,
            "range": "± 1345",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 129782,
            "range": "± 6639",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 224301,
            "range": "± 16013",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "2cbc70c57c390d4e8120869134d0b31be102dbbe",
          "message": "feat: add multi-factor authentication, security auditing, and enhanced threat mitigation middleware with dashboard tracking",
          "timestamp": "2026-08-07T16:34:05-03:00",
          "tree_id": "452620a622f1445ff98131090074ad81df01cf72",
          "url": "https://github.com/Rullst/Rullst/commit/2cbc70c57c390d4e8120869134d0b31be102dbbe"
        },
        "date": 1786131551450,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 15,
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
            "value": 512,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 399,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 195,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1625164,
            "range": "± 93773",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 75892,
            "range": "± 1323",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 78885,
            "range": "± 1419",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 75812,
            "range": "± 1201",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 84004,
            "range": "± 987",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 76009,
            "range": "± 826",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 84345,
            "range": "± 1225",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 151683,
            "range": "± 2782",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 252356,
            "range": "± 2472",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "e48b9f1494ff432b57ab9b6e7e5a95c4aad07e78",
          "message": "refactor: apply codebase-wide formatting changes to improve readability and style compliance",
          "timestamp": "2026-08-07T20:15:17-03:00",
          "tree_id": "574a0f0814a4aabd8df1fb666916ce3bd7f9a812",
          "url": "https://github.com/Rullst/Rullst/commit/e48b9f1494ff432b57ab9b6e7e5a95c4aad07e78"
        },
        "date": 1786144820472,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 9,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 36,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 383,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 300,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 157,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1808850,
            "range": "± 140605",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 59453,
            "range": "± 738",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 63271,
            "range": "± 586",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 60059,
            "range": "± 825",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 66277,
            "range": "± 646",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 60074,
            "range": "± 756",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 66742,
            "range": "± 529",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 119290,
            "range": "± 2424",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 202024,
            "range": "± 1902",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "fe57d1eace8910d16230fd769ffbee90b2f6f97a",
          "message": "fix(ci): resolve zero-panics lints, fix pages build, update typos and osv exemptions",
          "timestamp": "2026-08-07T20:57:30-03:00",
          "tree_id": "99734da3472262452083a420a5308f605aa9d971",
          "url": "https://github.com/Rullst/Rullst/commit/fe57d1eace8910d16230fd769ffbee90b2f6f97a"
        },
        "date": 1786147457163,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 465,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 364,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 189,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1923204,
            "range": "± 85310",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 101630,
            "range": "± 1105",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 106867,
            "range": "± 1335",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 100749,
            "range": "± 3240",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 108162,
            "range": "± 2638",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 99302,
            "range": "± 1792",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 108548,
            "range": "± 1087",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 156041,
            "range": "± 1471",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 252889,
            "range": "± 5498",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "34ee50f1290b5064a2ce8b1d097c4ff543fa3713",
          "message": "fix(clippy): allow too-many-arguments on generator functions and clean legacy website",
          "timestamp": "2026-08-07T21:32:10-03:00",
          "tree_id": "ca61f9a0fa752be4e1c3e150d72b8b833a8442ea",
          "url": "https://github.com/Rullst/Rullst/commit/34ee50f1290b5064a2ce8b1d097c4ff543fa3713"
        },
        "date": 1786149448455,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 9,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 13,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 41,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 451,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 338,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 175,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1643789,
            "range": "± 169187",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 69168,
            "range": "± 3405",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 74704,
            "range": "± 7086",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 66297,
            "range": "± 9809",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 77320,
            "range": "± 2750",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 70574,
            "range": "± 15167",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 79456,
            "range": "± 2887",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 147975,
            "range": "± 5874",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 260587,
            "range": "± 12632",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "32e1f244542c8d4f5200184d05b9fbed333e5f64",
          "message": "fix(site): update navbar buttons, add interactive benches anchor, wire full footer links, and fix 404 bench routes",
          "timestamp": "2026-08-07T22:11:22-03:00",
          "tree_id": "42058c6e6105d02d6e244e7ab7763a5cf61e5b9e",
          "url": "https://github.com/Rullst/Rullst/commit/32e1f244542c8d4f5200184d05b9fbed333e5f64"
        },
        "date": 1786151783882,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 9,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 36,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 413,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 306,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 163,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1823687,
            "range": "± 688204",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 59107,
            "range": "± 1087",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 62388,
            "range": "± 520",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 59491,
            "range": "± 859",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 65107,
            "range": "± 509",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 58767,
            "range": "± 2305",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 65567,
            "range": "± 1009",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 118895,
            "range": "± 7639",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 198327,
            "range": "± 1699",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "28bbbf9a2738bf0d655fd753c84224dc8031d1c7",
          "message": "refactor: unindent test module and remove incomplete function body in tests.rs",
          "timestamp": "2026-08-07T22:46:53-03:00",
          "tree_id": "0ed3592e4c105444257691198eed0995225714e8",
          "url": "https://github.com/Rullst/Rullst/commit/28bbbf9a2738bf0d655fd753c84224dc8031d1c7"
        },
        "date": 1786153920079,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 474,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 379,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 190,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1998791,
            "range": "± 103800",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 103214,
            "range": "± 4800",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 107785,
            "range": "± 1017",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 101307,
            "range": "± 2545",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 109571,
            "range": "± 1627",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 100253,
            "range": "± 2327",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 109757,
            "range": "± 2009",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155507,
            "range": "± 1041",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 250584,
            "range": "± 2945",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "33a17298ccfe0e2878283af1bb5a89141b95e6c4",
          "message": "docs: add link to dedicated benchmark repository in README",
          "timestamp": "2026-08-07T22:59:10-03:00",
          "tree_id": "404278cd822e2a65fdfca520d8e64b38f70f8a34",
          "url": "https://github.com/Rullst/Rullst/commit/33a17298ccfe0e2878283af1bb5a89141b95e6c4"
        },
        "date": 1786154686295,
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
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 460,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 363,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 196,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1891755,
            "range": "± 135474",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 97674,
            "range": "± 991",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 102102,
            "range": "± 1814",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 96420,
            "range": "± 2360",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 103471,
            "range": "± 1250",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 92844,
            "range": "± 2408",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 104124,
            "range": "± 2672",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 151305,
            "range": "± 5058",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 243200,
            "range": "± 5980",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "38b19b3bc6361e2262a2d9567e6ca1e5469950b0",
          "message": "docs: remove hardcoded microsecond numbers and point directly to live interactive benchmarks",
          "timestamp": "2026-08-07T23:08:50-03:00",
          "tree_id": "e30b75743691d0fbb12615787337008abd19a45a",
          "url": "https://github.com/Rullst/Rullst/commit/38b19b3bc6361e2262a2d9567e6ca1e5469950b0"
        },
        "date": 1786155383399,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 7,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 13,
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
            "value": 441,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 309,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 191,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2541208,
            "range": "± 81369",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 77907,
            "range": "± 1501",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 70774,
            "range": "± 1306",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 65978,
            "range": "± 2944",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 76188,
            "range": "± 2422",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 67715,
            "range": "± 1350",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 76320,
            "range": "± 2657",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155840,
            "range": "± 8047",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 280812,
            "range": "± 12745",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "346851fa7f8a665d043d9036f076c8dc9c58f32a",
          "message": "refactor: implement CSRF exemption list, update ZAP suppression rules, and harden PQC compliance pipeline",
          "timestamp": "2026-08-08T00:41:09-03:00",
          "tree_id": "045d627448f3ac4e0f51b8f699c92f354545ea16",
          "url": "https://github.com/Rullst/Rullst/commit/346851fa7f8a665d043d9036f076c8dc9c58f32a"
        },
        "date": 1786160803076,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 7,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 13,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 45,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 449,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 316,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 193,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 3463903,
            "range": "± 140110421",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 66958,
            "range": "± 1116",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 72198,
            "range": "± 1243",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 67905,
            "range": "± 1392",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 78089,
            "range": "± 1577",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 69360,
            "range": "± 1499",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 79079,
            "range": "± 2521",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155481,
            "range": "± 5271",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 276488,
            "range": "± 8960",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "79915f65c53ec9d1e2df2e57f07cb0213714335a",
          "message": "more tests workflows ci/cd",
          "timestamp": "2026-08-08T02:55:34-03:00",
          "tree_id": "861995cff6daaca59408bc62a2d959318e60686f",
          "url": "https://github.com/Rullst/Rullst/commit/79915f65c53ec9d1e2df2e57f07cb0213714335a"
        },
        "date": 1786168847137,
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
            "value": 466,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 368,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 194,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1978587,
            "range": "± 90910",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 102182,
            "range": "± 1328",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 107107,
            "range": "± 2196",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 99691,
            "range": "± 3296",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 108842,
            "range": "± 1491",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 99734,
            "range": "± 2556",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 108912,
            "range": "± 1355",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153992,
            "range": "± 1336",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 246081,
            "range": "± 2136",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "80c5731a7f7409ec7c4c7d46d7c2509d551fcc78",
          "message": "refactor: consolidate failure entry update logic in login_guard to return current count correctly",
          "timestamp": "2026-08-09T00:09:08-03:00",
          "tree_id": "53b194905a4a8818861239047f31655ca8988180",
          "url": "https://github.com/Rullst/Rullst/commit/80c5731a7f7409ec7c4c7d46d7c2509d551fcc78"
        },
        "date": 1786245248354,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 479,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 368,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 188,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1841364,
            "range": "± 63971",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 100112,
            "range": "± 1485",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 105119,
            "range": "± 949",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 98350,
            "range": "± 2241",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 106720,
            "range": "± 2660",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 97991,
            "range": "± 2404",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 106653,
            "range": "± 1272",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153134,
            "range": "± 1364",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 246582,
            "range": "± 2393",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "6bf14ffb7fdc095e4e5a89ec3908d030b2202203",
          "message": "refactor: improve code formatting, rename header configuration variables, and remove trailing whitespace across the codebase.",
          "timestamp": "2026-08-09T00:29:19-03:00",
          "tree_id": "8be2ccd69c76dc86a8880f81a487acaf443c629f",
          "url": "https://github.com/Rullst/Rullst/commit/6bf14ffb7fdc095e4e5a89ec3908d030b2202203"
        },
        "date": 1786246459705,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 467,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 366,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 192,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2402342,
            "range": "± 225297",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 91878,
            "range": "± 2670",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 96700,
            "range": "± 1603",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 89264,
            "range": "± 3501",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 99807,
            "range": "± 2824",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 90473,
            "range": "± 2737",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 100267,
            "range": "± 1881",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 142817,
            "range": "± 1637",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 230367,
            "range": "± 3016",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}