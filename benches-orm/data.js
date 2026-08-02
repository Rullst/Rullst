window.BENCHMARK_DATA = {
  "lastUpdate": 1785683755771,
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
      }
    ]
  }
}