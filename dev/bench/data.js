window.BENCHMARK_DATA = {
  "lastUpdate": 1782683492129,
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
      }
    ]
  }
}