window.BENCHMARK_DATA = {
  "lastUpdate": 1788536663103,
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
          "id": "299b192290af26d772d4d605eae07171fc57a099",
          "message": "style: apply line wrapping and formatting improvements to documentation templates and markdown files",
          "timestamp": "2026-08-11T22:22:51-03:00",
          "tree_id": "1cc1d64006622b16e57dd996fb69273277df14b9",
          "url": "https://github.com/Rullst/Rullst/commit/299b192290af26d772d4d605eae07171fc57a099"
        },
        "date": 1786498132578,
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
            "value": 459,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 364,
            "range": "± 3",
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
            "value": 2033843,
            "range": "± 90855",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 101287,
            "range": "± 877",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 106191,
            "range": "± 1436",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 100429,
            "range": "± 3360",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 107199,
            "range": "± 1851",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 99093,
            "range": "± 1915",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 108433,
            "range": "± 3361",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 152120,
            "range": "± 1649",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 248527,
            "range": "± 2320",
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
          "id": "bc331fea2a60a3086d4393541223738599e8be81",
          "message": "feat: implement native Brazilian NFS-e issuance module with A1 certificate signing and direct SEFAZ transmission",
          "timestamp": "2026-08-11T23:09:53-03:00",
          "tree_id": "4a808b2e3647d7d84b0f841660efc16ffdb6ccd8",
          "url": "https://github.com/Rullst/Rullst/commit/bc331fea2a60a3086d4393541223738599e8be81"
        },
        "date": 1786500895553,
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
            "value": 426,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 315,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 150,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2339705,
            "range": "± 1454587",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 59686,
            "range": "± 2323",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 62946,
            "range": "± 637",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 60778,
            "range": "± 1716",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 67453,
            "range": "± 2694",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 61191,
            "range": "± 2721",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 67449,
            "range": "± 3729",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 119185,
            "range": "± 1892",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 198850,
            "range": "± 2301",
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
          "id": "603713a8549288649ce683acd15003378140a2c5",
          "message": "feat: expand blog example into a comprehensive Rullst integration testbed with new security, AI, billing, and repository demo modules",
          "timestamp": "2026-08-12T12:03:14-03:00",
          "tree_id": "ac4df1d2d97dd608874c3739feff5fbef736a4c4",
          "url": "https://github.com/Rullst/Rullst/commit/603713a8549288649ce683acd15003378140a2c5"
        },
        "date": 1786547299215,
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
            "value": 412,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 298,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 156,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1901938,
            "range": "± 3927167",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 63656,
            "range": "± 1161",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 66848,
            "range": "± 509",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 62356,
            "range": "± 733",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 70025,
            "range": "± 572",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 62884,
            "range": "± 1367",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 70668,
            "range": "± 412",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 131741,
            "range": "± 1406",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 227343,
            "range": "± 3361",
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
          "id": "2d8255f9e6490e85dca531e87747837d83743106",
          "message": "test: remove memory database initialization from server tests and disable studio/nexus tests under strict database features",
          "timestamp": "2026-08-12T14:52:23-03:00",
          "tree_id": "a98f524a8f547934a0a76dd58c68509bd6c86975",
          "url": "https://github.com/Rullst/Rullst/commit/2d8255f9e6490e85dca531e87747837d83743106"
        },
        "date": 1786557438280,
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
            "value": 51,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 462,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 374,
            "range": "± 5",
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
            "value": 1932547,
            "range": "± 556097",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 98747,
            "range": "± 1373",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 103085,
            "range": "± 2614",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 97051,
            "range": "± 2958",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105274,
            "range": "± 3300",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 95203,
            "range": "± 3757",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 105535,
            "range": "± 936",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 152576,
            "range": "± 1286",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 245200,
            "range": "± 2889",
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
          "id": "91c107f03a578c22aaa9c9a370ea9ee41582d74f",
          "message": "style: reformat code for consistency using multi-line style throughout blog examples",
          "timestamp": "2026-08-12T15:11:02-03:00",
          "tree_id": "a5fb6e605c46262e266e99e0b1dc97c808a4e005",
          "url": "https://github.com/Rullst/Rullst/commit/91c107f03a578c22aaa9c9a370ea9ee41582d74f"
        },
        "date": 1786558573199,
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
            "value": 51,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 476,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 406,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 197,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1897987,
            "range": "± 72631",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 100706,
            "range": "± 3249",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 106207,
            "range": "± 1639",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 99095,
            "range": "± 2024",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 108371,
            "range": "± 2280",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 99510,
            "range": "± 1734",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 107721,
            "range": "± 1886",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155049,
            "range": "± 1356",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 245230,
            "range": "± 3381",
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
          "id": "de3970bd14189258fbc6154ba5894411adbb3d20",
          "message": "chore: update CI toolchain configurations, project brand name, and swagger-ui dependencies",
          "timestamp": "2026-08-12T15:40:23-03:00",
          "tree_id": "38b2c0e3ffe466325cda4963cba921e786d48e73",
          "url": "https://github.com/Rullst/Rullst/commit/de3970bd14189258fbc6154ba5894411adbb3d20"
        },
        "date": 1786575695881,
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
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 362,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 218,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2117644,
            "range": "± 105999",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 98990,
            "range": "± 878",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 103912,
            "range": "± 1345",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 97845,
            "range": "± 2865",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 106030,
            "range": "± 2580",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 96731,
            "range": "± 4436",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 105871,
            "range": "± 2199",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153667,
            "range": "± 1739",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 247919,
            "range": "± 2318",
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
          "id": "ff0282f330397a21fb718a4d87654ccef38745ab",
          "message": "ci: migrate to setup-rust-toolchain, update toolchain components, and include missing packages in zero-panic checks",
          "timestamp": "2026-08-13T09:37:06-03:00",
          "tree_id": "d1c0bcc61e2ec2f5653abfff1f989e80db70bc81",
          "url": "https://github.com/Rullst/Rullst/commit/ff0282f330397a21fb718a4d87654ccef38745ab"
        },
        "date": 1786624943806,
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
            "value": 462,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 367,
            "range": "± 7",
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
            "value": 1979005,
            "range": "± 164658",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 98594,
            "range": "± 1176",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 103804,
            "range": "± 2043",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 97191,
            "range": "± 2693",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 104717,
            "range": "± 1943",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 94268,
            "range": "± 2401",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 104735,
            "range": "± 1654",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153435,
            "range": "± 1868",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 248642,
            "range": "± 4101",
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
          "id": "d512c91c085dd1e2d5089a6aa2d5c5c619db576b",
          "message": "chore: standardize package metadata across all crates and refactor studio routing for readability",
          "timestamp": "2026-08-13T16:54:09-03:00",
          "tree_id": "0f3f9bebed17a5ef6288a3f72161d5e2c1e4d090",
          "url": "https://github.com/Rullst/Rullst/commit/d512c91c085dd1e2d5089a6aa2d5c5c619db576b"
        },
        "date": 1786651157464,
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
            "range": "± 2",
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
            "value": 365,
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
            "value": 1990041,
            "range": "± 108090",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 99894,
            "range": "± 1361",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 105555,
            "range": "± 1281",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 98304,
            "range": "± 2005",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105791,
            "range": "± 1543",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 97058,
            "range": "± 1976",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 107302,
            "range": "± 1971",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154026,
            "range": "± 1358",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 248086,
            "range": "± 7896",
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
          "id": "b58e00e296bd9e9b03ee5b93dba68049e4363d4b",
          "message": "feat: add Alipay billing provider and modularize billing demo examples",
          "timestamp": "2026-08-15T12:43:47-03:00",
          "tree_id": "af45eae3e94b9ff7b2d61c5d74c4d42e164f1872",
          "url": "https://github.com/Rullst/Rullst/commit/b58e00e296bd9e9b03ee5b93dba68049e4363d4b"
        },
        "date": 1786808959274,
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
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 318,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 216,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 124,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 5648278,
            "range": "± 9313415",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 57703,
            "range": "± 1185",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 61573,
            "range": "± 1630",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 57723,
            "range": "± 1974",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 68511,
            "range": "± 1653",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 59844,
            "range": "± 1597",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 72114,
            "range": "± 4206",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 125228,
            "range": "± 6896",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 219532,
            "range": "± 26691",
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
          "id": "6e25d3d87c7c4a39b2defa3cdf0f1fce74cfe9a1",
          "message": "refactor: remove unused Bytes import from ai_firewall middleware",
          "timestamp": "2026-08-15T13:51:35-03:00",
          "tree_id": "e369028c2cd260bdca86b793d5ac96865a523abd",
          "url": "https://github.com/Rullst/Rullst/commit/6e25d3d87c7c4a39b2defa3cdf0f1fce74cfe9a1"
        },
        "date": 1786813020755,
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
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 435,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 333,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 176,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1460011,
            "range": "± 71475",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 68663,
            "range": "± 1534",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 72946,
            "range": "± 2016",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 66122,
            "range": "± 2684",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 77831,
            "range": "± 2232",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 67221,
            "range": "± 1028",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 79944,
            "range": "± 3466",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 148563,
            "range": "± 4524",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 261595,
            "range": "± 6005",
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
          "id": "10dce6749335e9c0456c20ec6a5710d203ecde8f",
          "message": "feat: replace Topcoat CSS demonstration with Pico CSS integration in examples",
          "timestamp": "2026-08-15T15:41:39-03:00",
          "tree_id": "fbecad99e3c93155a04ceb3d571cd65695eb87bb",
          "url": "https://github.com/Rullst/Rullst/commit/10dce6749335e9c0456c20ec6a5710d203ecde8f"
        },
        "date": 1786819617202,
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
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 518,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 396,
            "range": "± 7",
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
            "value": 1436706,
            "range": "± 32152",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 76130,
            "range": "± 994",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 81509,
            "range": "± 634",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 77249,
            "range": "± 2739",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 84610,
            "range": "± 736",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 77043,
            "range": "± 957",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 85567,
            "range": "± 1020",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153654,
            "range": "± 2573",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 254117,
            "range": "± 6818",
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
          "id": "0414c3313b894dd85e8a6d09369c63d8267346c3",
          "message": "refactor: reorder module imports for consistency in example demo files",
          "timestamp": "2026-08-15T16:08:51-03:00",
          "tree_id": "dbbfb36faa762e2fbf23d6c94bcbea294ade3e14",
          "url": "https://github.com/Rullst/Rullst/commit/0414c3313b894dd85e8a6d09369c63d8267346c3"
        },
        "date": 1786821236137,
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
            "value": 47,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 476,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 375,
            "range": "± 3",
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
            "value": 2002773,
            "range": "± 95333",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 101580,
            "range": "± 1166",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 107249,
            "range": "± 1713",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 99695,
            "range": "± 3484",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 109621,
            "range": "± 2323",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 100493,
            "range": "± 2218",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 108655,
            "range": "± 2256",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155067,
            "range": "± 1873",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 250884,
            "range": "± 3758",
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
          "id": "7fb6299e9550222f4a307c11fee1ac6549d2ba01",
          "message": "refactor: modularize codebase into library, add rullst binary, and update studio port configurations",
          "timestamp": "2026-08-15T18:47:21-03:00",
          "tree_id": "e56f9421e5bf1bf08b55bf839487f9f47504156c",
          "url": "https://github.com/Rullst/Rullst/commit/7fb6299e9550222f4a307c11fee1ac6549d2ba01"
        },
        "date": 1786830774636,
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
            "value": 42,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 418,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 283,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 167,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 3768450,
            "range": "± 5408682",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 70315,
            "range": "± 3527",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 65046,
            "range": "± 1614",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 61874,
            "range": "± 1802",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 70622,
            "range": "± 2347",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 65054,
            "range": "± 1989",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 72590,
            "range": "± 1968",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 146308,
            "range": "± 7435",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 262634,
            "range": "± 12060",
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
          "id": "aca59f340bb3280d97f2042df2ddb826c2f33c9f",
          "message": "style: reformat get_routes function arguments for improved readability",
          "timestamp": "2026-08-16T02:37:07-03:00",
          "tree_id": "ca27d1f3d2c358f31f0093fa5577c5ef59f8f684",
          "url": "https://github.com/Rullst/Rullst/commit/aca59f340bb3280d97f2042df2ddb826c2f33c9f"
        },
        "date": 1786858930626,
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
            "value": 11,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 38,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 382,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 260,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 162,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2277808,
            "range": "± 1610739",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 80138,
            "range": "± 1558",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 61368,
            "range": "± 1644",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 59042,
            "range": "± 2036",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 67978,
            "range": "± 3885",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 60144,
            "range": "± 1476",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 67768,
            "range": "± 1520",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 141195,
            "range": "± 7585",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 248738,
            "range": "± 9751",
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
          "id": "4db5e073af71f80c6bf535ce2b73dcf5302320dd",
          "message": "feat: re-export axum http primitives in lib.rs for easier access",
          "timestamp": "2026-08-16T03:22:46-03:00",
          "tree_id": "ca20103c97089051b4cab993548ce8f06f9c9de8",
          "url": "https://github.com/Rullst/Rullst/commit/4db5e073af71f80c6bf535ce2b73dcf5302320dd"
        },
        "date": 1786861678528,
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 504,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 384,
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
            "value": 1479787,
            "range": "± 199245",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 77136,
            "range": "± 1105",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 82070,
            "range": "± 1326",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 76927,
            "range": "± 1198",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 86120,
            "range": "± 764",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 77056,
            "range": "± 2611",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 85704,
            "range": "± 1156",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153513,
            "range": "± 3935",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 257244,
            "range": "± 2463",
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
          "id": "001c3e623343b87085fc7487e168cce3983388db",
          "message": "refactor: update routing API, improve security policies, and standardize test database initialization",
          "timestamp": "2026-08-16T14:01:41-03:00",
          "tree_id": "d50b77e82998efd926143b88746af0837931477d",
          "url": "https://github.com/Rullst/Rullst/commit/001c3e623343b87085fc7487e168cce3983388db"
        },
        "date": 1786900025541,
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
            "value": 49,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 470,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 373,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 195,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1879113,
            "range": "± 62302",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 99546,
            "range": "± 1266",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 105040,
            "range": "± 1581",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 99951,
            "range": "± 2549",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 106715,
            "range": "± 1539",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 98676,
            "range": "± 2005",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 106938,
            "range": "± 1298",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155342,
            "range": "± 3411",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 247632,
            "range": "± 2713",
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
          "id": "ed6e0e1eb7a1ab731c647c4b7618e1912bca48ac",
          "message": "feat: add CI/CD infrastructure, initialize rullst-studio crate, and update development workflow documentation",
          "timestamp": "2026-08-17T17:04:01-03:00",
          "tree_id": "33da082ce4aa5c3c19eb27f9a630039c63e689b8",
          "url": "https://github.com/Rullst/Rullst/commit/ed6e0e1eb7a1ab731c647c4b7618e1912bca48ac"
        },
        "date": 1786997344017,
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
            "value": 50,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 465,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 369,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 188,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1945217,
            "range": "± 89682",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 97697,
            "range": "± 1088",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 102995,
            "range": "± 1051",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 98218,
            "range": "± 2609",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 104406,
            "range": "± 1768",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 96138,
            "range": "± 1854",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 105520,
            "range": "± 2095",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 151926,
            "range": "± 1065",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 240464,
            "range": "± 2490",
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
          "id": "0bce5e48eda7e9718a15d5849c5b4f21774b5d21",
          "message": "feat: implement CRUD table generation, telemetry utilities, and security CI/CD workflows",
          "timestamp": "2026-08-17T22:01:24-03:00",
          "tree_id": "0a7769009610f3c0fd5cb56c3684090b5c4cb5b6",
          "url": "https://github.com/Rullst/Rullst/commit/0bce5e48eda7e9718a15d5849c5b4f21774b5d21"
        },
        "date": 1787015190651,
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
            "value": 35,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 418,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 323,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 155,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2002455,
            "range": "± 376250",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 59652,
            "range": "± 791",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 63444,
            "range": "± 688",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 59251,
            "range": "± 877",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 67029,
            "range": "± 5134",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 60887,
            "range": "± 690",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 66961,
            "range": "± 924",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 118237,
            "range": "± 1354",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 196267,
            "range": "± 4308",
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
          "id": "35dc55663b40ab5b5e1efd7e79a1f823bdd4b735",
          "message": "docs: update project setup instructions and reorganize README visual content",
          "timestamp": "2026-08-17T22:30:12-03:00",
          "tree_id": "3041fe30815ce428bda138a8cb21bf416160a74f",
          "url": "https://github.com/Rullst/Rullst/commit/35dc55663b40ab5b5e1efd7e79a1f823bdd4b735"
        },
        "date": 1787016902548,
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
            "value": 47,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 448,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 362,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 186,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1803851,
            "range": "± 53802",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 93696,
            "range": "± 2189",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 99970,
            "range": "± 1642",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 94811,
            "range": "± 2960",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 102543,
            "range": "± 2965",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 91970,
            "range": "± 2117",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 102491,
            "range": "± 1358",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 149112,
            "range": "± 1991",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 238883,
            "range": "± 3972",
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
          "id": "3c2ce543d569f346d8b1b716328a3a5b7ae17168",
          "message": "feat: add openapi generator, security modules, and comprehensive integration testing suite",
          "timestamp": "2026-08-19T01:19:36-03:00",
          "tree_id": "dd803315983702bd32fc91679b51564fe6879351",
          "url": "https://github.com/Rullst/Rullst/commit/3c2ce543d569f346d8b1b716328a3a5b7ae17168"
        },
        "date": 1787114023322,
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
            "value": 415,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 316,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 163,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1870555,
            "range": "± 1374205",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 59221,
            "range": "± 829",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 62980,
            "range": "± 551",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 60250,
            "range": "± 818",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 65968,
            "range": "± 797",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 60014,
            "range": "± 3126",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 66288,
            "range": "± 1027",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 118806,
            "range": "± 1255",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 197751,
            "range": "± 3852",
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
          "id": "7dd283b2ac6c27f06718cb48dbcf49a5750d7b74",
          "message": "feat: implement Nexus CRUD dashboard with auto-generated admin interface and core system scaffolding",
          "timestamp": "2026-08-19T19:28:04-03:00",
          "tree_id": "282e6522ef2a314beade8c89e4352e0023ef4f61",
          "url": "https://github.com/Rullst/Rullst/commit/7dd283b2ac6c27f06718cb48dbcf49a5750d7b74"
        },
        "date": 1787178777789,
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
            "value": 51,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 427,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 306,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 165,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1917314,
            "range": "± 217927",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 59111,
            "range": "± 789",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 63222,
            "range": "± 1118",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 60248,
            "range": "± 588",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 66773,
            "range": "± 1012",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 60396,
            "range": "± 716",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 68069,
            "range": "± 938",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 120443,
            "range": "± 2166",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 202525,
            "range": "± 1558",
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
          "id": "0f8d2e5d4eb0da69c42093f05c92ae2758ad571e",
          "message": "feat: initialize project workspace with cargo packages for core infrastructure components",
          "timestamp": "2026-08-19T20:17:34-03:00",
          "tree_id": "6bbce0b67783e2b709d38a501cb144fc6049f240",
          "url": "https://github.com/Rullst/Rullst/commit/0f8d2e5d4eb0da69c42093f05c92ae2758ad571e"
        },
        "date": 1787181755595,
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
            "value": 509,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 391,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 200,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1506212,
            "range": "± 66691",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 76350,
            "range": "± 1298",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 81017,
            "range": "± 1334",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 77223,
            "range": "± 1105",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 85458,
            "range": "± 1007",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 76820,
            "range": "± 1261",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 85745,
            "range": "± 1144",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153557,
            "range": "± 4174",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 256100,
            "range": "± 11806",
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
          "id": "39149ada8757cb3abb3d6bf7fc105c7f4a9dbc48",
          "message": "feat: implement global security telemetry store for event tracking and monitoring",
          "timestamp": "2026-08-19T22:26:21-03:00",
          "tree_id": "6d02cff922faeff2c6930ea63e01d5e5d6a9d512",
          "url": "https://github.com/Rullst/Rullst/commit/39149ada8757cb3abb3d6bf7fc105c7f4a9dbc48"
        },
        "date": 1787189464252,
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
            "value": 9,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 25,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 291,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 203,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 123,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 4338926,
            "range": "± 11016465",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 51783,
            "range": "± 731",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 53091,
            "range": "± 1385",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 50888,
            "range": "± 852",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 57551,
            "range": "± 5117",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 51634,
            "range": "± 709",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 57950,
            "range": "± 3791",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 98577,
            "range": "± 14774",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 159911,
            "range": "± 1023",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "venelouis",
            "username": "venelouis",
            "email": "venelouistyago@gmail.com"
          },
          "committer": {
            "name": "venelouis",
            "username": "venelouis",
            "email": "venelouistyago@gmail.com"
          },
          "id": "39a4b129af8a5b27bcb06be9f40dd7a986918734",
          "message": "ci: add GitHub Actions workflows for CodeQL security analysis and LLVM test coverage reporting",
          "timestamp": "2026-08-20T18:48:43Z",
          "url": "https://github.com/Rullst/Rullst/commit/39a4b129af8a5b27bcb06be9f40dd7a986918734"
        },
        "date": 1787254918271,
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
            "value": 495,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 385,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 201,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1482583,
            "range": "± 37599",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 78498,
            "range": "± 1224",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 83286,
            "range": "± 813",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 77464,
            "range": "± 1524",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 88043,
            "range": "± 917",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 78871,
            "range": "± 876",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 88108,
            "range": "± 811",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 150436,
            "range": "± 4017",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 250788,
            "range": "± 2858",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "venelouis",
            "username": "venelouis",
            "email": "venelouistyago@gmail.com"
          },
          "committer": {
            "name": "venelouis",
            "username": "venelouis",
            "email": "venelouistyago@gmail.com"
          },
          "id": "068fbd3c8453af922ecfaaee6c2ccd181943a7dc",
          "message": "feat: implement multi-provider AI support, enhance security middleware, and expand project test coverage documentation",
          "timestamp": "2026-08-24T04:34:58Z",
          "url": "https://github.com/Rullst/Rullst/commit/068fbd3c8453af922ecfaaee6c2ccd181943a7dc"
        },
        "date": 1787546789022,
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
            "value": 478,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 387,
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
            "value": 1548650,
            "range": "± 106045",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 76904,
            "range": "± 1033",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 81523,
            "range": "± 782",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 77250,
            "range": "± 1056",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 85972,
            "range": "± 1030",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 77348,
            "range": "± 1415",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 86850,
            "range": "± 954",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 156552,
            "range": "± 2482",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 263832,
            "range": "± 2438",
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
          "id": "bb07dc2e7a96ca2c0dd87838be98e68c152f119f",
          "message": "ci: add automated benchmark and semver regression workflows, and expand project testing and modular support.",
          "timestamp": "2026-08-27T18:21:04-03:00",
          "tree_id": "11a05be9f6e35a84b89577a9fe3c6610cf5c1df9",
          "url": "https://github.com/Rullst/Rullst/commit/bb07dc2e7a96ca2c0dd87838be98e68c152f119f"
        },
        "date": 1787866071068,
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
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 407,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 321,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 176,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1343348,
            "range": "± 56364",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 64061,
            "range": "± 1501",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 68771,
            "range": "± 1514",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 62466,
            "range": "± 1137",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 72323,
            "range": "± 1301",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 63478,
            "range": "± 907",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 72673,
            "range": "± 988",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 143576,
            "range": "± 1966",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 251624,
            "range": "± 3442",
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
          "id": "d2e5580ab99bf2078b7dbc752de9fb7ff0203f79",
          "message": "feat(v12): harden security contracts and LMS scaffold",
          "timestamp": "2026-08-27T22:35:19-03:00",
          "tree_id": "fb43353d9afdf868f1926e5c3854df7dc8ef8e8e",
          "url": "https://github.com/Rullst/Rullst/commit/d2e5580ab99bf2078b7dbc752de9fb7ff0203f79"
        },
        "date": 1787881361420,
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
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 494,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 394,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 186,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2021357,
            "range": "± 114547",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 98833,
            "range": "± 857",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 103152,
            "range": "± 819",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 97579,
            "range": "± 2277",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105059,
            "range": "± 1808",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 96597,
            "range": "± 2268",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 105267,
            "range": "± 2023",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 152882,
            "range": "± 1554",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 243548,
            "range": "± 2216",
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
          "id": "cad5729a21eb15ae4be65dd8b1597fc705b5ec4d",
          "message": "fix(release): verify cli bootstrap from packages",
          "timestamp": "2026-08-29T02:50:30-03:00",
          "tree_id": "82759ff6998afe8f65244f598a019c6185d21c48",
          "url": "https://github.com/Rullst/Rullst/commit/cad5729a21eb15ae4be65dd8b1597fc705b5ec4d"
        },
        "date": 1787984396801,
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
            "value": 13,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 35,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 389,
            "range": "± 1",
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
            "value": 152,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1788824,
            "range": "± 125506",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 60029,
            "range": "± 887",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 63725,
            "range": "± 1202",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 59762,
            "range": "± 1238",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 67138,
            "range": "± 770",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 59925,
            "range": "± 682",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 66907,
            "range": "± 691",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 121045,
            "range": "± 2116",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 194872,
            "range": "± 1735",
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
          "id": "51922e63c965882224f78bb4c77860f3eac9acd2",
          "message": "fix(ci): restore clean cross-platform verification",
          "timestamp": "2026-08-29T03:36:37-03:00",
          "tree_id": "f51b5ca6a41e32641b087ddab2d57c5288cad470",
          "url": "https://github.com/Rullst/Rullst/commit/51922e63c965882224f78bb4c77860f3eac9acd2"
        },
        "date": 1787985671056,
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
            "range": "± 4",
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
            "value": 367,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 203,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1849098,
            "range": "± 73994",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 97062,
            "range": "± 1328",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 101938,
            "range": "± 1677",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 97307,
            "range": "± 2986",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 104212,
            "range": "± 1612",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 94835,
            "range": "± 2805",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 104411,
            "range": "± 2433",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154086,
            "range": "± 3205",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 241853,
            "range": "± 3461",
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
          "id": "dbd82756413a7194006988514993ab7ffb9e5001",
          "message": "fix(cli): align generated tera dependency",
          "timestamp": "2026-08-29T04:31:37-03:00",
          "tree_id": "528314d9f3fffdff3c6bf0669a3759f7b0bfef8d",
          "url": "https://github.com/Rullst/Rullst/commit/dbd82756413a7194006988514993ab7ffb9e5001"
        },
        "date": 1787990408623,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 5,
            "range": "± 1",
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
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 298,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 217,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 127,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 3995570,
            "range": "± 12539295",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 63393,
            "range": "± 1234",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 66751,
            "range": "± 1469",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 62711,
            "range": "± 1057",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 72743,
            "range": "± 1751",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 64404,
            "range": "± 1578",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 73363,
            "range": "± 2312",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 130060,
            "range": "± 8455",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 231062,
            "range": "± 16018",
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
          "id": "1b6b2da9a62eaa05dd98144c7b3c08cc239d8616",
          "message": "feat(v12): implement audited roadmap capabilities",
          "timestamp": "2026-08-29T23:27:39-03:00",
          "tree_id": "8f335a4285b1f154dd589ba76e4b432943b3eda8",
          "url": "https://github.com/Rullst/Rullst/commit/1b6b2da9a62eaa05dd98144c7b3c08cc239d8616"
        },
        "date": 1788057415308,
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
            "value": 14,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 42,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 453,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 340,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 176,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2216453,
            "range": "± 515181",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 68971,
            "range": "± 2116",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 73409,
            "range": "± 2024",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 68937,
            "range": "± 2511",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 75357,
            "range": "± 2431",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 67674,
            "range": "± 1756",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 76281,
            "range": "± 2159",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 139292,
            "range": "± 5934",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 228159,
            "range": "± 7490",
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
          "id": "3599d0adb6652a0d4d482f9fd3e152f535b6c711",
          "message": "fix(orm): make implicit cascades atomic",
          "timestamp": "2026-08-30T00:29:55-03:00",
          "tree_id": "e2093ed5d7b1cdc59acbf5827b5ca349d46d9519",
          "url": "https://github.com/Rullst/Rullst/commit/3599d0adb6652a0d4d482f9fd3e152f535b6c711"
        },
        "date": 1788061011057,
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
            "value": 463,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 373,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 194,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2168784,
            "range": "± 122575",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 102635,
            "range": "± 1144",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 106610,
            "range": "± 1106",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 100416,
            "range": "± 2442",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 109024,
            "range": "± 2069",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 100772,
            "range": "± 1839",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 108433,
            "range": "± 2722",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154792,
            "range": "± 2507",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 242561,
            "range": "± 4752",
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
          "id": "2cfc54a913445f2843b92b9ccb15924cd1434666",
          "message": "feat(orm): type generated primitive query filters",
          "timestamp": "2026-08-30T00:45:41-03:00",
          "tree_id": "c9958dea1e00deb8ae67c133ebfd6482f0a749cf",
          "url": "https://github.com/Rullst/Rullst/commit/2cfc54a913445f2843b92b9ccb15924cd1434666"
        },
        "date": 1788062124648,
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
            "value": 477,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 377,
            "range": "± 3",
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
            "value": 2054168,
            "range": "± 54141",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 100730,
            "range": "± 1473",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 105453,
            "range": "± 1205",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 99090,
            "range": "± 2462",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 107935,
            "range": "± 1881",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 100251,
            "range": "± 2364",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 108812,
            "range": "± 2161",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 156391,
            "range": "± 2722",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 252695,
            "range": "± 5956",
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
          "id": "644388ba7ad58f1c391004cfb761e145a6c6317e",
          "message": "feat(orm): add typed inverse polymorphic relations",
          "timestamp": "2026-08-30T00:58:04-03:00",
          "tree_id": "47313c9e9dc05ea34d1e65f6fcb48aebf7cd4b1f",
          "url": "https://github.com/Rullst/Rullst/commit/644388ba7ad58f1c391004cfb761e145a6c6317e"
        },
        "date": 1788062581018,
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
            "value": 17,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 475,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 394,
            "range": "± 1",
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
            "value": 1564725,
            "range": "± 98002",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 76889,
            "range": "± 1245",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 81619,
            "range": "± 943",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 77507,
            "range": "± 1067",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 85762,
            "range": "± 789",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 77518,
            "range": "± 879",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 87085,
            "range": "± 1274",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 156537,
            "range": "± 1786",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 253039,
            "range": "± 5120",
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
          "id": "b73d29b2fe86f2b65c38b4ce644eb6d37f7e6ff2",
          "message": "feat(cli): verify hardened iOS Omni scaffolds",
          "timestamp": "2026-08-30T03:18:37-03:00",
          "tree_id": "9fe5cc949e2d04f800df894d1f2e1b34e6871b81",
          "url": "https://github.com/Rullst/Rullst/commit/b73d29b2fe86f2b65c38b4ce644eb6d37f7e6ff2"
        },
        "date": 1788071306530,
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
            "value": 15,
            "range": "± 1",
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
            "value": 463,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 377,
            "range": "± 2",
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
            "value": 2105056,
            "range": "± 175687",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 103167,
            "range": "± 1240",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 106499,
            "range": "± 810",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 99991,
            "range": "± 1977",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 109662,
            "range": "± 2597",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 100311,
            "range": "± 2075",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 109307,
            "range": "± 3318",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154456,
            "range": "± 3497",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 246511,
            "range": "± 5844",
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
          "id": "3720f92ac7bc58b59c8d1505fb10be2e08d3aff3",
          "message": "fix(orm): replace vulnerable Turso transport",
          "timestamp": "2026-08-30T03:51:43-03:00",
          "tree_id": "79c493bc0ec3d91e98e92c275ee6df576def78cf",
          "url": "https://github.com/Rullst/Rullst/commit/3720f92ac7bc58b59c8d1505fb10be2e08d3aff3"
        },
        "date": 1788073027186,
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
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 424,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 339,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 183,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1477235,
            "range": "± 57687",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 63736,
            "range": "± 1329",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 67681,
            "range": "± 1153",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 62278,
            "range": "± 1526",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 73251,
            "range": "± 1760",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 64399,
            "range": "± 1343",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 73497,
            "range": "± 1627",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 142528,
            "range": "± 3617",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 248869,
            "range": "± 16215",
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
          "id": "a4a9092469fbe186b0724b4dd268a8cb72257063",
          "message": "fix(orm): make audit writes transaction atomic",
          "timestamp": "2026-08-30T05:43:20-03:00",
          "tree_id": "ac12ef7ef1b8114fb4d9a6dc10d9c194805d3deb",
          "url": "https://github.com/Rullst/Rullst/commit/a4a9092469fbe186b0724b4dd268a8cb72257063"
        },
        "date": 1788079682299,
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
            "range": "± 9",
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
            "value": 194,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2782412,
            "range": "± 394638",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 91298,
            "range": "± 1143",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 95275,
            "range": "± 2268",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 91973,
            "range": "± 3056",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 98525,
            "range": "± 1585",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 86454,
            "range": "± 3166",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 98623,
            "range": "± 1586",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 143414,
            "range": "± 2076",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 248439,
            "range": "± 2510",
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
          "id": "9005dcd38a603e1ec15f61d8a87a9fed8d5d2fbe",
          "message": "fix(orm): isolate remembered query caches",
          "timestamp": "2026-08-30T06:23:37-03:00",
          "tree_id": "06e98092aabdaa644a4771d30d9af9ce423751d3",
          "url": "https://github.com/Rullst/Rullst/commit/9005dcd38a603e1ec15f61d8a87a9fed8d5d2fbe"
        },
        "date": 1788082473997,
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
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 480,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 350,
            "range": "± 3",
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
            "value": 2131422,
            "range": "± 194049",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 100116,
            "range": "± 1109",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 104334,
            "range": "± 1212",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 98548,
            "range": "± 4638",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105724,
            "range": "± 1303",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 96504,
            "range": "± 2172",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 106358,
            "range": "± 2593",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155072,
            "range": "± 2189",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 250296,
            "range": "± 4160",
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
          "distinct": false,
          "id": "9503a16ad35d4e49e687fab1040437daae029507",
          "message": "chore(repo): converge v5 and v12 histories",
          "timestamp": "2026-08-30T06:45:36-03:00",
          "tree_id": "888a1f0fb86e976b847bb103fbab54a9c9418fd7",
          "url": "https://github.com/Rullst/Rullst/commit/9503a16ad35d4e49e687fab1040437daae029507"
        },
        "date": 1788083754061,
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
            "value": 48,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 491,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 403,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 214,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1559200,
            "range": "± 40166",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 75744,
            "range": "± 985",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 81087,
            "range": "± 1212",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 77279,
            "range": "± 1144",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 84731,
            "range": "± 1226",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 76305,
            "range": "± 1023",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 83966,
            "range": "± 1169",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 157370,
            "range": "± 10231",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 247230,
            "range": "± 3547",
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
          "id": "32c6c2fd314c8d42dd8948d1e686d58d1054fc15",
          "message": "ci(coverage): authenticate uploads with oidc",
          "timestamp": "2026-08-30T07:15:30-03:00",
          "tree_id": "809e1c3998fd3442d3db50d43d7af8a8bac16e52",
          "url": "https://github.com/Rullst/Rullst/commit/32c6c2fd314c8d42dd8948d1e686d58d1054fc15"
        },
        "date": 1788086080573,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 5,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 11,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 38,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 361,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 258,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 159,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2381516,
            "range": "± 459117",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 78045,
            "range": "± 1577",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 63590,
            "range": "± 1608",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 64523,
            "range": "± 5444",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 72532,
            "range": "± 3118",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 62347,
            "range": "± 2399",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 70237,
            "range": "± 2103",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 141069,
            "range": "± 8707",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 257982,
            "range": "± 10105",
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
          "id": "d8c38e4364d311957b87b1a9299f8a77efaae600",
          "message": "feat(orm): add managed post-commit effects",
          "timestamp": "2026-08-30T08:23:27-03:00",
          "tree_id": "2cddfb5c017bdcf9a75afec1118fbccf6b03c982",
          "url": "https://github.com/Rullst/Rullst/commit/d8c38e4364d311957b87b1a9299f8a77efaae600"
        },
        "date": 1788089644352,
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
            "value": 496,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 359,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 208,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2483691,
            "range": "± 250792",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 92170,
            "range": "± 1584",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 98233,
            "range": "± 2759",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 92284,
            "range": "± 2774",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 100490,
            "range": "± 2395",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 88191,
            "range": "± 2792",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 101022,
            "range": "± 1965",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 147673,
            "range": "± 3364",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 239300,
            "range": "± 3529",
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
          "id": "a88aabd66b0a52ed4e9a543a7ceefb9801574446",
          "message": "feat(orm): add durable transactional outbox",
          "timestamp": "2026-08-30T09:53:20-03:00",
          "tree_id": "078736268707dfec79cc38923280b17a61ddeb2b",
          "url": "https://github.com/Rullst/Rullst/commit/a88aabd66b0a52ed4e9a543a7ceefb9801574446"
        },
        "date": 1788095056933,
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
            "value": 49,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 474,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 357,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 215,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2263238,
            "range": "± 185504",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 101466,
            "range": "± 1436",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 105935,
            "range": "± 1282",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 100794,
            "range": "± 1783",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 108842,
            "range": "± 1612",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 99180,
            "range": "± 1961",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 108213,
            "range": "± 1429",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155089,
            "range": "± 1746",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 250306,
            "range": "± 3006",
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
          "id": "ef8f3aae18838f540faa893e74811f8ef90dbcf5",
          "message": "feat(orm): add bounded scout providers",
          "timestamp": "2026-08-30T10:22:52-03:00",
          "tree_id": "5f59868b0609bbb2fe9443484523c6d320ef4dff",
          "url": "https://github.com/Rullst/Rullst/commit/ef8f3aae18838f540faa893e74811f8ef90dbcf5"
        },
        "date": 1788096445849,
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
            "value": 476,
            "range": "± 3",
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
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2137602,
            "range": "± 120848",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 99203,
            "range": "± 1293",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 104974,
            "range": "± 1042",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 98050,
            "range": "± 2139",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105812,
            "range": "± 1422",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 95529,
            "range": "± 2405",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 106041,
            "range": "± 1416",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 152577,
            "range": "± 1443",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 245492,
            "range": "± 2648",
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
          "id": "69c0c8fdb08ce069d05598457bf131e09b75bd2b",
          "message": "feat(orm): add typed pgvector contract",
          "timestamp": "2026-08-30T10:56:53-03:00",
          "tree_id": "36b226e99264ebf14477456b3637b82eafa7c031",
          "url": "https://github.com/Rullst/Rullst/commit/69c0c8fdb08ce069d05598457bf131e09b75bd2b"
        },
        "date": 1788098845182,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 10,
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
            "value": 363,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 273,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 177,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2233347,
            "range": "± 2735072",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 61357,
            "range": "± 991",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 64697,
            "range": "± 988",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 60822,
            "range": "± 1135",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 68022,
            "range": "± 854",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 60573,
            "range": "± 835",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 67952,
            "range": "± 891",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 120758,
            "range": "± 1561",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 200548,
            "range": "± 2034",
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
          "id": "fb5909560635187434eabfab552f273fe3661447",
          "message": "feat(orm): add bounded qdrant and redis stores",
          "timestamp": "2026-08-30T12:13:42-03:00",
          "tree_id": "29d9407a16fe63ab01d34a65368258688afd12d2",
          "url": "https://github.com/Rullst/Rullst/commit/fb5909560635187434eabfab552f273fe3661447"
        },
        "date": 1788103359816,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 10,
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
            "value": 361,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 276,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 188,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2053312,
            "range": "± 2260187",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 59931,
            "range": "± 783",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 63364,
            "range": "± 837",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 58477,
            "range": "± 894",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 66437,
            "range": "± 1940",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 59485,
            "range": "± 586",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 66239,
            "range": "± 887",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 121260,
            "range": "± 1766",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 200526,
            "range": "± 2375",
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
          "id": "4f4da233e56dbdc7aa09e9e4813931ed942782bf",
          "message": "fix(security): box redis rate limit client",
          "timestamp": "2026-08-30T12:25:20-03:00",
          "tree_id": "28508b05d1583c1977cb859f7efa09bf0564a667",
          "url": "https://github.com/Rullst/Rullst/commit/4f4da233e56dbdc7aa09e9e4813931ed942782bf"
        },
        "date": 1788103850819,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 8,
            "range": "± 1",
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
            "value": 438,
            "range": "± 4",
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
            "value": 209,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2885171,
            "range": "± 690295",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 70080,
            "range": "± 2898",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 75528,
            "range": "± 1150",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 69617,
            "range": "± 1044",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 80579,
            "range": "± 1851",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 71719,
            "range": "± 831",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 81693,
            "range": "± 2179",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 162508,
            "range": "± 6290",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 295518,
            "range": "± 7592",
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
          "id": "891050c3d08da86fd85b492b8d7db9e95d0d83e7",
          "message": "docs(project): explain why to choose rullst",
          "timestamp": "2026-08-30T13:42:05-03:00",
          "tree_id": "a035685c59848d521ef568fa293641afdf69be28",
          "url": "https://github.com/Rullst/Rullst/commit/891050c3d08da86fd85b492b8d7db9e95d0d83e7"
        },
        "date": 1788108663970,
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
            "value": 38,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 365,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 273,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 184,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2034734,
            "range": "± 361085",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 59739,
            "range": "± 707",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 63441,
            "range": "± 654",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 59584,
            "range": "± 689",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 66956,
            "range": "± 1182",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 60295,
            "range": "± 1357",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 67082,
            "range": "± 681",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 119968,
            "range": "± 1547",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 208325,
            "range": "± 3678",
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
          "id": "353ebaec07c44723509d9239c7dc061b680f3d9b",
          "message": "feat(capital): add bounded nfse preparation",
          "timestamp": "2026-08-30T15:24:50-03:00",
          "tree_id": "47831ab5878bc7a0e895f9c6920d7e700d43e307",
          "url": "https://github.com/Rullst/Rullst/commit/353ebaec07c44723509d9239c7dc061b680f3d9b"
        },
        "date": 1788114665879,
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
            "value": 353,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 210,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 3588216,
            "range": "± 574415",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 90890,
            "range": "± 1339",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 95588,
            "range": "± 1772",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 90743,
            "range": "± 3693",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 100853,
            "range": "± 1881",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 91126,
            "range": "± 2297",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 99619,
            "range": "± 2013",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 143706,
            "range": "± 1782",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 246736,
            "range": "± 4078",
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
          "id": "8fd92136bb62a4bf8c80639118eb004abcd4c91d",
          "message": "feat(mail): complete fiscal and dunning scaffolds",
          "timestamp": "2026-08-30T16:05:06-03:00",
          "tree_id": "cb145acfc4976ff4132ce26718c486ee0eca11a7",
          "url": "https://github.com/Rullst/Rullst/commit/8fd92136bb62a4bf8c80639118eb004abcd4c91d"
        },
        "date": 1788117006942,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 8,
            "range": "± 1",
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
            "value": 47,
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
            "value": 302,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 202,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2622108,
            "range": "± 417205",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 70032,
            "range": "± 1189",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 74899,
            "range": "± 1030",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 70709,
            "range": "± 902",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 81282,
            "range": "± 1309",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 70994,
            "range": "± 1259",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 81431,
            "range": "± 1480",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 163081,
            "range": "± 6004",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 301419,
            "range": "± 9449",
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
          "id": "676026b4944c9a5886c9e9579894156d5e1c91b3",
          "message": "feat(mail): bind tenant resolver to auth context",
          "timestamp": "2026-08-30T16:11:35-03:00",
          "tree_id": "97375fd3f71f007bcaff1049aac6904a9662825d",
          "url": "https://github.com/Rullst/Rullst/commit/676026b4944c9a5886c9e9579894156d5e1c91b3"
        },
        "date": 1788117413068,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 7,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 11,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 40,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 372,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 260,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 168,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2831775,
            "range": "± 1377847",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 63201,
            "range": "± 3833",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 66270,
            "range": "± 3064",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 62529,
            "range": "± 2476",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 74481,
            "range": "± 3314",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 64503,
            "range": "± 2782",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 74433,
            "range": "± 3719",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 149618,
            "range": "± 8972",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 260030,
            "range": "± 16049",
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
          "id": "cd2b10965e831654639dd2a1188a82d3b326ebbe",
          "message": "feat(connect): add bounded corporate proxy client",
          "timestamp": "2026-08-30T16:17:56-03:00",
          "tree_id": "5dc0818e47fe6a0fcff8469fc5e7b5c95b5ef1a7",
          "url": "https://github.com/Rullst/Rullst/commit/cd2b10965e831654639dd2a1188a82d3b326ebbe"
        },
        "date": 1788117778354,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 6,
            "range": "± 1",
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
            "value": 300,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 211,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 130,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 3619798,
            "range": "± 28983564",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 60988,
            "range": "± 2151",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 64005,
            "range": "± 2332",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 61984,
            "range": "± 1497",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 69874,
            "range": "± 1396",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 62040,
            "range": "± 1350",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 69709,
            "range": "± 2086",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 130943,
            "range": "± 4893",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 230070,
            "range": "± 14281",
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
          "id": "8b77b3ffcbe304bcc0cfb3a279f5f398b6954232",
          "message": "feat(mail): classify failover errors",
          "timestamp": "2026-08-30T16:25:47-03:00",
          "tree_id": "804787300cce2cfd09a3aca297399cdd7b46b7c5",
          "url": "https://github.com/Rullst/Rullst/commit/8b77b3ffcbe304bcc0cfb3a279f5f398b6954232"
        },
        "date": 1788118545326,
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
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 459,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 347,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 210,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2162114,
            "range": "± 95843",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 91159,
            "range": "± 1441",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 95298,
            "range": "± 1186",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 90228,
            "range": "± 4312",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 99074,
            "range": "± 1661",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 88231,
            "range": "± 2577",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 99329,
            "range": "± 1054",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 144637,
            "range": "± 1591",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 254327,
            "range": "± 4086",
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
          "id": "0a9d35e30e1d6836558e97d920c49cb3d6663563",
          "message": "feat(mail): add durable scheduled delivery",
          "timestamp": "2026-08-30T16:45:44-03:00",
          "tree_id": "5c62c67a7864e3e050da1ae0cfe98c9502216c99",
          "url": "https://github.com/Rullst/Rullst/commit/0a9d35e30e1d6836558e97d920c49cb3d6663563"
        },
        "date": 1788119416886,
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 474,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 347,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 211,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2116340,
            "range": "± 154151",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 99508,
            "range": "± 1243",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 104584,
            "range": "± 1940",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 98080,
            "range": "± 2546",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105870,
            "range": "± 1606",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 94850,
            "range": "± 2581",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 106034,
            "range": "± 1752",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 156517,
            "range": "± 1621",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 253077,
            "range": "± 2190",
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
          "id": "26451e691ad885bff791c98c0bd0e1083fef951a",
          "message": "feat(studio): invalidate feature flag caches",
          "timestamp": "2026-08-30T16:52:16-03:00",
          "tree_id": "10b45c14d94ed557c20665d071ac6827b8019196",
          "url": "https://github.com/Rullst/Rullst/commit/26451e691ad885bff791c98c0bd0e1083fef951a"
        },
        "date": 1788120135459,
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
            "value": 464,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 346,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 210,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2152773,
            "range": "± 91837",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 99126,
            "range": "± 1369",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 103617,
            "range": "± 1451",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 96482,
            "range": "± 2806",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 104721,
            "range": "± 1581",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 95839,
            "range": "± 1827",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 104212,
            "range": "± 1413",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 151767,
            "range": "± 1351",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 243636,
            "range": "± 8053",
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
          "id": "171eda55c909245a86b061f369b79558d813467c",
          "message": "feat(ai): add tenant-bound audited rag pipeline",
          "timestamp": "2026-08-30T17:12:10-03:00",
          "tree_id": "bf1d07a51cbd9c4abf0c1bbb4bd4c8ac020d32fd",
          "url": "https://github.com/Rullst/Rullst/commit/171eda55c909245a86b061f369b79558d813467c"
        },
        "date": 1788121042350,
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 460,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 351,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 218,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2246340,
            "range": "± 112588",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 103865,
            "range": "± 1431",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 108905,
            "range": "± 1306",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 101745,
            "range": "± 1833",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 110785,
            "range": "± 1498",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 102786,
            "range": "± 1213",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 111695,
            "range": "± 2234",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 159909,
            "range": "± 2199",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 252173,
            "range": "± 3456",
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
          "id": "3baa13c4e496da89631e2a7ab19aca89bb36c8c0",
          "message": "feat(studio): retain bounded queue completion history",
          "timestamp": "2026-08-30T17:20:58-03:00",
          "tree_id": "7f078a8eae71471ea1f945003eadb5fa8d73e048",
          "url": "https://github.com/Rullst/Rullst/commit/3baa13c4e496da89631e2a7ab19aca89bb36c8c0"
        },
        "date": 1788121563168,
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
            "value": 468,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 344,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 211,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2255178,
            "range": "± 128280",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 97850,
            "range": "± 1232",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 102555,
            "range": "± 2043",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 96087,
            "range": "± 2149",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 104136,
            "range": "± 1181",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 94123,
            "range": "± 2524",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 104779,
            "range": "± 1984",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153002,
            "range": "± 1371",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 245954,
            "range": "± 4619",
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
          "id": "154c455a671755e029d0d087b24c2f2d177274c2",
          "message": "feat(connect): manage oauth session challenges",
          "timestamp": "2026-08-30T17:53:06-03:00",
          "tree_id": "4c844527fae21e8f5423345d78028b5dfa2cc676",
          "url": "https://github.com/Rullst/Rullst/commit/154c455a671755e029d0d087b24c2f2d177274c2"
        },
        "date": 1788123826143,
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
            "value": 463,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 345,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 210,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2121416,
            "range": "± 135610",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 100129,
            "range": "± 1099",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 105643,
            "range": "± 1694",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 98676,
            "range": "± 2937",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 106290,
            "range": "± 1774",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 97886,
            "range": "± 1435",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 106509,
            "range": "± 1900",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 156925,
            "range": "± 3185",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 250012,
            "range": "± 7600",
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
          "id": "3a5aa1aaead3451f5fe449ed8abb3cee05609d10",
          "message": "feat(capital): add actix webhook middleware",
          "timestamp": "2026-08-30T18:06:54-03:00",
          "tree_id": "b7edd1476c801638d713ab87ad6bbc63599537a2",
          "url": "https://github.com/Rullst/Rullst/commit/3a5aa1aaead3451f5fe449ed8abb3cee05609d10"
        },
        "date": 1788124630302,
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
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 468,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 351,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 211,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2146378,
            "range": "± 161620",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 98113,
            "range": "± 933",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 103702,
            "range": "± 930",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 96554,
            "range": "± 2591",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105099,
            "range": "± 1126",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 95785,
            "range": "± 2362",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 105091,
            "range": "± 2142",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154699,
            "range": "± 3402",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 248987,
            "range": "± 3475",
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
          "id": "8a767d71dbdc789ee76a9d96ae94ba579b417f0f",
          "message": "feat(cli): harden billing scaffold",
          "timestamp": "2026-08-30T18:24:19-03:00",
          "tree_id": "226a187b7daeb6cdc9b9f854236f25f91a02a55c",
          "url": "https://github.com/Rullst/Rullst/commit/8a767d71dbdc789ee76a9d96ae94ba579b417f0f"
        },
        "date": 1788125352324,
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
            "value": 62,
            "range": "± 0",
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
            "value": 353,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 217,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1787292,
            "range": "± 109800",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 80309,
            "range": "± 1667",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 85020,
            "range": "± 1729",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 77523,
            "range": "± 1282",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 88328,
            "range": "± 1307",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 78143,
            "range": "± 1547",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 89388,
            "range": "± 1307",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 156536,
            "range": "± 2803",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 260875,
            "range": "± 3135",
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
          "id": "3ef2d55f1bef2c85886ee3a80fd2536b9a893f36",
          "message": "feat(capital): add bounded subscription handles",
          "timestamp": "2026-08-30T18:37:02-03:00",
          "tree_id": "b80079c17dad3d1474a61aba5ce1daa4d9a80c72",
          "url": "https://github.com/Rullst/Rullst/commit/3ef2d55f1bef2c85886ee3a80fd2536b9a893f36"
        },
        "date": 1788126410149,
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
            "value": 53,
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
            "value": 350,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 210,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1660577,
            "range": "± 98650",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 76845,
            "range": "± 1264",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 82166,
            "range": "± 820",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 75222,
            "range": "± 1029",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 85944,
            "range": "± 1362",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 76860,
            "range": "± 888",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 84099,
            "range": "± 1252",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155021,
            "range": "± 2394",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 261314,
            "range": "± 2694",
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
          "id": "fbec614909d9a5e414ccb9b9a219014b8e7440a8",
          "message": "fix(cli): render billing blueprint scaffold",
          "timestamp": "2026-08-30T19:09:40-03:00",
          "tree_id": "f4582fed0e26359f7bf222b8886dfa29c17f5748",
          "url": "https://github.com/Rullst/Rullst/commit/fbec614909d9a5e414ccb9b9a219014b8e7440a8"
        },
        "date": 1788128115927,
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 477,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 343,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 223,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2214177,
            "range": "± 238545",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 100205,
            "range": "± 1984",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 105928,
            "range": "± 1043",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 98351,
            "range": "± 1940",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105041,
            "range": "± 1571",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 96043,
            "range": "± 3039",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 104965,
            "range": "± 1610",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 152988,
            "range": "± 2310",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 245971,
            "range": "± 2497",
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
          "id": "8b34ca4127825b9ebf58ed23dea03b0be0eeb30c",
          "message": "feat(studio): add bounded data mutations",
          "timestamp": "2026-08-30T19:17:16-03:00",
          "tree_id": "dd255b1883cb644b275092b525cad2fd60ffde07",
          "url": "https://github.com/Rullst/Rullst/commit/8b34ca4127825b9ebf58ed23dea03b0be0eeb30c"
        },
        "date": 1788128539085,
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
            "value": 464,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 347,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 209,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2164814,
            "range": "± 92887",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 97885,
            "range": "± 874",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 102368,
            "range": "± 2234",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 97461,
            "range": "± 2287",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105278,
            "range": "± 2429",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 94954,
            "range": "± 2438",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 105006,
            "range": "± 2014",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 152023,
            "range": "± 1447",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 246960,
            "range": "± 3087",
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
          "id": "20dc06a6bb04ce67ecb6446ec2a47af097d67728",
          "message": "feat(security): enforce bounded json schemas",
          "timestamp": "2026-08-30T19:51:44-03:00",
          "tree_id": "763fb73ffb17615e48c124fdf6206a807a816b50",
          "url": "https://github.com/Rullst/Rullst/commit/20dc06a6bb04ce67ecb6446ec2a47af097d67728"
        },
        "date": 1788130597288,
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
            "value": 49,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 465,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 353,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 213,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1769206,
            "range": "± 128775",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 79502,
            "range": "± 1292",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 83954,
            "range": "± 2724",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 78689,
            "range": "± 1162",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 86346,
            "range": "± 2340",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 78936,
            "range": "± 1466",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 88907,
            "range": "± 2824",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154526,
            "range": "± 2458",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 260550,
            "range": "± 6954",
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
          "id": "9d664652023b08c79fe310524358ca00846c79bf",
          "message": "feat(ai): add durable chat memory",
          "timestamp": "2026-08-30T22:17:17-03:00",
          "tree_id": "71a1dced9a53a2c4b0d2bbedc9413507329d3ea7",
          "url": "https://github.com/Rullst/Rullst/commit/9d664652023b08c79fe310524358ca00846c79bf"
        },
        "date": 1788139331392,
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
            "value": 47,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 463,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 356,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 212,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1836857,
            "range": "± 93533",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 77248,
            "range": "± 2393",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 81068,
            "range": "± 1350",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 77747,
            "range": "± 944",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 85713,
            "range": "± 1532",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 78292,
            "range": "± 2573",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 88474,
            "range": "± 3173",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 156619,
            "range": "± 2687",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 264153,
            "range": "± 10434",
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
          "id": "139065ff9fb026cf705b13dbf29ab7dd230db426",
          "message": "feat(security): add bounded threat sentinel",
          "timestamp": "2026-08-30T22:38:36-03:00",
          "tree_id": "2dcff2f2ea710aaeed935df5e05c55402c40e233",
          "url": "https://github.com/Rullst/Rullst/commit/139065ff9fb026cf705b13dbf29ab7dd230db426"
        },
        "date": 1788140620707,
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
            "value": 49,
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
            "value": 346,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 212,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1656710,
            "range": "± 160274",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 76565,
            "range": "± 2001",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 81257,
            "range": "± 925",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 76372,
            "range": "± 1124",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 85023,
            "range": "± 905",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 76766,
            "range": "± 865",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 84903,
            "range": "± 1465",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 156898,
            "range": "± 2960",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 266476,
            "range": "± 3330",
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
          "id": "13f7a2a9fe6e0f47ef1a2da607fdcbad54eac81f",
          "message": "feat(capital): add bounded nfse protocol codec",
          "timestamp": "2026-08-30T23:28:07-03:00",
          "tree_id": "18c4c31f9427d78f0a0d6b1cdb8784010fdf8a6c",
          "url": "https://github.com/Rullst/Rullst/commit/13f7a2a9fe6e0f47ef1a2da607fdcbad54eac81f"
        },
        "date": 1788143838544,
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
            "value": 11,
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
            "value": 378,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 259,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 169,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2355788,
            "range": "± 2000577",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 76660,
            "range": "± 2576",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 63391,
            "range": "± 2802",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 59886,
            "range": "± 2205",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 73701,
            "range": "± 5549",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 68154,
            "range": "± 2127",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 84163,
            "range": "± 2962",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 144210,
            "range": "± 6706",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 266504,
            "range": "± 11145",
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
          "id": "4776f584b4f42b810fe6e7e955797207efbf6829",
          "message": "fix(ci): restore cargo deny dependency gate",
          "timestamp": "2026-08-31T00:19:14-03:00",
          "tree_id": "25579edf808ed3bdb177973826d6d5f653474ae7",
          "url": "https://github.com/Rullst/Rullst/commit/4776f584b4f42b810fe6e7e955797207efbf6829"
        },
        "date": 1788146669357,
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
            "value": 47,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 462,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 351,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 213,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2291843,
            "range": "± 175885",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 102143,
            "range": "± 1501",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 106370,
            "range": "± 906",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 100498,
            "range": "± 2461",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 107591,
            "range": "± 1969",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 102534,
            "range": "± 1903",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 109589,
            "range": "± 2102",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155230,
            "range": "± 2369",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 248032,
            "range": "± 4624",
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
          "id": "afcc8bb1cf4edb2711c09d508bcfbbb934765b79",
          "message": "feat(core): add versioned client contract",
          "timestamp": "2026-08-31T00:41:26-03:00",
          "tree_id": "7bbe6e2a217d52dd1f6575b964045fa920c7e9c3",
          "url": "https://github.com/Rullst/Rullst/commit/afcc8bb1cf4edb2711c09d508bcfbbb934765b79"
        },
        "date": 1788148269589,
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
            "value": 49,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 464,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 351,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 210,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1649067,
            "range": "± 98021",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 76811,
            "range": "± 1402",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 81437,
            "range": "± 764",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 76319,
            "range": "± 1156",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 85802,
            "range": "± 924",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 76836,
            "range": "± 1245",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 85854,
            "range": "± 1085",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 156697,
            "range": "± 3481",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 259228,
            "range": "± 3480",
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
          "id": "3a55e726184ae7377ccad40434ed183c8d9b66bb",
          "message": "fix(ci): propagate benchmark failures",
          "timestamp": "2026-08-31T01:04:09-03:00",
          "tree_id": "e33c307da84da8701727162155f691e5cbdc7860",
          "url": "https://github.com/Rullst/Rullst/commit/3a55e726184ae7377ccad40434ed183c8d9b66bb"
        },
        "date": 1788149320216,
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
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 363,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 269,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 177,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2250640,
            "range": "± 2691450",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 61100,
            "range": "± 1370",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 63943,
            "range": "± 1225",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 60685,
            "range": "± 1031",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 65558,
            "range": "± 1012",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 59860,
            "range": "± 1129",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 66986,
            "range": "± 1166",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 120283,
            "range": "± 1603",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 199341,
            "range": "± 2359",
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
          "id": "f94d500f146cc9f172f63b14dee94ce59108c74f",
          "message": "fix(auth): use strong benchmark app key",
          "timestamp": "2026-08-31T01:23:42-03:00",
          "tree_id": "e2a39d8747d48cb226f949a6c7b336436d67190f",
          "url": "https://github.com/Rullst/Rullst/commit/f94d500f146cc9f172f63b14dee94ce59108c74f"
        },
        "date": 1788150501276,
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
            "value": 462,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 361,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 210,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2268093,
            "range": "± 129626",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 102543,
            "range": "± 1073",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 107747,
            "range": "± 1251",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 100194,
            "range": "± 2637",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 107860,
            "range": "± 1735",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 99727,
            "range": "± 2306",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 108421,
            "range": "± 1656",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155953,
            "range": "± 1883",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 250263,
            "range": "± 2520",
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
          "id": "14fb9f86401600aa6c028eb97059b049f203582c",
          "message": "feat(core): add bounded offline sync foundation",
          "timestamp": "2026-08-31T01:40:55-03:00",
          "tree_id": "90e893c1afffb96ff7163c155777eeac93d28a23",
          "url": "https://github.com/Rullst/Rullst/commit/14fb9f86401600aa6c028eb97059b049f203582c"
        },
        "date": 1788151840327,
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
            "value": 47,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 469,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 352,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 211,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2142369,
            "range": "± 183916",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 99622,
            "range": "± 1657",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 105310,
            "range": "± 1085",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 97526,
            "range": "± 2006",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105371,
            "range": "± 1122",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 96880,
            "range": "± 2513",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 105410,
            "range": "± 1332",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153778,
            "range": "± 1936",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 250352,
            "range": "± 2894",
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
          "id": "04547b0ca776419ef25629ba37da464ae55cc692",
          "message": "feat(core): orchestrate bounded offline sync",
          "timestamp": "2026-08-31T01:53:48-03:00",
          "tree_id": "c2971b9c06c7387d240b8f43a5cf0744d267f3cf",
          "url": "https://github.com/Rullst/Rullst/commit/04547b0ca776419ef25629ba37da464ae55cc692"
        },
        "date": 1788152339104,
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
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 462,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 348,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 212,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2164912,
            "range": "± 114193",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 103544,
            "range": "± 1481",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 109253,
            "range": "± 1681",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 103137,
            "range": "± 2103",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 108524,
            "range": "± 1717",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 101140,
            "range": "± 2302",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 109385,
            "range": "± 2130",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 157555,
            "range": "± 1269",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 254437,
            "range": "± 2450",
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
          "id": "d8f46faaa2a4512fe5354e18e21ca1ce81ed97c6",
          "message": "feat(academy): make activity scoring authoritative",
          "timestamp": "2026-08-31T02:22:37-03:00",
          "tree_id": "2be4b5a3112e09d85cd227bcab17cd885572c32f",
          "url": "https://github.com/Rullst/Rullst/commit/d8f46faaa2a4512fe5354e18e21ca1ce81ed97c6"
        },
        "date": 1788154066557,
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
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 459,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 346,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 211,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2116346,
            "range": "± 98404",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 100235,
            "range": "± 1377",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 106125,
            "range": "± 3687",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 97279,
            "range": "± 3087",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 106819,
            "range": "± 1465",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 98068,
            "range": "± 2065",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 107239,
            "range": "± 2191",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 156183,
            "range": "± 2350",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 252680,
            "range": "± 4465",
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
          "id": "859d8bc84131c195ee71a94269c3680e9fb77c5e",
          "message": "feat(academy): persist authoritative activity scores",
          "timestamp": "2026-08-31T02:46:44-03:00",
          "tree_id": "225b8d34cc33cf19058a54cf2f660d8058c49278",
          "url": "https://github.com/Rullst/Rullst/commit/859d8bc84131c195ee71a94269c3680e9fb77c5e"
        },
        "date": 1788155795883,
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
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 462,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 348,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 214,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2179457,
            "range": "± 123975",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 102119,
            "range": "± 1475",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 107861,
            "range": "± 890",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 101594,
            "range": "± 2311",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 107515,
            "range": "± 1712",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 99673,
            "range": "± 2043",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 108650,
            "range": "± 1325",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154711,
            "range": "± 3351",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 244927,
            "range": "± 3890",
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
          "id": "57eb6abb6317fe4d503fd99ebb11664b44ad9fb5",
          "message": "feat(academy): add durable activity submissions",
          "timestamp": "2026-08-31T03:25:58-03:00",
          "tree_id": "80a360bed4939c949533cad25d22ce8a5eb45adf",
          "url": "https://github.com/Rullst/Rullst/commit/57eb6abb6317fe4d503fd99ebb11664b44ad9fb5"
        },
        "date": 1788157860651,
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
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 420,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 330,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 198,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1667561,
            "range": "± 90268",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 66501,
            "range": "± 1156",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 74747,
            "range": "± 2192",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 65434,
            "range": "± 3034",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 75900,
            "range": "± 1397",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 66209,
            "range": "± 1116",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 76575,
            "range": "± 1805",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 148141,
            "range": "± 16138",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 265751,
            "range": "± 4487",
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
          "id": "0d0c1128f0f254a0706b7da4004e5dbd8241b3f7",
          "message": "feat(academy): add authoritative matching exercises",
          "timestamp": "2026-08-31T03:39:31-03:00",
          "tree_id": "1298b233f6b9c1130dd7d141c83258173c1b3a51",
          "url": "https://github.com/Rullst/Rullst/commit/0d0c1128f0f254a0706b7da4004e5dbd8241b3f7"
        },
        "date": 1788158660401,
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
            "value": 459,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 343,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 211,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2111509,
            "range": "± 137844",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 89934,
            "range": "± 1486",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 94432,
            "range": "± 1208",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 90059,
            "range": "± 3998",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 98852,
            "range": "± 1466",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 88970,
            "range": "± 2541",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 99095,
            "range": "± 1681",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 143758,
            "range": "± 2042",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 243484,
            "range": "± 4572",
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
          "id": "188a27b611a88f8b2697757f226acd11a57cf47a",
          "message": "feat(academy): add authoritative typed recall",
          "timestamp": "2026-08-31T03:59:04-03:00",
          "tree_id": "22558fa92def02bbc5a48b876bd8f3a289eacf01",
          "url": "https://github.com/Rullst/Rullst/commit/188a27b611a88f8b2697757f226acd11a57cf47a"
        },
        "date": 1788160141380,
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
            "value": 50,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 462,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 352,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 209,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1654271,
            "range": "± 84641",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 77368,
            "range": "± 1001",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 82288,
            "range": "± 1022",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 76779,
            "range": "± 1297",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 86770,
            "range": "± 2014",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 77988,
            "range": "± 997",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 86727,
            "range": "± 1617",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154678,
            "range": "± 2564",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 258436,
            "range": "± 2824",
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
          "id": "18e58b940a430f78593b825806295a85ee67f9ec",
          "message": "feat(academy): add deterministic review scheduling",
          "timestamp": "2026-08-31T04:22:24-03:00",
          "tree_id": "c61ec69b9df27995660fa5a33c7864ffb9d06cce",
          "url": "https://github.com/Rullst/Rullst/commit/18e58b940a430f78593b825806295a85ee67f9ec"
        },
        "date": 1788161242514,
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
            "value": 50,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 457,
            "range": "± 1",
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
            "value": 212,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1675445,
            "range": "± 169770",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 77418,
            "range": "± 1290",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 82019,
            "range": "± 1435",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 75722,
            "range": "± 1471",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 85856,
            "range": "± 826",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 76693,
            "range": "± 1132",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 85663,
            "range": "± 1481",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154189,
            "range": "± 2503",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 260485,
            "range": "± 3860",
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
          "id": "31644c3cc11b864db36f8ed7f29d2428492151d5",
          "message": "test(orm): add reproducible comparison benchmarks",
          "timestamp": "2026-08-31T04:59:22-03:00",
          "tree_id": "fcf44bb02468847d0c6495ead156b15e00f91856",
          "url": "https://github.com/Rullst/Rullst/commit/31644c3cc11b864db36f8ed7f29d2428492151d5"
        },
        "date": 1788163920285,
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
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 414,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 323,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 198,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2514434,
            "range": "± 290806",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 62953,
            "range": "± 1334",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 67394,
            "range": "± 2088",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 63727,
            "range": "± 1425",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 76399,
            "range": "± 2119",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 63353,
            "range": "± 1189",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 74958,
            "range": "± 2211",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 146506,
            "range": "± 2215",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 259022,
            "range": "± 4917",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 107173,
            "range": "± 6512",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 6842,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 117034,
            "range": "± 3263",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 109178,
            "range": "± 3011",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 7611,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 122003,
            "range": "± 4309",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 108354,
            "range": "± 3962",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 1437,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 107317,
            "range": "± 3163",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 113357,
            "range": "± 3220",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 10269,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 118430,
            "range": "± 4302",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 422773,
            "range": "± 22671",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 49837,
            "range": "± 5837",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 262388,
            "range": "± 13093",
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
          "id": "45798985404c430d17ceda81b9afb1ba1fe9fe7b",
          "message": "feat(connect): add signed local oidc fixture",
          "timestamp": "2026-08-31T06:45:08-03:00",
          "tree_id": "05e5646046e36859d7ccab14825f99da1ff146e8",
          "url": "https://github.com/Rullst/Rullst/commit/45798985404c430d17ceda81b9afb1ba1fe9fe7b"
        },
        "date": 1788170345787,
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
            "value": 459,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 347,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 212,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2197497,
            "range": "± 100525",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 99342,
            "range": "± 1461",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 103648,
            "range": "± 1475",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 96809,
            "range": "± 2226",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 104967,
            "range": "± 1437",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 96182,
            "range": "± 1684",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 103785,
            "range": "± 4650",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154149,
            "range": "± 3117",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 241501,
            "range": "± 3847",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 132598,
            "range": "± 2701",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8350,
            "range": "± 104",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 145213,
            "range": "± 7551",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 133299,
            "range": "± 6685",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9171,
            "range": "± 89",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 146838,
            "range": "± 6321",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 132198,
            "range": "± 6208",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2795,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 131517,
            "range": "± 4745",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 135660,
            "range": "± 4029",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12193,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 150177,
            "range": "± 7388",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 541665,
            "range": "± 32664",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 49090,
            "range": "± 1112",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 326982,
            "range": "± 12863",
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
          "id": "ae17f7b19bef8b7fcb2dff18822d4fa707f70bda",
          "message": "feat(nexus): validate semantic admin forms",
          "timestamp": "2026-08-31T07:13:16-03:00",
          "tree_id": "e46c4a906b4add5b5894561a2a0985995119da55",
          "url": "https://github.com/Rullst/Rullst/commit/ae17f7b19bef8b7fcb2dff18822d4fa707f70bda"
        },
        "date": 1788171620699,
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
            "value": 459,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 353,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 211,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2096537,
            "range": "± 174456",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 98793,
            "range": "± 1512",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 104723,
            "range": "± 1373",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 96697,
            "range": "± 1981",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 104446,
            "range": "± 1242",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 95919,
            "range": "± 1911",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 106184,
            "range": "± 911",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155195,
            "range": "± 3474",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 249882,
            "range": "± 6882",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 128170,
            "range": "± 6398",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8200,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 144174,
            "range": "± 9480",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 129131,
            "range": "± 6194",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9043,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 144214,
            "range": "± 4238",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 127592,
            "range": "± 4572",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2769,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 133381,
            "range": "± 4629",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 135112,
            "range": "± 4063",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12382,
            "range": "± 54",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 145946,
            "range": "± 7062",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 509535,
            "range": "± 27490",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 48844,
            "range": "± 1336",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 327691,
            "range": "± 16551",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Venelouis",
            "username": "venelouis",
            "email": "venelouistyago@gmail.com"
          },
          "committer": {
            "name": "Venelouis",
            "username": "venelouis",
            "email": "venelouistyago@gmail.com"
          },
          "id": "e2eb03514e451be41f1f4b807c9a6cb5bba28a68",
          "message": "feat(capital): add bounded direct charges",
          "timestamp": "2026-08-31T10:41:41Z",
          "url": "https://github.com/Rullst/Rullst/commit/e2eb03514e451be41f1f4b807c9a6cb5bba28a68"
        },
        "date": 1788173406970,
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
            "value": 38,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 357,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 283,
            "range": "± 3",
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
            "value": 2052183,
            "range": "± 376400",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 58828,
            "range": "± 888",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 63460,
            "range": "± 1461",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 58738,
            "range": "± 947",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 66885,
            "range": "± 1516",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 60394,
            "range": "± 849",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 66090,
            "range": "± 1077",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 121134,
            "range": "± 1509",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 202030,
            "range": "± 3105",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 85818,
            "range": "± 2571",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 6932,
            "range": "± 123",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 94532,
            "range": "± 2528",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 86122,
            "range": "± 4144",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 7491,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 96150,
            "range": "± 1853",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 87107,
            "range": "± 1868",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2356,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 87222,
            "range": "± 2239",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 90015,
            "range": "± 2334",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 9903,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 93263,
            "range": "± 3174",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 389169,
            "range": "± 311321",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 76464,
            "range": "± 17585",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 249890,
            "range": "± 42307",
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
          "id": "75fd320af2cfb300b9e53eb0fcf16017e90a2579",
          "message": "test(capital): assert receipt email redaction",
          "timestamp": "2026-08-31T08:29:39-03:00",
          "tree_id": "7d2072a41103f59b700c884cabcacc9937f749b5",
          "url": "https://github.com/Rullst/Rullst/commit/75fd320af2cfb300b9e53eb0fcf16017e90a2579"
        },
        "date": 1788176511976,
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
            "value": 11,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 40,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 377,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 260,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 167,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2729464,
            "range": "± 9485387",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 64722,
            "range": "± 1914",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 64478,
            "range": "± 1610",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 60641,
            "range": "± 887",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 74196,
            "range": "± 2598",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 62344,
            "range": "± 1294",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 70193,
            "range": "± 786",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 140601,
            "range": "± 5037",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 261728,
            "range": "± 9186",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 114959,
            "range": "± 3223",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 5580,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 124950,
            "range": "± 6767",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 115168,
            "range": "± 4484",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 6173,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 124038,
            "range": "± 5457",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 114535,
            "range": "± 3688",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 1189,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 117029,
            "range": "± 6203",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 121427,
            "range": "± 4529",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 8852,
            "range": "± 61",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 123648,
            "range": "± 3713",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 518953,
            "range": "± 584668",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 109951,
            "range": "± 309776",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 453249,
            "range": "± 1008240",
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
          "id": "4eb91eecd9cd5f541e3e8686ad71a72eba2d4a4b",
          "message": "feat(capital): add provider-specific metered billing",
          "timestamp": "2026-08-31T09:09:35-03:00",
          "tree_id": "98d4e34fb3e37c579bfb88e461ff43fdd5498c7e",
          "url": "https://github.com/Rullst/Rullst/commit/4eb91eecd9cd5f541e3e8686ad71a72eba2d4a4b"
        },
        "date": 1788178940566,
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
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 414,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 323,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 197,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1547550,
            "range": "± 41735",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 64224,
            "range": "± 1867",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 69495,
            "range": "± 1538",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 63824,
            "range": "± 1087",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 73742,
            "range": "± 1053",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 65035,
            "range": "± 1095",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 74610,
            "range": "± 1175",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 144206,
            "range": "± 2404",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 252662,
            "range": "± 6968",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 110014,
            "range": "± 6046",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 6786,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 120761,
            "range": "± 3098",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 111521,
            "range": "± 3473",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 7431,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 123825,
            "range": "± 5222",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 110723,
            "range": "± 2877",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 1371,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 111351,
            "range": "± 3814",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 115386,
            "range": "± 4895",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 10294,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 120176,
            "range": "± 4464",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 403195,
            "range": "± 14940",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 50919,
            "range": "± 6397",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 262474,
            "range": "± 14848",
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
          "id": "40de8fb554e5bb837bc0daf43bc77d7ced2ac26e",
          "message": "feat(capital): add shared transactional quotas",
          "timestamp": "2026-08-31T11:20:53-03:00",
          "tree_id": "44ae582dc622e45242c2e96f9b297e6696fdf4d2",
          "url": "https://github.com/Rullst/Rullst/commit/40de8fb554e5bb837bc0daf43bc77d7ced2ac26e"
        },
        "date": 1788186546051,
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
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 351,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 225,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2154971,
            "range": "± 144455",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 103148,
            "range": "± 1450",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 109179,
            "range": "± 1779",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 100224,
            "range": "± 2130",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 107896,
            "range": "± 2720",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 101475,
            "range": "± 1905",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 107417,
            "range": "± 1504",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155210,
            "range": "± 1476",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 250137,
            "range": "± 8102",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 133671,
            "range": "± 7173",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8457,
            "range": "± 54",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 149912,
            "range": "± 5252",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 135075,
            "range": "± 5464",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9167,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 152484,
            "range": "± 6212",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 134124,
            "range": "± 5114",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2818,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 134584,
            "range": "± 5992",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 139541,
            "range": "± 6804",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12233,
            "range": "± 55",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 150343,
            "range": "± 8355",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 561724,
            "range": "± 34561",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 49715,
            "range": "± 3112",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 339690,
            "range": "± 29639",
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
          "id": "fab3e03debb92286b67f0028c0ddf9a8bcc3dd5e",
          "message": "feat(capital): complete coupon and trial contracts",
          "timestamp": "2026-08-31T12:07:39-03:00",
          "tree_id": "5bb23192d241b47537d93c14e91e7987b7404c01",
          "url": "https://github.com/Rullst/Rullst/commit/fab3e03debb92286b67f0028c0ddf9a8bcc3dd5e"
        },
        "date": 1788189657253,
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
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 457,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 354,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 214,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2360613,
            "range": "± 194675",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 100647,
            "range": "± 1103",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 105022,
            "range": "± 1327",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 98678,
            "range": "± 2535",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 106280,
            "range": "± 1553",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 97600,
            "range": "± 3507",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 106415,
            "range": "± 1733",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155167,
            "range": "± 2025",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 250504,
            "range": "± 2414",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 129332,
            "range": "± 5155",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8234,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 142773,
            "range": "± 6133",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 132128,
            "range": "± 5193",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9164,
            "range": "± 69",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 146972,
            "range": "± 5879",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 129050,
            "range": "± 7287",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2775,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 133115,
            "range": "± 3678",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 133801,
            "range": "± 5567",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12051,
            "range": "± 109",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 148555,
            "range": "± 4221",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 548216,
            "range": "± 25462",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 49836,
            "range": "± 2117",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 330978,
            "range": "± 15910",
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
          "id": "ae85037b921a574675b3fdc9b74355ca150a9ee6",
          "message": "feat(connect): coordinate automatic token refresh",
          "timestamp": "2026-08-31T12:42:35-03:00",
          "tree_id": "c6776549166ca761ac8893140312e8a08826f786",
          "url": "https://github.com/Rullst/Rullst/commit/ae85037b921a574675b3fdc9b74355ca150a9ee6"
        },
        "date": 1788191435972,
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
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 460,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 349,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 220,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2079436,
            "range": "± 70947",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 100068,
            "range": "± 1127",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 105729,
            "range": "± 1265",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 98869,
            "range": "± 2212",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 106118,
            "range": "± 1580",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 97671,
            "range": "± 2778",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 106335,
            "range": "± 1805",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154926,
            "range": "± 4165",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 248512,
            "range": "± 2991",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 131032,
            "range": "± 3205",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8372,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 145546,
            "range": "± 5266",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 133544,
            "range": "± 5101",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9256,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 147311,
            "range": "± 5219",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 131650,
            "range": "± 6042",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2820,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 130614,
            "range": "± 3737",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 137155,
            "range": "± 5284",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12239,
            "range": "± 75",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 148015,
            "range": "± 6927",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 526678,
            "range": "± 29062",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 49151,
            "range": "± 1343",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 330336,
            "range": "± 18744",
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
          "id": "8546cc0d7dcd507f6178378cb730c92050a9cf0f",
          "message": "docs(roadmap): quantify scope through v13",
          "timestamp": "2026-08-31T13:47:25-03:00",
          "tree_id": "47eda06423c7f071ea107a36a7e52f1f9ea7f108",
          "url": "https://github.com/Rullst/Rullst/commit/8546cc0d7dcd507f6178378cb730c92050a9cf0f"
        },
        "date": 1788195362419,
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
            "value": 51,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 461,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 348,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 208,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1957851,
            "range": "± 156712",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 76842,
            "range": "± 912",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 81741,
            "range": "± 686",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 76465,
            "range": "± 1394",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 86090,
            "range": "± 1040",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 76632,
            "range": "± 1141",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 86459,
            "range": "± 1096",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154462,
            "range": "± 2929",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 253990,
            "range": "± 3977",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 107866,
            "range": "± 2199",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8928,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 118241,
            "range": "± 2145",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 108719,
            "range": "± 2172",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9564,
            "range": "± 96",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 120121,
            "range": "± 2585",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 109025,
            "range": "± 1862",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3305,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 108993,
            "range": "± 3062",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 113210,
            "range": "± 1620",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12842,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 121028,
            "range": "± 4695",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 432442,
            "range": "± 17548",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 50823,
            "range": "± 1234",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 273322,
            "range": "± 14241",
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
          "id": "b7efccc14bb05ea46fda842e1822b9f2742ee28e",
          "message": "feat(messaging): add bounded broker foundation",
          "timestamp": "2026-08-31T15:29:38-03:00",
          "tree_id": "ef79dc5cff1c39ddbb4e30fe6d8b4ab72e633ccf",
          "url": "https://github.com/Rullst/Rullst/commit/b7efccc14bb05ea46fda842e1822b9f2742ee28e"
        },
        "date": 1788201775663,
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
            "value": 38,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 365,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 270,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 179,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2018575,
            "range": "± 194256",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 59950,
            "range": "± 1040",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 63657,
            "range": "± 856",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 59921,
            "range": "± 914",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 67037,
            "range": "± 1155",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 60034,
            "range": "± 1752",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 67890,
            "range": "± 2124",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 124580,
            "range": "± 1727",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 212876,
            "range": "± 3341",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 83752,
            "range": "± 1773",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 6892,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 92335,
            "range": "± 1957",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 83320,
            "range": "± 1479",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 7387,
            "range": "± 142",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 93361,
            "range": "± 1628",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 84882,
            "range": "± 1546",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2599,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 86304,
            "range": "± 1608",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 89063,
            "range": "± 1693",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 9917,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 92475,
            "range": "± 4112",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 422968,
            "range": "± 281623",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 112682,
            "range": "± 58402",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 363895,
            "range": "± 261080",
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
          "id": "53a74af35e471a3810dfbc1561b16fb26c8ac0ec",
          "message": "feat(mail): bound attachment delivery",
          "timestamp": "2026-08-31T16:34:09-03:00",
          "tree_id": "b2339c67baed4da25698ebe7a2374de6a380e168",
          "url": "https://github.com/Rullst/Rullst/commit/53a74af35e471a3810dfbc1561b16fb26c8ac0ec"
        },
        "date": 1788205680079,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 8,
            "range": "± 1",
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
            "value": 425,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 296,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 198,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2906461,
            "range": "± 757026",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 66584,
            "range": "± 1281",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 72618,
            "range": "± 1724",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 78846,
            "range": "± 4909",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 77664,
            "range": "± 1797",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 70014,
            "range": "± 2785",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 77804,
            "range": "± 1946",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 161074,
            "range": "± 3389",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 288480,
            "range": "± 9365",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 125213,
            "range": "± 4342",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 6445,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 134366,
            "range": "± 6531",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 126246,
            "range": "± 4153",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 7050,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 129835,
            "range": "± 4595",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 126440,
            "range": "± 7160",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 1339,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 127420,
            "range": "± 6929",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 132569,
            "range": "± 4723",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 9802,
            "range": "± 66",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 137632,
            "range": "± 4378",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 559541,
            "range": "± 93307",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 114063,
            "range": "± 18276",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 394633,
            "range": "± 97506",
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
          "id": "90fb4f0cc540d3ca017c48d60821cf3e8bb4f167",
          "message": "feat(connect): add typed token revocation",
          "timestamp": "2026-08-31T17:43:51-03:00",
          "tree_id": "14f9bcbd20ea11daa9a7a2396d9a42aae389343a",
          "url": "https://github.com/Rullst/Rullst/commit/90fb4f0cc540d3ca017c48d60821cf3e8bb4f167"
        },
        "date": 1788209468963,
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
            "value": 9,
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
            "value": 303,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 213,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 130,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 4758893,
            "range": "± 92037744",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 60696,
            "range": "± 1573",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 65167,
            "range": "± 1334",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 61340,
            "range": "± 1597",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 68686,
            "range": "± 1983",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 62104,
            "range": "± 2395",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 71596,
            "range": "± 2712",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 129845,
            "range": "± 5708",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 218887,
            "range": "± 14549",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 96622,
            "range": "± 3050",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 4541,
            "range": "± 225",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 102712,
            "range": "± 2444",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 97819,
            "range": "± 1956",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 4983,
            "range": "± 123",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 105501,
            "range": "± 8785",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 97052,
            "range": "± 4503",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 957,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 98258,
            "range": "± 5152",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 103435,
            "range": "± 1413",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 7048,
            "range": "± 294",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 103765,
            "range": "± 1929",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 597630,
            "range": "± 678832",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 288831,
            "range": "± 403189",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 934578,
            "range": "± 513816",
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
          "id": "4453782a995e8f9b648eea80a1bd00819369f965",
          "message": "feat(orm): add auditable revision restore",
          "timestamp": "2026-08-31T20:18:06-03:00",
          "tree_id": "b7949e65930ef4fee994a45bc3cc9427923ee425",
          "url": "https://github.com/Rullst/Rullst/commit/4453782a995e8f9b648eea80a1bd00819369f965"
        },
        "date": 1788219027239,
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
            "value": 375,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 211,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2093842,
            "range": "± 101789",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 99521,
            "range": "± 1419",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 104323,
            "range": "± 1405",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 96754,
            "range": "± 3193",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105173,
            "range": "± 1393",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 95644,
            "range": "± 2599",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 105496,
            "range": "± 2495",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153212,
            "range": "± 4268",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 246806,
            "range": "± 4358",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 131028,
            "range": "± 5776",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8277,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 146184,
            "range": "± 4052",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 132463,
            "range": "± 4052",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9153,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 148761,
            "range": "± 3878",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 128851,
            "range": "± 5172",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2801,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 130978,
            "range": "± 4195",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 136025,
            "range": "± 5095",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12183,
            "range": "± 42",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 146093,
            "range": "± 6285",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 543409,
            "range": "± 36519",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 49685,
            "range": "± 1199",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 331091,
            "range": "± 21803",
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
          "id": "58d03a9e9d82c48ad378a3b3b3d5691f31a06cbe",
          "message": "ci(scorecard): publish authenticated results",
          "timestamp": "2026-08-31T22:59:36-03:00",
          "tree_id": "0c0a84d459eabb0dc1f5074ce6edb44fe3df249c",
          "url": "https://github.com/Rullst/Rullst/commit/58d03a9e9d82c48ad378a3b3b3d5691f31a06cbe"
        },
        "date": 1788228422472,
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
            "value": 44,
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
            "value": 374,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 209,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2185340,
            "range": "± 135424",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 100861,
            "range": "± 1661",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 106097,
            "range": "± 1537",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 100163,
            "range": "± 1974",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 106971,
            "range": "± 1264",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 98914,
            "range": "± 1455",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 107695,
            "range": "± 2475",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154384,
            "range": "± 2812",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 250225,
            "range": "± 2509",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 131352,
            "range": "± 5272",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8070,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 146954,
            "range": "± 5866",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 132214,
            "range": "± 7096",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 8928,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 151799,
            "range": "± 6577",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 132591,
            "range": "± 4833",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2695,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 133774,
            "range": "± 4258",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 135518,
            "range": "± 8762",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 11972,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 151689,
            "range": "± 7629",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 533115,
            "range": "± 31239",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 48368,
            "range": "± 1819",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 336566,
            "range": "± 14475",
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
          "id": "04006e822c7c7f7482bb5c29e097efec9e678b40",
          "message": "test(messaging): expand bounded input coverage",
          "timestamp": "2026-08-31T23:38:03-03:00",
          "tree_id": "0a85cd2cd6eda95e697de84dfb9a68927f4c9995",
          "url": "https://github.com/Rullst/Rullst/commit/04006e822c7c7f7482bb5c29e097efec9e678b40"
        },
        "date": 1788231029979,
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
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 370,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 223,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2453160,
            "range": "± 211881",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 98986,
            "range": "± 1294",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 104839,
            "range": "± 998",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 97903,
            "range": "± 2090",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105697,
            "range": "± 1667",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 95703,
            "range": "± 2194",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 106057,
            "range": "± 1420",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154266,
            "range": "± 1451",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 250719,
            "range": "± 6519",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 132122,
            "range": "± 5151",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8317,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 143607,
            "range": "± 5445",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 130316,
            "range": "± 3777",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9241,
            "range": "± 81",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 146276,
            "range": "± 5428",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 129561,
            "range": "± 4629",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2803,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 132927,
            "range": "± 4943",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 133996,
            "range": "± 7171",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12481,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 147653,
            "range": "± 6772",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 542479,
            "range": "± 30262",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 50746,
            "range": "± 1612",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 331515,
            "range": "± 11256",
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
          "id": "4fc63e9e25276e5f7e1859a9e255fadc74a6f85d",
          "message": "docs(release): record v12 feature freeze",
          "timestamp": "2026-09-01T00:06:25-03:00",
          "tree_id": "062fe6582f8b168370611e82cd6eff9ed9cc58c9",
          "url": "https://github.com/Rullst/Rullst/commit/4fc63e9e25276e5f7e1859a9e255fadc74a6f85d"
        },
        "date": 1788232455701,
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
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 50,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 527,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 387,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 211,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1770329,
            "range": "± 80009",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 77777,
            "range": "± 1282",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 82231,
            "range": "± 590",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 76829,
            "range": "± 1016",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 85206,
            "range": "± 1720",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 77634,
            "range": "± 919",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 86844,
            "range": "± 796",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154373,
            "range": "± 2580",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 253893,
            "range": "± 4108",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 106454,
            "range": "± 2177",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8865,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 119539,
            "range": "± 2744",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 107812,
            "range": "± 2305",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9374,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 122185,
            "range": "± 3501",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 109386,
            "range": "± 1947",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3356,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 109691,
            "range": "± 1962",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 113222,
            "range": "± 2316",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12773,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 118684,
            "range": "± 3606",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 442010,
            "range": "± 18904",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 52284,
            "range": "± 894",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 275452,
            "range": "± 11578",
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
          "id": "02f9f753d4276a06cad967a2141f5616a196e0b2",
          "message": "docs(release): make v13 the next feature line",
          "timestamp": "2026-09-01T00:53:28-03:00",
          "tree_id": "de0ab5614ce15a97cf85c96fa772cf3998624bea",
          "url": "https://github.com/Rullst/Rullst/commit/02f9f753d4276a06cad967a2141f5616a196e0b2"
        },
        "date": 1788235564291,
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
            "value": 469,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 375,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 209,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2083207,
            "range": "± 80241",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 98469,
            "range": "± 1313",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 103954,
            "range": "± 952",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 96790,
            "range": "± 2929",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105176,
            "range": "± 2067",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 96228,
            "range": "± 1648",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 105248,
            "range": "± 950",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155477,
            "range": "± 1499",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 250847,
            "range": "± 3107",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 127763,
            "range": "± 7501",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8408,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 141947,
            "range": "± 5527",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 128654,
            "range": "± 5049",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9172,
            "range": "± 90",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 147818,
            "range": "± 5424",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 126472,
            "range": "± 5249",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2797,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 132469,
            "range": "± 4528",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 135692,
            "range": "± 5668",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12319,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 146929,
            "range": "± 7503",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 520324,
            "range": "± 25953",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 49825,
            "range": "± 1086",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 323645,
            "range": "± 19254",
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
          "id": "8b93a4d397fba07d346b0b04d649e155c4163fb2",
          "message": "fix(security): generate recovery salts directly",
          "timestamp": "2026-09-01T01:10:07-03:00",
          "tree_id": "f5c5abb55243e8b511f143ee9f5b09e24606ae05",
          "url": "https://github.com/Rullst/Rullst/commit/8b93a4d397fba07d346b0b04d649e155c4163fb2"
        },
        "date": 1788236597008,
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
            "value": 469,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 377,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 209,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2413149,
            "range": "± 179038",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 98795,
            "range": "± 1494",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 104692,
            "range": "± 1381",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 98343,
            "range": "± 2574",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105777,
            "range": "± 1383",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 96725,
            "range": "± 2495",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 105772,
            "range": "± 1518",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154110,
            "range": "± 2344",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 243724,
            "range": "± 4070",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 128334,
            "range": "± 5287",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 7999,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 143394,
            "range": "± 3555",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 129415,
            "range": "± 7402",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9135,
            "range": "± 105",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 148660,
            "range": "± 5540",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 129765,
            "range": "± 6101",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2784,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 130443,
            "range": "± 4690",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 135664,
            "range": "± 4365",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12147,
            "range": "± 177",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 149736,
            "range": "± 7067",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 534104,
            "range": "± 26014",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 49863,
            "range": "± 2325",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 321184,
            "range": "± 15160",
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
          "id": "c01e7b183ef56438fe271165d669ea5227304931",
          "message": "docs(api): add runnable REST quickstart",
          "timestamp": "2026-09-01T01:37:49-03:00",
          "tree_id": "3259948933367b79137e9159090e766245934254",
          "url": "https://github.com/Rullst/Rullst/commit/c01e7b183ef56438fe271165d669ea5227304931"
        },
        "date": 1788237891531,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 5,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 8,
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
            "value": 333,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 233,
            "range": "± 16",
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
            "value": 5866851,
            "range": "± 37759634",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 60671,
            "range": "± 2105",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 68018,
            "range": "± 2677",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 60224,
            "range": "± 1572",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 70817,
            "range": "± 2459",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 63332,
            "range": "± 1825",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 72487,
            "range": "± 3587",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 132372,
            "range": "± 6447",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 240173,
            "range": "± 13430",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 155624,
            "range": "± 29224",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 4513,
            "range": "± 81",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 166858,
            "range": "± 22236",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 166359,
            "range": "± 17662",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 4957,
            "range": "± 222",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 169134,
            "range": "± 16048",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 162365,
            "range": "± 17691",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 936,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 166999,
            "range": "± 16212",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 151598,
            "range": "± 24984",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 7246,
            "range": "± 180",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 175893,
            "range": "± 33986",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 714299,
            "range": "± 533279",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 288215,
            "range": "± 287201",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 768209,
            "range": "± 747480",
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
          "id": "1bb6537ff58c5caba796415ff330d12f5bb1e32f",
          "message": "test(runtime): exercise default feature contracts",
          "timestamp": "2026-09-01T03:35:42-03:00",
          "tree_id": "453cf3c74e339ab6a9b2ae185fc49c3e4b287bd5",
          "url": "https://github.com/Rullst/Rullst/commit/1bb6537ff58c5caba796415ff330d12f5bb1e32f"
        },
        "date": 1788245323436,
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
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 47,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 522,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 379,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 216,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1675258,
            "range": "± 58032",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 76716,
            "range": "± 1493",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 80891,
            "range": "± 1745",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 76025,
            "range": "± 967",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 85387,
            "range": "± 1062",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 75909,
            "range": "± 1118",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 85248,
            "range": "± 1671",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 158109,
            "range": "± 4049",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 266694,
            "range": "± 3906",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 106505,
            "range": "± 2774",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8716,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 117868,
            "range": "± 4155",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 106627,
            "range": "± 2786",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9343,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 119228,
            "range": "± 2296",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 107838,
            "range": "± 1735",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3305,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 109468,
            "range": "± 2141",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 112143,
            "range": "± 1896",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12979,
            "range": "± 236",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 119490,
            "range": "± 5353",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 427359,
            "range": "± 14654",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 51794,
            "range": "± 947",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 278622,
            "range": "± 10834",
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
          "id": "61714453b8bb10dff305bd97f0300e44008d938f",
          "message": "test(mail): serialize shared trap state",
          "timestamp": "2026-09-01T04:38:07-03:00",
          "tree_id": "2bdad4f4d63412a2a6b5ab61806d23c6f1dc004e",
          "url": "https://github.com/Rullst/Rullst/commit/61714453b8bb10dff305bd97f0300e44008d938f"
        },
        "date": 1788248764706,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 5,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 8,
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
            "value": 304,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 258,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 133,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 5437133,
            "range": "± 95430572",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 65319,
            "range": "± 2882",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 68746,
            "range": "± 2234",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 63939,
            "range": "± 1629",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 75497,
            "range": "± 3399",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 66294,
            "range": "± 2323",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 72694,
            "range": "± 2621",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 131552,
            "range": "± 6466",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 227218,
            "range": "± 18815",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 153540,
            "range": "± 23323",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 4668,
            "range": "± 258",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 175148,
            "range": "± 16405",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 164600,
            "range": "± 28360",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 4970,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 144917,
            "range": "± 28124",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 164387,
            "range": "± 20098",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 936,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 160669,
            "range": "± 20743",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 166407,
            "range": "± 23795",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 7152,
            "range": "± 234",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 155079,
            "range": "± 34059",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 1137285,
            "range": "± 1231368",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 601643,
            "range": "± 2869441",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 510112,
            "range": "± 730168",
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
          "distinct": false,
          "id": "e14e3ef58cf082ee3e9b07b750278a36f4c51cbe",
          "message": "test(security): exercise guarded adapter failures",
          "timestamp": "2026-09-01T06:27:09-03:00",
          "tree_id": "e8266da871527daff3ac0759b85877fd77b0e157",
          "url": "https://github.com/Rullst/Rullst/commit/e14e3ef58cf082ee3e9b07b750278a36f4c51cbe"
        },
        "date": 1788255274663,
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
            "value": 470,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 373,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 211,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2524695,
            "range": "± 222167",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 91061,
            "range": "± 1072",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 95897,
            "range": "± 1649",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 91767,
            "range": "± 3258",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 100782,
            "range": "± 1261",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 91741,
            "range": "± 1900",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 100942,
            "range": "± 1482",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 144124,
            "range": "± 1773",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 238302,
            "range": "± 3973",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 126741,
            "range": "± 3740",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8097,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 144500,
            "range": "± 3683",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 126052,
            "range": "± 3291",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 8849,
            "range": "± 47",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 145690,
            "range": "± 3842",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 127819,
            "range": "± 3697",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2705,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 125173,
            "range": "± 5245",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 131644,
            "range": "± 3172",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12161,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 144400,
            "range": "± 4310",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 512996,
            "range": "± 23243",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 48757,
            "range": "± 1647",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 322321,
            "range": "± 19438",
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
          "id": "2754d75c632953d59231e5a772afab6f63e3be03",
          "message": "test(coverage): exercise provider and safety contracts",
          "timestamp": "2026-09-01T06:46:52-03:00",
          "tree_id": "c0a34ff31fa47c5d787e6c8af2b1c9280f505f52",
          "url": "https://github.com/Rullst/Rullst/commit/2754d75c632953d59231e5a772afab6f63e3be03"
        },
        "date": 1788256461829,
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
            "value": 44,
            "range": "± 1",
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
            "value": 373,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 213,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2159158,
            "range": "± 85630",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 100908,
            "range": "± 1263",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 105806,
            "range": "± 2014",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 97851,
            "range": "± 2399",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105109,
            "range": "± 1838",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 96330,
            "range": "± 2050",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 105469,
            "range": "± 1704",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155022,
            "range": "± 3047",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 248508,
            "range": "± 4462",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 140762,
            "range": "± 6587",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8383,
            "range": "± 67",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 157081,
            "range": "± 6143",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 138367,
            "range": "± 5786",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9091,
            "range": "± 202",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 157724,
            "range": "± 5050",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 139052,
            "range": "± 3974",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2816,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 137016,
            "range": "± 5763",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 141691,
            "range": "± 5695",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12314,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 150313,
            "range": "± 6218",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 551634,
            "range": "± 29285",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 48860,
            "range": "± 2230",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 348950,
            "range": "± 19951",
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
          "id": "6c35af8bc11b6e51caac1119d6eba9d8dc78ffe0",
          "message": "fix(orm): bound wide audit restore patches",
          "timestamp": "2026-09-01T08:44:45-03:00",
          "tree_id": "6a4ddc336a70df30c65aa0ead77567099d9a7294",
          "url": "https://github.com/Rullst/Rullst/commit/6c35af8bc11b6e51caac1119d6eba9d8dc78ffe0"
        },
        "date": 1788263542589,
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
            "value": 414,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 298,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 174,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2367608,
            "range": "± 3331318",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 62741,
            "range": "± 734",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 66527,
            "range": "± 1519",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 62857,
            "range": "± 1004",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 69742,
            "range": "± 2385",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 62519,
            "range": "± 1165",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 69722,
            "range": "± 740",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 123509,
            "range": "± 1826",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 204649,
            "range": "± 2901",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 87067,
            "range": "± 3381",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 6717,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 96869,
            "range": "± 1839",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 87200,
            "range": "± 4345",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 7181,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 98890,
            "range": "± 2139",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 90687,
            "range": "± 7606",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2556,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 90614,
            "range": "± 1711",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 92558,
            "range": "± 5860",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 9866,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 98818,
            "range": "± 5106",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 567510,
            "range": "± 319465",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 124083,
            "range": "± 101133",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 396028,
            "range": "± 668090",
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
          "distinct": false,
          "id": "76bf9c6e24c4f9f7a197f8d12e4d5538f556f83b",
          "message": "docs(readme): align public quality badges",
          "timestamp": "2026-09-01T09:04:52-03:00",
          "tree_id": "90754f5662c73fc386208be71e70b574cc2f5841",
          "url": "https://github.com/Rullst/Rullst/commit/76bf9c6e24c4f9f7a197f8d12e4d5538f556f83b"
        },
        "date": 1788264729615,
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
            "value": 479,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 370,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 210,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2415809,
            "range": "± 161416",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 103183,
            "range": "± 1226",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 108155,
            "range": "± 1040",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 102403,
            "range": "± 2973",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 108304,
            "range": "± 1517",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 98972,
            "range": "± 2306",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 108716,
            "range": "± 1354",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 158489,
            "range": "± 1398",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 256884,
            "range": "± 2242",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 135751,
            "range": "± 6061",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8247,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 150151,
            "range": "± 8139",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 135672,
            "range": "± 5119",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9031,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 152164,
            "range": "± 5285",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 135353,
            "range": "± 6089",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2803,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 136685,
            "range": "± 5191",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 138539,
            "range": "± 5065",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12552,
            "range": "± 66",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 156852,
            "range": "± 8556",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 535426,
            "range": "± 29578",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 50976,
            "range": "± 1711",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 337233,
            "range": "± 17856",
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
          "distinct": false,
          "id": "25787fa9e84e1025e41ec2e73513486ddd5ff97a",
          "message": "docs(release): record coverage and gh login",
          "timestamp": "2026-09-01T09:30:50-03:00",
          "tree_id": "c659e5b958c007ea7d43fef8fc306f9a213be754",
          "url": "https://github.com/Rullst/Rullst/commit/25787fa9e84e1025e41ec2e73513486ddd5ff97a"
        },
        "date": 1788266285771,
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
            "value": 47,
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
            "value": 389,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 209,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2134576,
            "range": "± 143447",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 98901,
            "range": "± 1078",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 104033,
            "range": "± 1163",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 96868,
            "range": "± 2803",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105309,
            "range": "± 1696",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 95235,
            "range": "± 1918",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 104693,
            "range": "± 2523",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 152800,
            "range": "± 1432",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 247133,
            "range": "± 2410",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 128039,
            "range": "± 4696",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8192,
            "range": "± 273",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 141982,
            "range": "± 5578",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 129713,
            "range": "± 6364",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 8934,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 146220,
            "range": "± 5706",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 127408,
            "range": "± 5422",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2747,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 132513,
            "range": "± 5810",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 135744,
            "range": "± 17256",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12093,
            "range": "± 102",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 149451,
            "range": "± 9407",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 527104,
            "range": "± 30687",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 49832,
            "range": "± 1438",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 318701,
            "range": "± 21360",
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
          "id": "c2a10ff786c182842a197bc2eb1a0adc3e4d0b10",
          "message": "ci(docs): validate book and links",
          "timestamp": "2026-09-01T09:52:13-03:00",
          "tree_id": "15dcf0cdfaf7543572a7d71fb77ff4a1088effa4",
          "url": "https://github.com/Rullst/Rullst/commit/c2a10ff786c182842a197bc2eb1a0adc3e4d0b10"
        },
        "date": 1788267581552,
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
            "value": 413,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 292,
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
            "value": 2296470,
            "range": "± 6605967",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 60292,
            "range": "± 918",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 63699,
            "range": "± 940",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 60259,
            "range": "± 865",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 66853,
            "range": "± 1258",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 59917,
            "range": "± 836",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 65822,
            "range": "± 1188",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 120648,
            "range": "± 1853",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 194078,
            "range": "± 1889",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 84068,
            "range": "± 1430",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 6832,
            "range": "± 139",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 94219,
            "range": "± 3149",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 85688,
            "range": "± 10320",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 7331,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 95489,
            "range": "± 2469",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 87187,
            "range": "± 1993",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2556,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 86553,
            "range": "± 1613",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 88026,
            "range": "± 1627",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 9968,
            "range": "± 69",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 92384,
            "range": "± 3342",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 420233,
            "range": "± 282285",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 192297,
            "range": "± 190580",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 290646,
            "range": "± 363318",
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
          "id": "de49be4afc3ecf2073a539c750b34749edc64efd",
          "message": "docs(audit): correct framework usage contracts",
          "timestamp": "2026-09-01T10:38:33-03:00",
          "tree_id": "3d64a82a6bd3ea4025004ea716178ab62617f410",
          "url": "https://github.com/Rullst/Rullst/commit/de49be4afc3ecf2073a539c750b34749edc64efd"
        },
        "date": 1788270354232,
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
            "value": 50,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 551,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 388,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 214,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2083043,
            "range": "± 128150",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 95210,
            "range": "± 14427",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 89728,
            "range": "± 9822",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 94473,
            "range": "± 10512",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 108768,
            "range": "± 15588",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 81622,
            "range": "± 5899",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 95801,
            "range": "± 10703",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 152854,
            "range": "± 4998",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 257340,
            "range": "± 7014",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 108521,
            "range": "± 3853",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8668,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 121108,
            "range": "± 6523",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 110744,
            "range": "± 9672",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9360,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 121838,
            "range": "± 3598",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 110986,
            "range": "± 3750",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3312,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 108864,
            "range": "± 1618",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 113883,
            "range": "± 2511",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12798,
            "range": "± 421",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 119644,
            "range": "± 5820",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 442456,
            "range": "± 134661",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 52102,
            "range": "± 836",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 280266,
            "range": "± 43580",
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
          "id": "8b7ff1824a27b1c648de44d07ed3fd317233d3a7",
          "message": "fix(ci): expose pinned lychee binary",
          "timestamp": "2026-09-01T10:51:38-03:00",
          "tree_id": "ee18f238ca936f10e2c4e4d86877aeb2dcc80036",
          "url": "https://github.com/Rullst/Rullst/commit/8b7ff1824a27b1c648de44d07ed3fd317233d3a7"
        },
        "date": 1788271166429,
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
            "value": 38,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 415,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 293,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 179,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2334765,
            "range": "± 13615227",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 61220,
            "range": "± 1002",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 64555,
            "range": "± 709",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 60333,
            "range": "± 788",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 65227,
            "range": "± 1008",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 59550,
            "range": "± 854",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 67070,
            "range": "± 741",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 119300,
            "range": "± 1845",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 197310,
            "range": "± 1856",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 86125,
            "range": "± 2519",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 6737,
            "range": "± 84",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 94741,
            "range": "± 2574",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 86512,
            "range": "± 2931",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 7228,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 94504,
            "range": "± 3848",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 87486,
            "range": "± 1942",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2335,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 86317,
            "range": "± 2040",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 89460,
            "range": "± 2342",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 9890,
            "range": "± 61",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 94336,
            "range": "± 4657",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 478711,
            "range": "± 412718",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 307079,
            "range": "± 289795",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 461118,
            "range": "± 403049",
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
          "distinct": false,
          "id": "bdc8600b4da8df64a8388ae673c40963f9514489",
          "message": "ci(docs): validate every markdown link",
          "timestamp": "2026-09-01T11:01:07-03:00",
          "tree_id": "1fab6be1f3df7337a0d0c72b237a39211dc09d6c",
          "url": "https://github.com/Rullst/Rullst/commit/bdc8600b4da8df64a8388ae673c40963f9514489"
        },
        "date": 1788271719033,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 7,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 9,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 26,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 256,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 218,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 123,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 3202039,
            "range": "± 9167631",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 49612,
            "range": "± 1293",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 53067,
            "range": "± 1375",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 50699,
            "range": "± 3771",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 56281,
            "range": "± 3005",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 51868,
            "range": "± 1633",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 57732,
            "range": "± 3259",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 100842,
            "range": "± 4869",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 158088,
            "range": "± 5778",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 75309,
            "range": "± 1399",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 5241,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 79520,
            "range": "± 5043",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 76859,
            "range": "± 7685",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 5764,
            "range": "± 96",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 82308,
            "range": "± 3250",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 78364,
            "range": "± 4089",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 1940,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 79497,
            "range": "± 3918",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 82104,
            "range": "± 3021",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 7785,
            "range": "± 158",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 81432,
            "range": "± 2251",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 399664,
            "range": "± 822775",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 592593,
            "range": "± 584575",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 372985,
            "range": "± 591986",
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
          "id": "7d4ab6e907e4907ed3c4f117b3bc887b70166851",
          "message": "docs(audit): retire stale release guidance",
          "timestamp": "2026-09-01T13:21:47-03:00",
          "tree_id": "a891518804c53c0c08525284d2107ab3fea08e35",
          "url": "https://github.com/Rullst/Rullst/commit/7d4ab6e907e4907ed3c4f117b3bc887b70166851"
        },
        "date": 1788280158813,
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
            "value": 17,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 50,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 531,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 384,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 210,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1765165,
            "range": "± 92887",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 81109,
            "range": "± 965",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 85444,
            "range": "± 1419",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 80206,
            "range": "± 1720",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 90626,
            "range": "± 813",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 78791,
            "range": "± 1427",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 90287,
            "range": "± 1832",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155533,
            "range": "± 1929",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 263291,
            "range": "± 6100",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 110220,
            "range": "± 2214",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8858,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 121289,
            "range": "± 2318",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 110451,
            "range": "± 2187",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9334,
            "range": "± 130",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 124832,
            "range": "± 2604",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 111353,
            "range": "± 1715",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3312,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 112022,
            "range": "± 4798",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 115444,
            "range": "± 3087",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12953,
            "range": "± 47",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 122795,
            "range": "± 5514",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 442653,
            "range": "± 20927",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 51765,
            "range": "± 3632",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 283950,
            "range": "± 11174",
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
          "id": "060933005ba365230dfd960935307857e2f6d229",
          "message": "test(docs): compile public tutorial examples",
          "timestamp": "2026-09-01T14:12:03-03:00",
          "tree_id": "21ad47d5bcf363f0751a865a9d10d34112c20315",
          "url": "https://github.com/Rullst/Rullst/commit/060933005ba365230dfd960935307857e2f6d229"
        },
        "date": 1788283211928,
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
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 468,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 376,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 207,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2195850,
            "range": "± 108858",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 101980,
            "range": "± 1175",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 107102,
            "range": "± 1729",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 98782,
            "range": "± 2102",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 107716,
            "range": "± 2953",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 98080,
            "range": "± 2168",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 107749,
            "range": "± 1777",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154869,
            "range": "± 1255",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 250386,
            "range": "± 2946",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 131873,
            "range": "± 5312",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8281,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 146269,
            "range": "± 3808",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 131247,
            "range": "± 6366",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9082,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 149335,
            "range": "± 4421",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 133304,
            "range": "± 5651",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2780,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 131123,
            "range": "± 6055",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 139004,
            "range": "± 7300",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12287,
            "range": "± 59",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 150801,
            "range": "± 6293",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 551270,
            "range": "± 35525",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 49220,
            "range": "± 1449",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 335663,
            "range": "± 15502",
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
          "id": "42a63bad754a3714cc899c241d7b3d0d3cd55dca",
          "message": "fix(ci): exclude dev dependencies from architecture graph",
          "timestamp": "2026-09-01T16:49:59-03:00",
          "tree_id": "2892a262222604b75f4e8ddad86ec4a1a5381c10",
          "url": "https://github.com/Rullst/Rullst/commit/42a63bad754a3714cc899c241d7b3d0d3cd55dca"
        },
        "date": 1788292688331,
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
            "value": 49,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 528,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 384,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 214,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2043205,
            "range": "± 118708",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 77689,
            "range": "± 2858",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 82444,
            "range": "± 2134",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 77476,
            "range": "± 1298",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 84929,
            "range": "± 1136",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 77504,
            "range": "± 1216",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 85732,
            "range": "± 3123",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155184,
            "range": "± 2602",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 258414,
            "range": "± 4234",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 107940,
            "range": "± 2101",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8697,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 118350,
            "range": "± 2558",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 107394,
            "range": "± 1804",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9256,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 119922,
            "range": "± 2529",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 109280,
            "range": "± 1509",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3283,
            "range": "± 308",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 108872,
            "range": "± 2534",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 113854,
            "range": "± 16037",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12657,
            "range": "± 121",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 121068,
            "range": "± 3381",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 446617,
            "range": "± 18239",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 53918,
            "range": "± 1198",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 280123,
            "range": "± 12553",
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
          "id": "7cc12a2edac0a984c7c036e0e12624e37e0b1712",
          "message": "feat(quality): strengthen macro and IoT release evidence",
          "timestamp": "2026-09-01T17:26:54-03:00",
          "tree_id": "27558ff1e280654e26a77a3e7dd650ddc0082d3f",
          "url": "https://github.com/Rullst/Rullst/commit/7cc12a2edac0a984c7c036e0e12624e37e0b1712"
        },
        "date": 1788294891115,
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
            "value": 306,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 246,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 129,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 45807622,
            "range": "± 144733058",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 62108,
            "range": "± 2550",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 65599,
            "range": "± 2921",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 63720,
            "range": "± 2117",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 70608,
            "range": "± 2301",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 63869,
            "range": "± 2045",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 71537,
            "range": "± 3145",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 143262,
            "range": "± 12122",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 224576,
            "range": "± 16324",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 165173,
            "range": "± 24343",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 4539,
            "range": "± 366",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 165503,
            "range": "± 26908",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 166258,
            "range": "± 12477",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 5005,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 168602,
            "range": "± 23958",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 159249,
            "range": "± 18199",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 1033,
            "range": "± 66",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 166693,
            "range": "± 14901",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 167005,
            "range": "± 23264",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 7194,
            "range": "± 434",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 172839,
            "range": "± 28661",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 824383,
            "range": "± 706310",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 135304,
            "range": "± 258063",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 868357,
            "range": "± 361228",
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
          "distinct": false,
          "id": "ba11f6dde724ae9e2863c6cb3d4ab600fa0acdac",
          "message": "fix(ci): build the DAST blog target explicitly",
          "timestamp": "2026-09-01T17:40:23-03:00",
          "tree_id": "981b8f7ac12e91db4759641fe0100bae1ec67983",
          "url": "https://github.com/Rullst/Rullst/commit/ba11f6dde724ae9e2863c6cb3d4ab600fa0acdac"
        },
        "date": 1788295686693,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 5,
            "range": "± 1",
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 302,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 231,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 129,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 5655900,
            "range": "± 87549368",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 64446,
            "range": "± 3904",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 68150,
            "range": "± 1960",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 62676,
            "range": "± 1720",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 72727,
            "range": "± 2146",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 64901,
            "range": "± 1506",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 72864,
            "range": "± 1495",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 129236,
            "range": "± 5191",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 229319,
            "range": "± 15975",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 154278,
            "range": "± 19794",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 4499,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 172726,
            "range": "± 25901",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 163507,
            "range": "± 22853",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 5168,
            "range": "± 53",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 175711,
            "range": "± 20806",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 163832,
            "range": "± 23595",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 941,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 166158,
            "range": "± 15371",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 170593,
            "range": "± 13673",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 7230,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 137174,
            "range": "± 36207",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 655050,
            "range": "± 498909",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 102233,
            "range": "± 206757",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 422184,
            "range": "± 438906",
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
          "id": "683e2b2eccbd40d0896138746f62447850896c77",
          "message": "docs(release): record v12 quality gate",
          "timestamp": "2026-09-01T18:24:55-03:00",
          "tree_id": "c41db748a230ea1db3ae5d1b0a7bb9522f10fd96",
          "url": "https://github.com/Rullst/Rullst/commit/683e2b2eccbd40d0896138746f62447850896c77"
        },
        "date": 1788298354160,
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
            "value": 50,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 523,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 382,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 210,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1774050,
            "range": "± 115292",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 77895,
            "range": "± 989",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 82963,
            "range": "± 3382",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 77917,
            "range": "± 2016",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 88169,
            "range": "± 3442",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 78883,
            "range": "± 1736",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 88026,
            "range": "± 4637",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153615,
            "range": "± 3039",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 255980,
            "range": "± 3395",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 108203,
            "range": "± 2514",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8750,
            "range": "± 127",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 117133,
            "range": "± 1763",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 107111,
            "range": "± 19438",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9380,
            "range": "± 272",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 119744,
            "range": "± 6840",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 108625,
            "range": "± 8601",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3033,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 108241,
            "range": "± 2185",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 111600,
            "range": "± 1680",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12785,
            "range": "± 480",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 120713,
            "range": "± 15718",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 434373,
            "range": "± 15168",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 52800,
            "range": "± 650",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 277008,
            "range": "± 15008",
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
          "id": "fa9216bda0cfa4a07a62c48549353b0399e18c20",
          "message": "feat(security): add durable local SIEM spool",
          "timestamp": "2026-09-01T18:37:17-03:00",
          "tree_id": "26d6dc22f32939eb4e37d2f3b0913ee4b907b9fb",
          "url": "https://github.com/Rullst/Rullst/commit/fa9216bda0cfa4a07a62c48549353b0399e18c20"
        },
        "date": 1788299088821,
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
            "value": 53,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 474,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 371,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 211,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2129933,
            "range": "± 118300",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 97660,
            "range": "± 1206",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 103110,
            "range": "± 1272",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 95355,
            "range": "± 1879",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 104252,
            "range": "± 1480",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 94524,
            "range": "± 2129",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 104151,
            "range": "± 2746",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 152343,
            "range": "± 1422",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 241780,
            "range": "± 3546",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 127165,
            "range": "± 6283",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 7997,
            "range": "± 63",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 143133,
            "range": "± 6255",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 127947,
            "range": "± 4490",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 8921,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 145480,
            "range": "± 4404",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 128927,
            "range": "± 5996",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2607,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 131642,
            "range": "± 5097",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 133220,
            "range": "± 3466",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12115,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 148242,
            "range": "± 7790",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 526859,
            "range": "± 29543",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 49920,
            "range": "± 2204",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 319895,
            "range": "± 15681",
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
          "id": "fa29c2c3b1d16258ee8a392a2bdd493bdac6f06e",
          "message": "feat(connect): encrypt persisted refresh state",
          "timestamp": "2026-09-01T19:46:57-03:00",
          "tree_id": "624ea87583bb75d413c70b2ba081dd00a5b90dfa",
          "url": "https://github.com/Rullst/Rullst/commit/fa29c2c3b1d16258ee8a392a2bdd493bdac6f06e"
        },
        "date": 1788303317304,
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
            "value": 50,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 537,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 391,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 214,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1711050,
            "range": "± 67504",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 76548,
            "range": "± 1130",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 81756,
            "range": "± 969",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 77442,
            "range": "± 869",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 83879,
            "range": "± 1853",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 76484,
            "range": "± 1082",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 85096,
            "range": "± 740",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153417,
            "range": "± 2188",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 254195,
            "range": "± 2675",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 107055,
            "range": "± 2177",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8659,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 116714,
            "range": "± 2724",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 105470,
            "range": "± 1800",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9433,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 118908,
            "range": "± 3140",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 107691,
            "range": "± 3785",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3302,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 107819,
            "range": "± 1948",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 112090,
            "range": "± 2152",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12733,
            "range": "± 142",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 120922,
            "range": "± 4573",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 435997,
            "range": "± 16746",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 50468,
            "range": "± 724",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 273177,
            "range": "± 10697",
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
          "distinct": false,
          "id": "2adfe0a9df1d1c6da46932953f42532d9c96af66",
          "message": "feat(ai): add compatible provider adapter",
          "timestamp": "2026-09-01T23:52:08-03:00",
          "tree_id": "9fbe592fed2b06650a82a5b336556ab35e885343",
          "url": "https://github.com/Rullst/Rullst/commit/2adfe0a9df1d1c6da46932953f42532d9c96af66"
        },
        "date": 1788317973072,
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
            "value": 51,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 531,
            "range": "± 9",
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
            "value": 206,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1684860,
            "range": "± 53212",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 77851,
            "range": "± 1092",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 79861,
            "range": "± 1559",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 76223,
            "range": "± 899",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 86393,
            "range": "± 1236",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 76361,
            "range": "± 854",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 85030,
            "range": "± 2321",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154837,
            "range": "± 1998",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 257365,
            "range": "± 5350",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 108800,
            "range": "± 3570",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8691,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 120203,
            "range": "± 2367",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 107738,
            "range": "± 1720",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9238,
            "range": "± 156",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 121419,
            "range": "± 3506",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 109083,
            "range": "± 1703",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3318,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 109017,
            "range": "± 2548",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 113037,
            "range": "± 2322",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12576,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 120580,
            "range": "± 5801",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 444065,
            "range": "± 20707",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 53394,
            "range": "± 1161",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 277388,
            "range": "± 11866",
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
          "distinct": false,
          "id": "a9c13acb8a22272a168419fb753c066ff874a93e",
          "message": "docs(site): redesign landing and benchmark hub",
          "timestamp": "2026-09-02T00:31:48-03:00",
          "tree_id": "1fca7a51edbd859a79a93e3d0fa215710f1df9b0",
          "url": "https://github.com/Rullst/Rullst/commit/a9c13acb8a22272a168419fb753c066ff874a93e"
        },
        "date": 1788320359716,
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
            "value": 51,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 533,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 380,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 207,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1855797,
            "range": "± 73924",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 79482,
            "range": "± 1116",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 84481,
            "range": "± 689",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 79218,
            "range": "± 1179",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 88094,
            "range": "± 824",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 78127,
            "range": "± 1074",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 89086,
            "range": "± 1971",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 157151,
            "range": "± 2820",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 259268,
            "range": "± 4103",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 110453,
            "range": "± 1721",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8695,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 121476,
            "range": "± 2384",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 110444,
            "range": "± 2725",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9528,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 123619,
            "range": "± 2871",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 112264,
            "range": "± 1938",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3306,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 112226,
            "range": "± 1731",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 115627,
            "range": "± 2642",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12519,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 122522,
            "range": "± 3521",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 446018,
            "range": "± 17806",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 51577,
            "range": "± 1323",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 279372,
            "range": "± 11594",
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
          "distinct": false,
          "id": "99b7f940caab400125427dfe390b4cce010ede53",
          "message": "ci(release): verify complete feature boundaries",
          "timestamp": "2026-09-02T03:29:59-03:00",
          "tree_id": "bec28409f156cfd18b2168e0b2265325eed0be79",
          "url": "https://github.com/Rullst/Rullst/commit/99b7f940caab400125427dfe390b4cce010ede53"
        },
        "date": 1788331084231,
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
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 39,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 370,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 277,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 165,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 3460588,
            "range": "± 10337009",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 59723,
            "range": "± 894",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 63356,
            "range": "± 928",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 70244,
            "range": "± 3510",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 69875,
            "range": "± 2362",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 61639,
            "range": "± 1140",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 69363,
            "range": "± 1263",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 142458,
            "range": "± 4311",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 249389,
            "range": "± 6515",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 115333,
            "range": "± 6201",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 5526,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 123423,
            "range": "± 6580",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 117311,
            "range": "± 7185",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 5984,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 124501,
            "range": "± 6445",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 114467,
            "range": "± 8015",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 1158,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 116360,
            "range": "± 6997",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 122822,
            "range": "± 5496",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 8622,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 125175,
            "range": "± 7120",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 580620,
            "range": "± 1753105",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 274091,
            "range": "± 503592",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 380028,
            "range": "± 348476",
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
          "id": "bc9eff484e9c666f4904c2db4c8389ff49464579",
          "message": "feat(ai): add durable local audit trails",
          "timestamp": "2026-09-02T04:53:32-03:00",
          "tree_id": "2c9e5b6cb1602d23acb9906c888df0d81909cbbe",
          "url": "https://github.com/Rullst/Rullst/commit/bc9eff484e9c666f4904c2db4c8389ff49464579"
        },
        "date": 1788336058224,
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
            "value": 44,
            "range": "± 1",
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
            "value": 378,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 211,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2359187,
            "range": "± 160658",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 99431,
            "range": "± 1340",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 104746,
            "range": "± 1808",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 97929,
            "range": "± 2264",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 104992,
            "range": "± 1471",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 96056,
            "range": "± 2077",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 105223,
            "range": "± 1253",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153242,
            "range": "± 2278",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 246902,
            "range": "± 2392",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 130986,
            "range": "± 5016",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8438,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 148205,
            "range": "± 4625",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 131871,
            "range": "± 4596",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9354,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 148324,
            "range": "± 3793",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 133033,
            "range": "± 4613",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2864,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 134740,
            "range": "± 4306",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 136064,
            "range": "± 7144",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12404,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 149718,
            "range": "± 5481",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 536704,
            "range": "± 30861",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 49119,
            "range": "± 2111",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 330725,
            "range": "± 16345",
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
          "id": "e48d52638c873fdd5a7fb3c0d6dec37fca5faee6",
          "message": "feat(auth): add durable shared sqlite state",
          "timestamp": "2026-09-02T06:02:37-03:00",
          "tree_id": "2293d3c7128846118c17cc03f81fbc7a9e2f5638",
          "url": "https://github.com/Rullst/Rullst/commit/e48d52638c873fdd5a7fb3c0d6dec37fca5faee6"
        },
        "date": 1788340187307,
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 383,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 209,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2624320,
            "range": "± 381094",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 89357,
            "range": "± 1105",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 94151,
            "range": "± 2282",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 91012,
            "range": "± 2308",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 98712,
            "range": "± 1816",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 89660,
            "range": "± 1547",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 98701,
            "range": "± 1554",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 145537,
            "range": "± 4547",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 243114,
            "range": "± 7308",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 125621,
            "range": "± 2578",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8078,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 139607,
            "range": "± 7846",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 127851,
            "range": "± 2818",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 8904,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 143384,
            "range": "± 3962",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 126652,
            "range": "± 4605",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2697,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 126002,
            "range": "± 4659",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 132704,
            "range": "± 5758",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12073,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 141339,
            "range": "± 3110",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 522522,
            "range": "± 36882",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 50104,
            "range": "± 1475",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 322664,
            "range": "± 16023",
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
          "id": "7263d971fa7c85b16df175bffe10549823d5aded",
          "message": "feat(mail): add durable suppression and inspection",
          "timestamp": "2026-09-02T06:40:18-03:00",
          "tree_id": "f3aa916c999f3c70e14d89721718f968b69cf073",
          "url": "https://github.com/Rullst/Rullst/commit/7263d971fa7c85b16df175bffe10549823d5aded"
        },
        "date": 1788342462567,
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
            "value": 51,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 526,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 382,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 211,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1761370,
            "range": "± 105621",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 77369,
            "range": "± 951",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 80523,
            "range": "± 2172",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 75765,
            "range": "± 1161",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 85360,
            "range": "± 1096",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 76389,
            "range": "± 1153",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 83485,
            "range": "± 1658",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153382,
            "range": "± 2399",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 255841,
            "range": "± 3246",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 107278,
            "range": "± 3479",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8850,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 117285,
            "range": "± 3659",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 106922,
            "range": "± 2015",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9364,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 121691,
            "range": "± 3589",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 110091,
            "range": "± 7184",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3284,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 109078,
            "range": "± 1827",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 112197,
            "range": "± 2301",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12692,
            "range": "± 83",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 122400,
            "range": "± 5434",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 433747,
            "range": "± 19403",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 51753,
            "range": "± 571",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 276328,
            "range": "± 10708",
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
          "id": "61f1326a518ea52368010a208933997d77d2d8c2",
          "message": "feat(messaging): add wire and trace contracts",
          "timestamp": "2026-09-02T06:55:34-03:00",
          "tree_id": "cf0ce63bbffd8cbe4edd397da2c2c728cd67aaf4",
          "url": "https://github.com/Rullst/Rullst/commit/61f1326a518ea52368010a208933997d77d2d8c2"
        },
        "date": 1788343356359,
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
            "value": 470,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 374,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 212,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2154702,
            "range": "± 101830",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 100289,
            "range": "± 1486",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 105807,
            "range": "± 1766",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 97985,
            "range": "± 3405",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105798,
            "range": "± 1969",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 95796,
            "range": "± 2837",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 105820,
            "range": "± 1526",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154742,
            "range": "± 2273",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 249362,
            "range": "± 2704",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 131588,
            "range": "± 7594",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8260,
            "range": "± 105",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 148721,
            "range": "± 5695",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 133026,
            "range": "± 4463",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9257,
            "range": "± 55",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 151054,
            "range": "± 5850",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 133268,
            "range": "± 7177",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2814,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 134957,
            "range": "± 5926",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 137704,
            "range": "± 4828",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12298,
            "range": "± 202",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 151139,
            "range": "± 5290",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 552639,
            "range": "± 28681",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 48651,
            "range": "± 1290",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 341177,
            "range": "± 20879",
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
          "id": "948b56c0151558aa5ab2e85b8418a53ac24702a0",
          "message": "feat(messaging): encrypt durability and bridge outbox",
          "timestamp": "2026-09-02T08:05:31-03:00",
          "tree_id": "fb63ef8b2e2004399c4342400886752a3963da76",
          "url": "https://github.com/Rullst/Rullst/commit/948b56c0151558aa5ab2e85b8418a53ac24702a0"
        },
        "date": 1788347670584,
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
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 376,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 213,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2451088,
            "range": "± 153504",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 102920,
            "range": "± 1933",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 108042,
            "range": "± 3679",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 101209,
            "range": "± 2324",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 110642,
            "range": "± 2049",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 102696,
            "range": "± 2198",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 109720,
            "range": "± 1304",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155924,
            "range": "± 1386",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 249973,
            "range": "± 4149",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 136031,
            "range": "± 7581",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8380,
            "range": "± 75",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 151376,
            "range": "± 5310",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 135348,
            "range": "± 6443",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9351,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 153532,
            "range": "± 5768",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 136967,
            "range": "± 6133",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2786,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 138523,
            "range": "± 7013",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 142356,
            "range": "± 5947",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12345,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 156074,
            "range": "± 5713",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 547767,
            "range": "± 29358",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 49744,
            "range": "± 1289",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 342207,
            "range": "± 17661",
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
          "id": "fa940a6ebcf3d797950938773c287ef5dbc70bc6",
          "message": "docs(ai): define contribution and maintainability policy",
          "timestamp": "2026-09-02T10:42:06-03:00",
          "tree_id": "0d41f2e38ec797d660a6bd79ff491bb660ac015e",
          "url": "https://github.com/Rullst/Rullst/commit/fa940a6ebcf3d797950938773c287ef5dbc70bc6"
        },
        "date": 1788356960574,
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
            "value": 470,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 375,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 208,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2189917,
            "range": "± 127893",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 101841,
            "range": "± 1634",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 107544,
            "range": "± 1524",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 99468,
            "range": "± 2738",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 107670,
            "range": "± 1382",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 99017,
            "range": "± 2008",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 108165,
            "range": "± 1698",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153956,
            "range": "± 5001",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 247258,
            "range": "± 4754",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 134142,
            "range": "± 4874",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8228,
            "range": "± 77",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 148688,
            "range": "± 5430",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 135217,
            "range": "± 4128",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9056,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 150377,
            "range": "± 4799",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 130526,
            "range": "± 4293",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2709,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 133138,
            "range": "± 4365",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 136935,
            "range": "± 6125",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12352,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 146640,
            "range": "± 5766",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 533029,
            "range": "± 41356",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 49126,
            "range": "± 1534",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 331600,
            "range": "± 15969",
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
          "id": "d4f18e1dd2d9e5688549cb896c4e7934251bab13",
          "message": "style(cli): refine startup color wave",
          "timestamp": "2026-09-02T13:20:36-03:00",
          "tree_id": "8315e3a901c1b25e8b6310ec775f004d0b4e6c0a",
          "url": "https://github.com/Rullst/Rullst/commit/d4f18e1dd2d9e5688549cb896c4e7934251bab13"
        },
        "date": 1788366869422,
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
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 49,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 533,
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
            "value": 213,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1707756,
            "range": "± 68509",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 77332,
            "range": "± 2153",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 81395,
            "range": "± 2486",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 76980,
            "range": "± 1601",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 84145,
            "range": "± 1231",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 78082,
            "range": "± 1222",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 85316,
            "range": "± 1394",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 162493,
            "range": "± 3448",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 267390,
            "range": "± 3920",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 107163,
            "range": "± 2894",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8840,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 120258,
            "range": "± 4715",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 108373,
            "range": "± 3370",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9574,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 119084,
            "range": "± 5120",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 109577,
            "range": "± 1756",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3046,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 108414,
            "range": "± 1642",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 113652,
            "range": "± 2104",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12880,
            "range": "± 197",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 122129,
            "range": "± 5220",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 441470,
            "range": "± 19202",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 52585,
            "range": "± 525",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 277805,
            "range": "± 12408",
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
          "id": "5b429c761145f2a17d74088bd7081c785097db6a",
          "message": "style(cli): stage startup brand gradients",
          "timestamp": "2026-09-02T13:39:02-03:00",
          "tree_id": "f6537391e66cef2a6f39b6e25a8ec2519dd3ca1c",
          "url": "https://github.com/Rullst/Rullst/commit/5b429c761145f2a17d74088bd7081c785097db6a"
        },
        "date": 1788367659189,
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
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 531,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 387,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 207,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1724936,
            "range": "± 123123",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 78053,
            "range": "± 3898",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 81752,
            "range": "± 1592",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 75708,
            "range": "± 2679",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 85982,
            "range": "± 1433",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 77426,
            "range": "± 684",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 86147,
            "range": "± 1544",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154353,
            "range": "± 2471",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 257241,
            "range": "± 2555",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 107370,
            "range": "± 2361",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8886,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 118405,
            "range": "± 3299",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 106900,
            "range": "± 2295",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9696,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 119124,
            "range": "± 2465",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 107943,
            "range": "± 1678",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3316,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 109281,
            "range": "± 3206",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 112127,
            "range": "± 2336",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 13397,
            "range": "± 82",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 119045,
            "range": "± 5474",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 434242,
            "range": "± 18916",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 51202,
            "range": "± 748",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 274856,
            "range": "± 12565",
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
          "id": "6020901c2eeb6f0d0efa74a47d51b9ceb4721752",
          "message": "feat(ai): add policy-bound vision sources",
          "timestamp": "2026-09-02T13:49:43-03:00",
          "tree_id": "2e0ebb8e777a4d988fb0b599feaecf7eff039594",
          "url": "https://github.com/Rullst/Rullst/commit/6020901c2eeb6f0d0efa74a47d51b9ceb4721752"
        },
        "date": 1788368642775,
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
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 48,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 533,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 395,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 211,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1849056,
            "range": "± 196565",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 79728,
            "range": "± 1708",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 81720,
            "range": "± 2243",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 77473,
            "range": "± 4027",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 88117,
            "range": "± 940",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 78417,
            "range": "± 1204",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 87899,
            "range": "± 1017",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154624,
            "range": "± 9596",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 258908,
            "range": "± 3013",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 108370,
            "range": "± 2606",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8804,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 118973,
            "range": "± 1815",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 108090,
            "range": "± 2268",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9533,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 122462,
            "range": "± 4204",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 109347,
            "range": "± 1755",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3315,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 107472,
            "range": "± 1783",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 112343,
            "range": "± 2091",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 13075,
            "range": "± 93",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 119681,
            "range": "± 4517",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 435998,
            "range": "± 19988",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 52570,
            "range": "± 995",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 275433,
            "range": "± 12766",
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
          "id": "fcbc24d48be2f716fb7538d3ae88357f884f7dd5",
          "message": "test(docs): include hot reload tutorial in doctests",
          "timestamp": "2026-09-02T15:22:54-03:00",
          "tree_id": "3084c2e60a059c0b8fa68c342dc6fb7a861c9d80",
          "url": "https://github.com/Rullst/Rullst/commit/fcbc24d48be2f716fb7538d3ae88357f884f7dd5"
        },
        "date": 1788374165029,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 7,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 9,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 25,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 269,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 213,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 127,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 3887089,
            "range": "± 24317147",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 54342,
            "range": "± 976",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 55008,
            "range": "± 2781",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 52555,
            "range": "± 1047",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 57540,
            "range": "± 2524",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 51511,
            "range": "± 1322",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 58764,
            "range": "± 3408",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 99929,
            "range": "± 5892",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 160328,
            "range": "± 18004",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 77303,
            "range": "± 2309",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 5474,
            "range": "± 146",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 83387,
            "range": "± 4535",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 77391,
            "range": "± 7533",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 5978,
            "range": "± 318",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 84865,
            "range": "± 7256",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 80019,
            "range": "± 2681",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2041,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 79140,
            "range": "± 1784",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 80769,
            "range": "± 2005",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 7825,
            "range": "± 256",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 81802,
            "range": "± 3091",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 469663,
            "range": "± 649017",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 288438,
            "range": "± 174325",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 514873,
            "range": "± 1040198",
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
          "distinct": false,
          "id": "cf58a64140abf47b1bd13cdfb089626da0683c99",
          "message": "feat(orm): complete bounded native enum mapping",
          "timestamp": "2026-09-02T16:47:47-03:00",
          "tree_id": "20f41c4bfd9d822da593200a45b8fd629d580834",
          "url": "https://github.com/Rullst/Rullst/commit/cf58a64140abf47b1bd13cdfb089626da0683c99"
        },
        "date": 1788380710552,
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
            "value": 11,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 368,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 277,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 168,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2328038,
            "range": "± 597570",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 61529,
            "range": "± 1261",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 81301,
            "range": "± 1271",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 61580,
            "range": "± 1460",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 70525,
            "range": "± 1034",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 63191,
            "range": "± 1274",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 79166,
            "range": "± 2848",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 142165,
            "range": "± 5395",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 257227,
            "range": "± 8264",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 120250,
            "range": "± 8294",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 5652,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 125863,
            "range": "± 7422",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 121661,
            "range": "± 6045",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 6145,
            "range": "± 68",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 125417,
            "range": "± 7094",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 116412,
            "range": "± 5320",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 1181,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 116669,
            "range": "± 6814",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 122409,
            "range": "± 5717",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 8790,
            "range": "± 57",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 129562,
            "range": "± 3733",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 576344,
            "range": "± 83383",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 97359,
            "range": "± 36833",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 306010,
            "range": "± 52222",
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
          "id": "24097d809093a47300ed5fd47de98d1fd28b1a39",
          "message": "feat(macros): add bounded typed server functions",
          "timestamp": "2026-09-02T18:33:59-03:00",
          "tree_id": "4f1166b38668d15bc7dfa21f91d01c885968b1cc",
          "url": "https://github.com/Rullst/Rullst/commit/24097d809093a47300ed5fd47de98d1fd28b1a39"
        },
        "date": 1788387875184,
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
            "value": 51,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 512,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 398,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 209,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1639443,
            "range": "± 545871",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 76420,
            "range": "± 1883",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 81240,
            "range": "± 796",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 76136,
            "range": "± 1013",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 85726,
            "range": "± 1330",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 76617,
            "range": "± 3055",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 86269,
            "range": "± 807",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 158011,
            "range": "± 2278",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 262489,
            "range": "± 3183",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 105930,
            "range": "± 4073",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8796,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 117013,
            "range": "± 2668",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 106881,
            "range": "± 1635",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9516,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 119344,
            "range": "± 3834",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 108450,
            "range": "± 1948",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3353,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 107232,
            "range": "± 1646",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 111800,
            "range": "± 1463",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 13080,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 118836,
            "range": "± 3964",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 431930,
            "range": "± 18195",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 51731,
            "range": "± 1808",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 276745,
            "range": "± 13042",
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
          "id": "745bde23cb31a352ecfe58d2a1739abef789a49f",
          "message": "fix(sqlite): close pools after failed initialization",
          "timestamp": "2026-09-02T20:21:28-03:00",
          "tree_id": "cca3ddb88c1be09d3c30058e750a0631eb883809",
          "url": "https://github.com/Rullst/Rullst/commit/745bde23cb31a352ecfe58d2a1739abef789a49f"
        },
        "date": 1788392116255,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 8,
            "range": "± 1",
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
            "value": 437,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 329,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 196,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 5988641,
            "range": "± 21233419",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 73711,
            "range": "± 11946",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 79208,
            "range": "± 5654",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 69532,
            "range": "± 3450",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 95277,
            "range": "± 14996",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 70802,
            "range": "± 1300",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 80744,
            "range": "± 4347",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 166431,
            "range": "± 4935",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 300114,
            "range": "± 13024",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 130344,
            "range": "± 7422",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 6585,
            "range": "± 80",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 141881,
            "range": "± 13534",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 130142,
            "range": "± 6412",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 7272,
            "range": "± 96",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 136144,
            "range": "± 5801",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 124548,
            "range": "± 5271",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 1388,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 133318,
            "range": "± 22831",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 139470,
            "range": "± 9953",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 10406,
            "range": "± 69",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 141605,
            "range": "± 10130",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 826996,
            "range": "± 1261079",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 102454,
            "range": "± 404542",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 413265,
            "range": "± 1485158",
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
          "id": "50796c962356ebbfbc1fc37b1a5bbd936c774981",
          "message": "feat(orm-macros): enforce fail-closed derive parsing",
          "timestamp": "2026-09-02T20:46:06-03:00",
          "tree_id": "7302a5e87eb52270bf04eb80370183227d6567ed",
          "url": "https://github.com/Rullst/Rullst/commit/50796c962356ebbfbc1fc37b1a5bbd936c774981"
        },
        "date": 1788393229426,
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
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 470,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 380,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 221,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2169660,
            "range": "± 109447",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 102596,
            "range": "± 1348",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 107721,
            "range": "± 757",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 101203,
            "range": "± 2118",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 108790,
            "range": "± 1196",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 98377,
            "range": "± 2033",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 108416,
            "range": "± 1530",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 156257,
            "range": "± 1495",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 248798,
            "range": "± 4191",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 133629,
            "range": "± 4935",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8556,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 150384,
            "range": "± 4560",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 138002,
            "range": "± 5667",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9414,
            "range": "± 295",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 153111,
            "range": "± 4511",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 137453,
            "range": "± 6112",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2776,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 137234,
            "range": "± 4359",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 138616,
            "range": "± 5693",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12511,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 153663,
            "range": "± 6491",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 558427,
            "range": "± 35691",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 48914,
            "range": "± 3142",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 341880,
            "range": "± 22178",
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
          "id": "f1d78028568cee152f453c1b8bca96b994ac21c0",
          "message": "feat(capital): add durable webhook replay ledger",
          "timestamp": "2026-09-02T21:37:50-03:00",
          "tree_id": "3cf575ad52ce3d390d69efcf46e618692ef85321",
          "url": "https://github.com/Rullst/Rullst/commit/f1d78028568cee152f453c1b8bca96b994ac21c0"
        },
        "date": 1788396657969,
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
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 417,
            "range": "± 1",
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
            "value": 190,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1599331,
            "range": "± 46396",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 64601,
            "range": "± 1372",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 70256,
            "range": "± 1458",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 63599,
            "range": "± 988",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 75684,
            "range": "± 1546",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 65401,
            "range": "± 1310",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 75630,
            "range": "± 1230",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 146153,
            "range": "± 3033",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 255948,
            "range": "± 6314",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 109788,
            "range": "± 2519",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 6717,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 120122,
            "range": "± 3324",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 109950,
            "range": "± 3563",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 7464,
            "range": "± 75",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 121208,
            "range": "± 3711",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 109532,
            "range": "± 3133",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 1404,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 108901,
            "range": "± 3266",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 116401,
            "range": "± 4700",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 10736,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 120223,
            "range": "± 3314",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 406028,
            "range": "± 19597",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 49867,
            "range": "± 5847",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 258785,
            "range": "± 16125",
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
          "id": "9e84fcb6a684a97643b1ef19b405d0d9bb66b24b",
          "message": "feat(capital): classify provider gateway failures",
          "timestamp": "2026-09-02T22:40:36-03:00",
          "tree_id": "0a7775cf4b83fc191afef02da6f037f80396d92e",
          "url": "https://github.com/Rullst/Rullst/commit/9e84fcb6a684a97643b1ef19b405d0d9bb66b24b"
        },
        "date": 1788400360173,
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 371,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 209,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 3666480,
            "range": "± 634014",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 92655,
            "range": "± 1274",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 96612,
            "range": "± 1872",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 90531,
            "range": "± 4309",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 100717,
            "range": "± 1072",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 91160,
            "range": "± 3125",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 100898,
            "range": "± 2254",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 145557,
            "range": "± 1353",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 247050,
            "range": "± 3903",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 127294,
            "range": "± 3671",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8227,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 142165,
            "range": "± 5319",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 127916,
            "range": "± 3158",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9024,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 144998,
            "range": "± 4407",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 126084,
            "range": "± 4369",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2749,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 126323,
            "range": "± 6428",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 132842,
            "range": "± 3482",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12181,
            "range": "± 78",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 142614,
            "range": "± 4488",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 527835,
            "range": "± 26565",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 53685,
            "range": "± 4556",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 326688,
            "range": "± 22336",
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
          "id": "95ea6caf6bd5cd00f684de1a4570518e82a7f889",
          "message": "feat(capital): add authenticated fiscal command journal",
          "timestamp": "2026-09-02T23:25:54-03:00",
          "tree_id": "9c46d7a097ec35b8c887faa3c4627ade0b248ed5",
          "url": "https://github.com/Rullst/Rullst/commit/95ea6caf6bd5cd00f684de1a4570518e82a7f889"
        },
        "date": 1788403023147,
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
            "value": 298,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 226,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 128,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 3996398,
            "range": "± 21501910",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 60550,
            "range": "± 1870",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 64200,
            "range": "± 2112",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 61133,
            "range": "± 1731",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 70893,
            "range": "± 1487",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 61738,
            "range": "± 2128",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 69631,
            "range": "± 2032",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 122742,
            "range": "± 11154",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 216421,
            "range": "± 16562",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 150083,
            "range": "± 19399",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 4638,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 172572,
            "range": "± 26219",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 149127,
            "range": "± 23241",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 4953,
            "range": "± 116",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 173646,
            "range": "± 18355",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 164030,
            "range": "± 16344",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 929,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 166472,
            "range": "± 16912",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 170633,
            "range": "± 22695",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 7221,
            "range": "± 141",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 178799,
            "range": "± 18418",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 884644,
            "range": "± 395154",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 177035,
            "range": "± 100602",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 765276,
            "range": "± 435276",
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
          "id": "74e1b827b1ab7a7d804dfd73011cf2ca439d8a2e",
          "message": "test(cli): verify generated project recovery matrices",
          "timestamp": "2026-09-03T02:05:52-03:00",
          "tree_id": "8a26c63c6c7f3c0b32cb6e28b3e90b55f87ff1fc",
          "url": "https://github.com/Rullst/Rullst/commit/74e1b827b1ab7a7d804dfd73011cf2ca439d8a2e"
        },
        "date": 1788412869675,
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
            "value": 51,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 526,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 404,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 211,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1760504,
            "range": "± 95809",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 76388,
            "range": "± 4490",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 80647,
            "range": "± 1188",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 76904,
            "range": "± 1297",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 83999,
            "range": "± 2544",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 77155,
            "range": "± 1141",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 85936,
            "range": "± 1582",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155358,
            "range": "± 2681",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 259204,
            "range": "± 25699",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 106561,
            "range": "± 1977",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8657,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 116270,
            "range": "± 2581",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 106692,
            "range": "± 2115",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9477,
            "range": "± 693",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 120314,
            "range": "± 3276",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 107605,
            "range": "± 1909",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3326,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 108303,
            "range": "± 1855",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 111566,
            "range": "± 2071",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12829,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 119141,
            "range": "± 8281",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 429242,
            "range": "± 21484",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 52765,
            "range": "± 473",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 273763,
            "range": "± 11641",
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
          "id": "ce2c683959077907e0cfc1e3f72a2b33c7a87b7e",
          "message": "feat(studio): add bounded distributed diagnostics",
          "timestamp": "2026-09-03T04:25:16-03:00",
          "tree_id": "f852c289bd9d5537c00e61dfee05487d7cb6ed9a",
          "url": "https://github.com/Rullst/Rullst/commit/ce2c683959077907e0cfc1e3f72a2b33c7a87b7e"
        },
        "date": 1788421173548,
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
            "value": 51,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 537,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 399,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 208,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1830761,
            "range": "± 163344",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 78210,
            "range": "± 1239",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 83407,
            "range": "± 1023",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 78478,
            "range": "± 2496",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 86946,
            "range": "± 1145",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 78303,
            "range": "± 1323",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 88609,
            "range": "± 1221",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 156365,
            "range": "± 2543",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 269530,
            "range": "± 3966",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 109996,
            "range": "± 2804",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8657,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 120736,
            "range": "± 2206",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 108767,
            "range": "± 2307",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9331,
            "range": "± 93",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 124790,
            "range": "± 3617",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 114459,
            "range": "± 2601",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3318,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 111849,
            "range": "± 3653",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 118485,
            "range": "± 4050",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12895,
            "range": "± 47",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 121792,
            "range": "± 3983",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 441790,
            "range": "± 18303",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 51842,
            "range": "± 447",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 284997,
            "range": "± 12509",
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
          "id": "b58ba7cc07ebc8410c70e854683ff6844c01e37e",
          "message": "feat(core): add lifecycle-aware request draining",
          "timestamp": "2026-09-03T06:16:27-03:00",
          "tree_id": "be96d1f527ec38acd1e1b23c612e8b01fa42d82e",
          "url": "https://github.com/Rullst/Rullst/commit/b58ba7cc07ebc8410c70e854683ff6844c01e37e"
        },
        "date": 1788427824616,
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
            "value": 17,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 51,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 534,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 406,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 210,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1709420,
            "range": "± 92602",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 78352,
            "range": "± 1182",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 81671,
            "range": "± 969",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 76893,
            "range": "± 1182",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 86582,
            "range": "± 2437",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 77970,
            "range": "± 957",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 86169,
            "range": "± 1771",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154938,
            "range": "± 1734",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 256923,
            "range": "± 3941",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 108257,
            "range": "± 2346",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8667,
            "range": "± 123",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 118747,
            "range": "± 2483",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 108704,
            "range": "± 3149",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9347,
            "range": "± 165",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 119983,
            "range": "± 6243",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 109411,
            "range": "± 1933",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3318,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 109483,
            "range": "± 3922",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 113624,
            "range": "± 3770",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12816,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 120761,
            "range": "± 5130",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 439734,
            "range": "± 31806",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 52472,
            "range": "± 717",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 280012,
            "range": "± 17521",
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
          "id": "28e2cea9b8aa63ff01156dfb02c29405464a6c17",
          "message": "feat(orm): add authenticated document recovery",
          "timestamp": "2026-09-03T09:57:16-03:00",
          "tree_id": "15481e1f40d4d08ff10f275ba37adf90a9ebe682",
          "url": "https://github.com/Rullst/Rullst/commit/28e2cea9b8aa63ff01156dfb02c29405464a6c17"
        },
        "date": 1788440690705,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 8,
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
            "value": 419,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 316,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 201,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2917146,
            "range": "± 752453",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 66234,
            "range": "± 1193",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 71267,
            "range": "± 1547",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 67038,
            "range": "± 1149",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 77035,
            "range": "± 1282",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 68448,
            "range": "± 1122",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 78250,
            "range": "± 1844",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155133,
            "range": "± 4624",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 283923,
            "range": "± 7687",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 122812,
            "range": "± 5900",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 6471,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 132638,
            "range": "± 5254",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 124200,
            "range": "± 4686",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 7078,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 129568,
            "range": "± 4913",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 124096,
            "range": "± 5316",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 1320,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 126447,
            "range": "± 7537",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 130816,
            "range": "± 4585",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 10111,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 133170,
            "range": "± 6079",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 547688,
            "range": "± 103338",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 106439,
            "range": "± 20246",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 364937,
            "range": "± 102356",
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
          "id": "fbca1ed0aaa2e833f3f0ae38e42249bc8505369e",
          "message": "docs(coverage): pin audited codecov checkpoint",
          "timestamp": "2026-09-03T11:14:33-03:00",
          "tree_id": "0d6d40ed238c782a7c912cf97f320bf9a01431f8",
          "url": "https://github.com/Rullst/Rullst/commit/fbca1ed0aaa2e833f3f0ae38e42249bc8505369e"
        },
        "date": 1788445352785,
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
            "value": 477,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 380,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 217,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2319098,
            "range": "± 322604",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 100570,
            "range": "± 11056",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 107000,
            "range": "± 5636",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 99900,
            "range": "± 1999",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 108212,
            "range": "± 2602",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 99236,
            "range": "± 7818",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 108027,
            "range": "± 6479",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155757,
            "range": "± 3191",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 249825,
            "range": "± 12714",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 134678,
            "range": "± 6047",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8151,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 148178,
            "range": "± 6388",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 134885,
            "range": "± 7391",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9008,
            "range": "± 131",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 147582,
            "range": "± 6334",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 134381,
            "range": "± 4469",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2761,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 133717,
            "range": "± 4965",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 139117,
            "range": "± 6221",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12072,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 147582,
            "range": "± 7148",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 563873,
            "range": "± 41088",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 49085,
            "range": "± 2685",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 335750,
            "range": "± 15553",
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
          "id": "d25f08e6dd5dfdebc8282d4df4916839f44fd04c",
          "message": "chore(release): update toolchain and coverage status",
          "timestamp": "2026-09-03T11:33:00-03:00",
          "tree_id": "a90ae8197584003a7df35369394ba1af0b7a4488",
          "url": "https://github.com/Rullst/Rullst/commit/d25f08e6dd5dfdebc8282d4df4916839f44fd04c"
        },
        "date": 1788446795461,
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
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 396,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 209,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2137499,
            "range": "± 74010",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 99373,
            "range": "± 1637",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 105460,
            "range": "± 2160",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 98607,
            "range": "± 2591",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105349,
            "range": "± 1612",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 96156,
            "range": "± 2322",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 105424,
            "range": "± 1364",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155759,
            "range": "± 3796",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 253150,
            "range": "± 4394",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 134073,
            "range": "± 5204",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8204,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 149677,
            "range": "± 4729",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 134832,
            "range": "± 4692",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 8952,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 151278,
            "range": "± 5226",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 129736,
            "range": "± 4868",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2781,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 133220,
            "range": "± 5697",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 137583,
            "range": "± 5032",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12234,
            "range": "± 54",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 148301,
            "range": "± 5030",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 542007,
            "range": "± 78811",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 50343,
            "range": "± 696",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 336232,
            "range": "± 21842",
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
          "id": "e61077026114cc168dd4b0862703d8d5db3f35d8",
          "message": "fix(ci): address security scan and Windows lint",
          "timestamp": "2026-09-03T15:59:50-03:00",
          "tree_id": "432b2fea4801facb84eb7859631b5bd47763c07c",
          "url": "https://github.com/Rullst/Rullst/commit/e61077026114cc168dd4b0862703d8d5db3f35d8"
        },
        "date": 1788462907461,
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
            "value": 47,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 472,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 374,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 207,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2280214,
            "range": "± 125085",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 100833,
            "range": "± 1001",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 106829,
            "range": "± 1296",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 98212,
            "range": "± 2188",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105905,
            "range": "± 1634",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 96127,
            "range": "± 2820",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 107235,
            "range": "± 2217",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 155451,
            "range": "± 1805",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 248524,
            "range": "± 1997",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 134889,
            "range": "± 5616",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8297,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 145897,
            "range": "± 5607",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 131532,
            "range": "± 7755",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9137,
            "range": "± 222",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 147810,
            "range": "± 5633",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 132904,
            "range": "± 5982",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2796,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 132080,
            "range": "± 3612",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 137563,
            "range": "± 10195",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12300,
            "range": "± 98",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 149735,
            "range": "± 8108",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 544794,
            "range": "± 30248",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 48810,
            "range": "± 1580",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 329613,
            "range": "± 20117",
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
          "id": "837494db69e2456e49d2c3dad9a5ad906a3e4039",
          "message": "test(cli): clean generated scaffold artifacts",
          "timestamp": "2026-09-03T16:25:02-03:00",
          "tree_id": "50821fd40779e0e1a203fcb31e05880564802133",
          "url": "https://github.com/Rullst/Rullst/commit/837494db69e2456e49d2c3dad9a5ad906a3e4039"
        },
        "date": 1788464010442,
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
            "value": 11,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 364,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 271,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 164,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2339030,
            "range": "± 181003",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 78196,
            "range": "± 1902",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 64708,
            "range": "± 1286",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 71530,
            "range": "± 1539",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 70657,
            "range": "± 1206",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 62435,
            "range": "± 983",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 70620,
            "range": "± 1071",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 145361,
            "range": "± 4679",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 258958,
            "range": "± 10049",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 118410,
            "range": "± 4902",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 5577,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 126373,
            "range": "± 6506",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 119250,
            "range": "± 4876",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 6067,
            "range": "± 62",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 126515,
            "range": "± 6867",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 118584,
            "range": "± 5759",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 1144,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 123125,
            "range": "± 8229",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 123940,
            "range": "± 4567",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 8668,
            "range": "± 106",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 129000,
            "range": "± 4414",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 531570,
            "range": "± 68436",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 92935,
            "range": "± 11385",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 343261,
            "range": "± 77005",
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
          "id": "c7e8bda4ec0b557e57da10dabcdc32e55ebdb35a",
          "message": "test(cli): lock generated offline builds",
          "timestamp": "2026-09-03T17:37:09-03:00",
          "tree_id": "81d53c088b76ffc431c585b81321c94bea54d2b8",
          "url": "https://github.com/Rullst/Rullst/commit/c7e8bda4ec0b557e57da10dabcdc32e55ebdb35a"
        },
        "date": 1788468562029,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 10,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 404,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 310,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 174,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2275852,
            "range": "± 8080196",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 61547,
            "range": "± 866",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 65113,
            "range": "± 3034",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 59739,
            "range": "± 1233",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 68336,
            "range": "± 1386",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 61306,
            "range": "± 1054",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 65647,
            "range": "± 6058",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 121701,
            "range": "± 1660",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 200878,
            "range": "± 3274",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 84384,
            "range": "± 4209",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 6803,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 93721,
            "range": "± 1700",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 84812,
            "range": "± 1638",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 7405,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 95065,
            "range": "± 2271",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 86053,
            "range": "± 1305",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2572,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 86660,
            "range": "± 5566",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 89518,
            "range": "± 3884",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 9879,
            "range": "± 62",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 93823,
            "range": "± 3226",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 405564,
            "range": "± 223719",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 102887,
            "range": "± 874602",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 273020,
            "range": "± 182163",
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
          "id": "f321169fbb6ab4cec913d8524c0348d0e7b3b241",
          "message": "feat(ai): add bounded streaming cancellation",
          "timestamp": "2026-09-03T18:00:15-03:00",
          "tree_id": "7837c0c7d9968d42ec4a7452abd3d15684c974ef",
          "url": "https://github.com/Rullst/Rullst/commit/f321169fbb6ab4cec913d8524c0348d0e7b3b241"
        },
        "date": 1788469653199,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 6,
            "range": "± 1",
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 307,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 230,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 142,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 5236459,
            "range": "± 36296728",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 66717,
            "range": "± 1867",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 68754,
            "range": "± 1776",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 65446,
            "range": "± 1866",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 77745,
            "range": "± 1683",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 67931,
            "range": "± 1469",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 78705,
            "range": "± 1971",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 134012,
            "range": "± 7023",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 238237,
            "range": "± 12377",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 158144,
            "range": "± 24607",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 5087,
            "range": "± 129",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 162304,
            "range": "± 21392",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 160905,
            "range": "± 17283",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 5647,
            "range": "± 128",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 172382,
            "range": "± 17936",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 162859,
            "range": "± 7327",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 970,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 159920,
            "range": "± 18476",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 169361,
            "range": "± 23608",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 8058,
            "range": "± 234",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 175053,
            "range": "± 23566",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 655870,
            "range": "± 1746472",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 177964,
            "range": "± 311053",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 828235,
            "range": "± 599965",
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
          "id": "34efe8f649fc2049cfe6666639557b9c1cb971b6",
          "message": "feat(ai): add authenticated audit delivery",
          "timestamp": "2026-09-03T18:19:17-03:00",
          "tree_id": "1e5f815cc7b022f7cf389f88634750bfe00f6471",
          "url": "https://github.com/Rullst/Rullst/commit/34efe8f649fc2049cfe6666639557b9c1cb971b6"
        },
        "date": 1788470835558,
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
            "value": 48,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 516,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 399,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 204,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1710361,
            "range": "± 66257",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 76836,
            "range": "± 1072",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 81449,
            "range": "± 1350",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 76949,
            "range": "± 3192",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 85512,
            "range": "± 1228",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 79146,
            "range": "± 676",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 87124,
            "range": "± 863",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 159618,
            "range": "± 2055",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 261255,
            "range": "± 3545",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 106897,
            "range": "± 1977",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 9010,
            "range": "± 230",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 118534,
            "range": "± 2988",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 106746,
            "range": "± 1689",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9638,
            "range": "± 83",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 120306,
            "range": "± 2663",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 109920,
            "range": "± 4617",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3327,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 110266,
            "range": "± 3402",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 112467,
            "range": "± 1923",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12906,
            "range": "± 80",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 124871,
            "range": "± 6398",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 428233,
            "range": "± 19705",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 52012,
            "range": "± 2479",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 275707,
            "range": "± 10610",
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
          "id": "26b69982a74d618e768b062065313e823ab3dfef",
          "message": "feat(ai): add bounded adaptive evaluations",
          "timestamp": "2026-09-03T18:42:51-03:00",
          "tree_id": "04032e9f8469b2b811182ca55d67cd2995468eae",
          "url": "https://github.com/Rullst/Rullst/commit/26b69982a74d618e768b062065313e823ab3dfef"
        },
        "date": 1788472221133,
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
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 517,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 398,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 211,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1673285,
            "range": "± 138182",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 78925,
            "range": "± 1196",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 82743,
            "range": "± 745",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 78239,
            "range": "± 834",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 87565,
            "range": "± 1843",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 78753,
            "range": "± 1467",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 87415,
            "range": "± 936",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 157601,
            "range": "± 2204",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 256437,
            "range": "± 2340",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 107025,
            "range": "± 2467",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8722,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 118676,
            "range": "± 3063",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 107474,
            "range": "± 1871",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9405,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 120272,
            "range": "± 2565",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 109964,
            "range": "± 1949",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3322,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 108533,
            "range": "± 1655",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 112511,
            "range": "± 2141",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12845,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 122499,
            "range": "± 5937",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 437670,
            "range": "± 23903",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 51002,
            "range": "± 983",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 276155,
            "range": "± 11861",
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
          "id": "67b89517575ea37bbb4b089d9a154f0a9f03e335",
          "message": "fix(orm): pin broken tinyvec resolution",
          "timestamp": "2026-09-03T18:59:40-03:00",
          "tree_id": "00f83ccb27f3afff1a8da7a95e5729345f861f9c",
          "url": "https://github.com/Rullst/Rullst/commit/67b89517575ea37bbb4b089d9a154f0a9f03e335"
        },
        "date": 1788473550920,
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
            "value": 470,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 387,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 211,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2088215,
            "range": "± 108953",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 98458,
            "range": "± 1843",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 103259,
            "range": "± 2166",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 97680,
            "range": "± 2235",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 104716,
            "range": "± 1382",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 95224,
            "range": "± 2701",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 105035,
            "range": "± 2349",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 154413,
            "range": "± 5137",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 251271,
            "range": "± 4700",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 129427,
            "range": "± 5133",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8136,
            "range": "± 111",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 146661,
            "range": "± 6537",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 130935,
            "range": "± 6447",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 8987,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 144074,
            "range": "± 5740",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 128939,
            "range": "± 4071",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2688,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 131594,
            "range": "± 4924",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 134786,
            "range": "± 5416",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12001,
            "range": "± 64",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 144093,
            "range": "± 6759",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 512199,
            "range": "± 33896",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 49513,
            "range": "± 1149",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 326761,
            "range": "± 23439",
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
          "id": "194b5c8a80716962bcd63fe8777adcb083bea260",
          "message": "fix(ci): constrain semver baseline resolver",
          "timestamp": "2026-09-03T19:52:22-03:00",
          "tree_id": "d50d57e4c96e16a63b213b3958be0e75ddaadb93",
          "url": "https://github.com/Rullst/Rullst/commit/194b5c8a80716962bcd63fe8777adcb083bea260"
        },
        "date": 1788476409627,
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
            "value": 11,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 360,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 270,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 162,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 3070319,
            "range": "± 1099209",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 74374,
            "range": "± 1314",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 77268,
            "range": "± 1140",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 64780,
            "range": "± 1345",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 88571,
            "range": "± 5576",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 72756,
            "range": "± 1191",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 83368,
            "range": "± 2881",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 144504,
            "range": "± 6204",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 256676,
            "range": "± 9943",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 118574,
            "range": "± 7033",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 5489,
            "range": "± 91",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 128496,
            "range": "± 7427",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 126182,
            "range": "± 7254",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 6029,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 130126,
            "range": "± 4803",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 121112,
            "range": "± 6832",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 1146,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 120408,
            "range": "± 5104",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 125801,
            "range": "± 8222",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 8598,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 132243,
            "range": "± 3876",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 553133,
            "range": "± 68965",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 96527,
            "range": "± 15009",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 344208,
            "range": "± 80382",
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
          "id": "7943211fcdef95611cf205c25b2aa951f624f243",
          "message": "test(cli): raise whole-repository coverage",
          "timestamp": "2026-09-03T20:58:18-03:00",
          "tree_id": "104b3f24cf040edf73e20d87cf44042f72c9f4cd",
          "url": "https://github.com/Rullst/Rullst/commit/7943211fcdef95611cf205c25b2aa951f624f243"
        },
        "date": 1788480701736,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 517,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 402,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 216,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1865165,
            "range": "± 221088",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 77756,
            "range": "± 5351",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 80815,
            "range": "± 1695",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 77140,
            "range": "± 1323",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 86263,
            "range": "± 987",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 77389,
            "range": "± 1568",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 84890,
            "range": "± 1430",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153498,
            "range": "± 5928",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 255340,
            "range": "± 6490",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 109164,
            "range": "± 4612",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8783,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 118855,
            "range": "± 5148",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 107839,
            "range": "± 2186",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9568,
            "range": "± 165",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 119795,
            "range": "± 2110",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 108277,
            "range": "± 1902",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3326,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 109877,
            "range": "± 2750",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 112780,
            "range": "± 4143",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12942,
            "range": "± 139",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 120204,
            "range": "± 7071",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 488041,
            "range": "± 44511",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 63849,
            "range": "± 6399",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 319266,
            "range": "± 29533",
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
          "id": "704b6d4dd3a4731d5eedd3c70677bd3f2fc97def",
          "message": "fix(ci): stabilize cross-platform CLI coverage",
          "timestamp": "2026-09-03T21:57:39-03:00",
          "tree_id": "e039358b703cdddcc4b9ffd49d08f483f3add2d2",
          "url": "https://github.com/Rullst/Rullst/commit/704b6d4dd3a4731d5eedd3c70677bd3f2fc97def"
        },
        "date": 1788484153557,
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
            "value": 11,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 39,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 364,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 274,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 165,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 3358479,
            "range": "± 4218693",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 58903,
            "range": "± 2359",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 62980,
            "range": "± 1380",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 58806,
            "range": "± 1464",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 79372,
            "range": "± 6171",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 60551,
            "range": "± 1051",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 74260,
            "range": "± 1407",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 140444,
            "range": "± 5962",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 251240,
            "range": "± 8569",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 120572,
            "range": "± 7777",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 6035,
            "range": "± 57",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 122963,
            "range": "± 5145",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 117124,
            "range": "± 4313",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 6571,
            "range": "± 72",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 130510,
            "range": "± 7742",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 114260,
            "range": "± 3577",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 1180,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 115625,
            "range": "± 7082",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 125224,
            "range": "± 4193",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 9463,
            "range": "± 78",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 127362,
            "range": "± 3292",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 507876,
            "range": "± 568708",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 76413,
            "range": "± 43202",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 390729,
            "range": "± 296469",
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
          "id": "84d17f72e9a5ac66d05301486742ad52f56cd60e",
          "message": "docs(release): record verified coverage gate",
          "timestamp": "2026-09-03T22:34:38-03:00",
          "tree_id": "328850642e071f51d6940a5cc1e84a1635638cab",
          "url": "https://github.com/Rullst/Rullst/commit/84d17f72e9a5ac66d05301486742ad52f56cd60e"
        },
        "date": 1788486474969,
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
            "value": 47,
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
            "value": 385,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 222,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2309437,
            "range": "± 232999",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 99904,
            "range": "± 1276",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 104669,
            "range": "± 1539",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 98898,
            "range": "± 2826",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 105820,
            "range": "± 2025",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 96927,
            "range": "± 3044",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 106245,
            "range": "± 2002",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 156456,
            "range": "± 4777",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 248846,
            "range": "± 5151",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 132098,
            "range": "± 4551",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8307,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 147678,
            "range": "± 4916",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 132352,
            "range": "± 6351",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9084,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 148178,
            "range": "± 5728",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 131899,
            "range": "± 5814",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2803,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 135772,
            "range": "± 5540",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 136551,
            "range": "± 7061",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12492,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 149583,
            "range": "± 7390",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 546314,
            "range": "± 32074",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 51149,
            "range": "± 2207",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 332340,
            "range": "± 20182",
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
          "id": "2f160d7557b716c35b56e60b7db2934a301dc1a3",
          "message": "fix(ci): harden release test campaigns",
          "timestamp": "2026-09-04T00:58:24-03:00",
          "tree_id": "300ce842f5f7489c121ef7092f33fa800fb466e0",
          "url": "https://github.com/Rullst/Rullst/commit/2f160d7557b716c35b56e60b7db2934a301dc1a3"
        },
        "date": 1788495209700,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/qualified",
            "value": 18,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/validate_identifier/invalid",
            "value": 56,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 606,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 463,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 250,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1996857,
            "range": "± 35275",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 88893,
            "range": "± 2352",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 93624,
            "range": "± 1909",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 89201,
            "range": "± 923",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 99054,
            "range": "± 1012",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 88706,
            "range": "± 904",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 99327,
            "range": "± 1556",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 173173,
            "range": "± 3349",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 290058,
            "range": "± 4088",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 121195,
            "range": "± 5172",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 10251,
            "range": "± 87",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 133962,
            "range": "± 2501",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 122381,
            "range": "± 1972",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 10985,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 134710,
            "range": "± 6485",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 123414,
            "range": "± 2067",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3861,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 122573,
            "range": "± 1893",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 126645,
            "range": "± 1741",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 15239,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 133954,
            "range": "± 4678",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 492248,
            "range": "± 26087",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 60380,
            "range": "± 1234",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 309279,
            "range": "± 15008",
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
          "id": "21871c24ad38b5988e4d0fb39acab01aba5d29e9",
          "message": "fix(auth): normalize Windows SQLite file URLs",
          "timestamp": "2026-09-04T02:49:27-03:00",
          "tree_id": "b292bccc9c9c3b5bbf1399c3672fcc95f00bcb6c",
          "url": "https://github.com/Rullst/Rullst/commit/21871c24ad38b5988e4d0fb39acab01aba5d29e9"
        },
        "date": 1788501385670,
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
            "value": 47,
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
            "value": 391,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 208,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2169603,
            "range": "± 117952",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 98512,
            "range": "± 1120",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 103962,
            "range": "± 1451",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 96545,
            "range": "± 3284",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 104676,
            "range": "± 1609",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 94911,
            "range": "± 2409",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 104736,
            "range": "± 1156",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 152301,
            "range": "± 1066",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 243105,
            "range": "± 2478",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 131381,
            "range": "± 5622",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8552,
            "range": "± 155",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 146042,
            "range": "± 3902",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 130431,
            "range": "± 5200",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9312,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 150730,
            "range": "± 5959",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 129978,
            "range": "± 4639",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2771,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 133464,
            "range": "± 3982",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 135768,
            "range": "± 5667",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12192,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 148462,
            "range": "± 7927",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 544881,
            "range": "± 28576",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 49362,
            "range": "± 1548",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 321631,
            "range": "± 18504",
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
          "id": "eec6ac711af806f41a755035370566d3336cbbd9",
          "message": "fix(connect): normalize Windows SQLite file URLs",
          "timestamp": "2026-09-04T04:30:46-03:00",
          "tree_id": "8129f94aa0b8325f1a7ba4fc7fbce297194ee16d",
          "url": "https://github.com/Rullst/Rullst/commit/eec6ac711af806f41a755035370566d3336cbbd9"
        },
        "date": 1788507830159,
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
            "value": 50,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 518,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 414,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 222,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1716319,
            "range": "± 97397",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 81556,
            "range": "± 831",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 85633,
            "range": "± 2552",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 78918,
            "range": "± 1138",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 87819,
            "range": "± 3694",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 79258,
            "range": "± 2050",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 89247,
            "range": "± 789",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 152614,
            "range": "± 3251",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 255313,
            "range": "± 3598",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 110326,
            "range": "± 2366",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 9107,
            "range": "± 189",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 121881,
            "range": "± 2266",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 111263,
            "range": "± 1553",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9851,
            "range": "± 108",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 122444,
            "range": "± 3390",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 112474,
            "range": "± 2050",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3042,
            "range": "± 124",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 112623,
            "range": "± 12097",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 115112,
            "range": "± 2968",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12785,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 120885,
            "range": "± 3327",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 449718,
            "range": "± 23117",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 51463,
            "range": "± 895",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 280684,
            "range": "± 12709",
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
          "id": "e6f0d559e1c6e5af01d87dffb551b8caa9daed29",
          "message": "fix(sqlite): normalize Windows store paths",
          "timestamp": "2026-09-04T06:16:43-03:00",
          "tree_id": "ec7045a8807c97a3aa6362a1c5178989f7dd81ab",
          "url": "https://github.com/Rullst/Rullst/commit/e6f0d559e1c6e5af01d87dffb551b8caa9daed29"
        },
        "date": 1788513864113,
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
            "value": 28,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 302,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 227,
            "range": "± 10",
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
            "value": 4090763,
            "range": "± 61377835",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 65094,
            "range": "± 1991",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 71161,
            "range": "± 3697",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 62507,
            "range": "± 1267",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 71840,
            "range": "± 1877",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 66040,
            "range": "± 2699",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 73068,
            "range": "± 1968",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 137358,
            "range": "± 13866",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 234860,
            "range": "± 13084",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 161880,
            "range": "± 18240",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 4537,
            "range": "± 190",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 174234,
            "range": "± 15693",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 169741,
            "range": "± 16090",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 4942,
            "range": "± 288",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 173557,
            "range": "± 25326",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 168949,
            "range": "± 16193",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 936,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 161564,
            "range": "± 26640",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 165019,
            "range": "± 27424",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 7295,
            "range": "± 385",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 178880,
            "range": "± 19687",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 731943,
            "range": "± 1504695",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 232197,
            "range": "± 401974",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 623872,
            "range": "± 1080720",
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
          "id": "e35c9c5e564bca1998c7b3ee3d63b81f1cf339ad",
          "message": "fix(test): make release checks deterministic on Windows",
          "timestamp": "2026-09-04T07:57:53-03:00",
          "tree_id": "e1fcbd8d0aeb563cb4045c3f8f5adf111882cc17",
          "url": "https://github.com/Rullst/Rullst/commit/e35c9c5e564bca1998c7b3ee3d63b81f1cf339ad"
        },
        "date": 1788520260657,
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
            "value": 48,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/to_json/user",
            "value": 515,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 400,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 211,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 1813792,
            "range": "± 124795",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 77870,
            "range": "± 1175",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 81294,
            "range": "± 1967",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 78070,
            "range": "± 1000",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 87320,
            "range": "± 4080",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 77604,
            "range": "± 2093",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 86095,
            "range": "± 1772",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 157008,
            "range": "± 2216",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 257959,
            "range": "± 3619",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 106372,
            "range": "± 1964",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8868,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 117453,
            "range": "± 2347",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 106124,
            "range": "± 1477",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9615,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 118607,
            "range": "± 2417",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 108501,
            "range": "± 1542",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 3313,
            "range": "± 189",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 107415,
            "range": "± 1863",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 111985,
            "range": "± 1385",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12945,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 121793,
            "range": "± 4761",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 435805,
            "range": "± 22909",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 52007,
            "range": "± 804",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 277229,
            "range": "± 15849",
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
          "id": "ac516494771e79c55e5f8ae808d7fc52fb75576e",
          "message": "fix(ci): make manual verification evidence executable",
          "timestamp": "2026-09-04T10:56:37-03:00",
          "tree_id": "db47deb3667d2137074d1d11ae5e8b4c8c1a9419",
          "url": "https://github.com/Rullst/Rullst/commit/ac516494771e79c55e5f8ae808d7fc52fb75576e"
        },
        "date": 1788530657372,
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
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 387,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 210,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2267082,
            "range": "± 156000",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 91257,
            "range": "± 1189",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 95485,
            "range": "± 1184",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 91398,
            "range": "± 3168",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 99983,
            "range": "± 1920",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 89677,
            "range": "± 3285",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 100256,
            "range": "± 2001",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 142569,
            "range": "± 1865",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 240783,
            "range": "± 3050",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 127728,
            "range": "± 3689",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 8170,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 143184,
            "range": "± 7471",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 127793,
            "range": "± 2961",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 9035,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 143507,
            "range": "± 6198",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 127661,
            "range": "± 5270",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 2681,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 127944,
            "range": "± 6255",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 132573,
            "range": "± 2606",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 12163,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 145330,
            "range": "± 4725",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 533448,
            "range": "± 31215",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 49862,
            "range": "± 1428",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 325381,
            "range": "± 18628",
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
          "id": "3373e8236c5f33c1184c16b2bb5756c5775dca67",
          "message": "fix(ci): isolate bounded Kani proofs",
          "timestamp": "2026-09-04T12:31:28-03:00",
          "tree_id": "10b5aab5776795a921f02e684b53a2c89a7df565",
          "url": "https://github.com/Rullst/Rullst/commit/3373e8236c5f33c1184c16b2bb5756c5775dca67"
        },
        "date": 1788536662553,
        "tool": "cargo",
        "benches": [
          {
            "name": "cpu/validate_identifier/short",
            "value": 8,
            "range": "± 1",
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
            "value": 417,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/from_json/user",
            "value": 317,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cpu/query_builder/build",
            "value": 197,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/save/insert",
            "value": 2606857,
            "range": "± 487753",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/find_by_id",
            "value": 65226,
            "range": "± 1225",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/where_eq_first",
            "value": 71429,
            "range": "± 1113",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/count",
            "value": 66147,
            "range": "± 955",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/all_limit_10",
            "value": 77238,
            "range": "± 1271",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/1",
            "value": 67735,
            "range": "± 1000",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/10",
            "value": 76927,
            "range": "± 1592",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/50",
            "value": 153341,
            "range": "± 4387",
            "unit": "ns/iter"
          },
          {
            "name": "db_roundtrip/query/limit_n/100",
            "value": 283388,
            "range": "± 9469",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/rullst",
            "value": 121704,
            "range": "± 6735",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/diesel",
            "value": 6431,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/find_by_id/seaorm",
            "value": 128205,
            "range": "± 4412",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/rullst",
            "value": 124132,
            "range": "± 6973",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/diesel",
            "value": 7014,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/filter_email/seaorm",
            "value": 128221,
            "range": "± 6114",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/rullst",
            "value": 119730,
            "range": "± 4112",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/diesel",
            "value": 1301,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/count/seaorm",
            "value": 123476,
            "range": "± 7301",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/rullst",
            "value": 127095,
            "range": "± 6145",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/diesel",
            "value": 10055,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/list_10/seaorm",
            "value": 133971,
            "range": "± 4194",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/rullst",
            "value": 553364,
            "range": "± 101712",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/diesel",
            "value": 113468,
            "range": "± 18742",
            "unit": "ns/iter"
          },
          {
            "name": "orm_comparison/sqlite/insert_delete/seaorm",
            "value": 360349,
            "range": "± 92542",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}