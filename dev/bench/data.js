window.BENCHMARK_DATA = {
  "lastUpdate": 1782684886358,
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
          "id": "be83999cdd83c1d86cee2db4be3a765cd9a7f716",
          "message": "test(ci): desabilita isolamento do miri para permitir testes do reqwest",
          "timestamp": "2026-06-27T22:19:38-03:00",
          "tree_id": "a64c6581e98ccdecb8819923a7e63c6adb2389f1",
          "url": "https://github.com/Rullst/Rullst/commit/be83999cdd83c1d86cee2db4be3a765cd9a7f716"
        },
        "date": 1782609682372,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 748,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 1007,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "html_macro_static",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "html_macro_dynamic",
            "value": 641,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2327,
            "range": "± 63",
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
          "id": "9c619e7d6aa0704acecb1ceb0d5e68a074f1a600",
          "message": "ci: update actions/checkout to v4 and fix Miri UB detection",
          "timestamp": "2026-06-28T18:38:32-03:00",
          "tree_id": "1ca9fbd9bc47018c90fe2790c7a19aca289ffaa9",
          "url": "https://github.com/Rullst/Rullst/commit/9c619e7d6aa0704acecb1ceb0d5e68a074f1a600"
        },
        "date": 1782682821453,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 735,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 974,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "html_macro_static",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "html_macro_dynamic",
            "value": 625,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2278,
            "range": "± 37",
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
          "id": "6e0714bbc40346ccf2bdfe78431ae0a24d732b6a",
          "message": "ci: pin github actions by SHA to resolve scorecard alerts and restore missing workflows",
          "timestamp": "2026-06-28T18:49:45-03:00",
          "tree_id": "280e685981cd842bc35dd4814068cfd375710693",
          "url": "https://github.com/Rullst/Rullst/commit/6e0714bbc40346ccf2bdfe78431ae0a24d732b6a"
        },
        "date": 1782683491313,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 755,
            "range": "± 138",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 994,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "html_macro_static",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "html_macro_dynamic",
            "value": 658,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2337,
            "range": "± 43",
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
          "id": "0f775770fcbcee9cacbb9af3ee2f4102f1ff25b8",
          "message": "ci: fix workflow runners and configure spellcheck exceptions",
          "timestamp": "2026-06-28T18:58:24-03:00",
          "tree_id": "af778992de681e03a485777c305f7003c6e5830f",
          "url": "https://github.com/Rullst/Rullst/commit/0f775770fcbcee9cacbb9af3ee2f4102f1ff25b8"
        },
        "date": 1782684004501,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 769,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 1003,
            "range": "± 15",
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
            "value": 639,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2296,
            "range": "± 30",
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
          "id": "35c559422da3147039f6ecefde4c012482474091",
          "message": "ci: fix cargo-geiger virtual manifest error by adding workspace flag",
          "timestamp": "2026-06-28T19:04:42-03:00",
          "tree_id": "510cfb94503b522a4c6f8903ebc0fbe7c7a6c322",
          "url": "https://github.com/Rullst/Rullst/commit/35c559422da3147039f6ecefde4c012482474091"
        },
        "date": 1782684393537,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 743,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 975,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "html_macro_static",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "html_macro_dynamic",
            "value": 628,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2322,
            "range": "± 36",
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
          "id": "6d1d4b120604775bd980d8041516c15cc72599eb",
          "message": "ci: run cargo-geiger individually per package",
          "timestamp": "2026-06-28T19:13:10-03:00",
          "tree_id": "19e380da523b837e979cb8401ad09b39b9859dcd",
          "url": "https://github.com/Rullst/Rullst/commit/6d1d4b120604775bd980d8041516c15cc72599eb"
        },
        "date": 1782684885589,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 758,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 1002,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "html_macro_static",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "html_macro_dynamic",
            "value": 596,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2291,
            "range": "± 32",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}