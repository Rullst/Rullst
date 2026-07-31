window.BENCHMARK_DATA = {
  "lastUpdate": 1785524473200,
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
      }
    ]
  }
}