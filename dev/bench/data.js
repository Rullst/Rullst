window.BENCHMARK_DATA = {
  "lastUpdate": 1782860377554,
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
          "id": "89ad873d37f36f140dff7a966efddeed5fd927e0",
          "message": "ci: replace cargo-geiger with ripgrep-based unsafe audit scan",
          "timestamp": "2026-06-28T19:22:36-03:00",
          "tree_id": "18a677056d6a174205e7841571ccbb9378d31be7",
          "url": "https://github.com/Rullst/Rullst/commit/89ad873d37f36f140dff7a966efddeed5fd927e0"
        },
        "date": 1782685484593,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 768,
            "range": "± 29",
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
            "value": 612,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2317,
            "range": "± 40",
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
          "id": "85e41336d82360e3a30d16814087e286d3471176",
          "message": "ci: fix Miri aws-lc-rs test, Kani MSRV conflict, upgrade all checkout to v4.3.1 (Node 24)",
          "timestamp": "2026-06-28T19:44:08-03:00",
          "tree_id": "13cd514d4acda1db44f314c5dbbc0e0b318fefc3",
          "url": "https://github.com/Rullst/Rullst/commit/85e41336d82360e3a30d16814087e286d3471176"
        },
        "date": 1782686856817,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 752,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 1021,
            "range": "± 24",
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
            "value": 640,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2322,
            "range": "± 28",
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
          "id": "c6d152350a73bd8995b7587427a87efeb45024b6",
          "message": "ci: upgrade all checkout to v5 (Node 24), skip crypto FFI in Miri, fix Kani no-harness exit",
          "timestamp": "2026-06-28T20:45:37-03:00",
          "tree_id": "25d5544afed6c20bc82f2a16119044aa36485eae",
          "url": "https://github.com/Rullst/Rullst/commit/c6d152350a73bd8995b7587427a87efeb45024b6"
        },
        "date": 1782690490772,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 738,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 976,
            "range": "± 17",
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
            "value": 619,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2373,
            "range": "± 62",
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
          "id": "f96deed88f719a272502f8783256b3c7b8022739",
          "message": "ci: fix Kani shell quoting error, use continue-on-error for missing harnesses",
          "timestamp": "2026-06-28T20:53:29-03:00",
          "tree_id": "f112df421bf44e24729be97e3a68cf0c272a5f6f",
          "url": "https://github.com/Rullst/Rullst/commit/f96deed88f719a272502f8783256b3c7b8022739"
        },
        "date": 1782690940344,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 734,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 973,
            "range": "± 12",
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
            "value": 659,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2305,
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
          "id": "554483153cc34d6c7bddeac3ebcf185daf0988e2",
          "message": "docs: update security audit table to include Matrix DB tests and reorder entries in README.md",
          "timestamp": "2026-06-28T21:18:01-03:00",
          "tree_id": "0e4672f05de571f93466abf035ccc2d958a1895e",
          "url": "https://github.com/Rullst/Rullst/commit/554483153cc34d6c7bddeac3ebcf185daf0988e2"
        },
        "date": 1782692389385,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 779,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 1003,
            "range": "± 16",
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
            "value": 601,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2319,
            "range": "± 20",
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
          "id": "80b9ba606138a74278fffa7bc73134ab0256548f",
          "message": "feat: add task scheduler module, Horizon dashboard routing, and initial auth/security boilerplate",
          "timestamp": "2026-06-29T13:01:11-03:00",
          "tree_id": "3f6f125257eabc18db64507d5e51368fd1aca5e6",
          "url": "https://github.com/Rullst/Rullst/commit/80b9ba606138a74278fffa7bc73134ab0256548f"
        },
        "date": 1782749000350,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 737,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 988,
            "range": "± 15",
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
            "value": 637,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2252,
            "range": "± 35",
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
          "id": "380758b7f376e37d965d58275516cdca690daa87",
          "message": "changelog update",
          "timestamp": "2026-06-29T18:03:50-03:00",
          "tree_id": "127a1f39f6aa120658da4882d31a67be7c1206f4",
          "url": "https://github.com/Rullst/Rullst/commit/380758b7f376e37d965d58275516cdca690daa87"
        },
        "date": 1782767216788,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 755,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 991,
            "range": "± 33",
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
            "value": 626,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2332,
            "range": "± 25",
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
          "id": "6454a06284528e61079f889993f8c7e2b52e552f",
          "message": "ok",
          "timestamp": "2026-06-29T18:09:59-03:00",
          "tree_id": "fc0ddf489e98d9e7f7d6923419460108ec895bcf",
          "url": "https://github.com/Rullst/Rullst/commit/6454a06284528e61079f889993f8c7e2b52e552f"
        },
        "date": 1782767529533,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 739,
            "range": "± 32",
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
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "html_macro_dynamic",
            "value": 663,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2323,
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
          "id": "3d4feca23c646823f5c30695464c15e5c0b1f73e",
          "message": "feat: implement CLI UI components and initialize foundational service modules for auth, database, scheduler, server, and storage",
          "timestamp": "2026-06-29T18:22:57-03:00",
          "tree_id": "94c313687a554458a48c8c2b25d2ef3b3a6633b3",
          "url": "https://github.com/Rullst/Rullst/commit/3d4feca23c646823f5c30695464c15e5c0b1f73e"
        },
        "date": 1782768297640,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 727,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 967,
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
            "value": 662,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2297,
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
          "id": "7fe94fbcaa2589cfc2a5df93d8c8894f1fe7b33d",
          "message": "feat: add capital module with billing provider traits and Stripe implementation",
          "timestamp": "2026-06-29T18:35:08-03:00",
          "tree_id": "7d2bfcca76bdbd0bf48fdc4547dd652db04ae3eb",
          "url": "https://github.com/Rullst/Rullst/commit/7fe94fbcaa2589cfc2a5df93d8c8894f1fe7b33d"
        },
        "date": 1782769012028,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 784,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 1023,
            "range": "± 11",
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
            "value": 620,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2384,
            "range": "± 132",
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
          "id": "7970f318a728d9f6aabac1965fb1977e81a6e044",
          "message": "chore: add ignore entry for RUSTSEC-2023-0071 in osv-scanner.toml",
          "timestamp": "2026-06-29T18:38:52-03:00",
          "tree_id": "4573a51f7e2c039e04c5601e54d1ec94300a5eef",
          "url": "https://github.com/Rullst/Rullst/commit/7970f318a728d9f6aabac1965fb1977e81a6e044"
        },
        "date": 1782769237601,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 759,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 975,
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
            "value": 642,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2302,
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
          "id": "649d77dda62eadc6b87a74baf92ad9dac7a31416",
          "message": "feat: add Horizon dashboard with Axum routing and HTMX-based job management",
          "timestamp": "2026-06-29T18:56:06-03:00",
          "tree_id": "54ed92104e1c28912416738278bd072861b69417",
          "url": "https://github.com/Rullst/Rullst/commit/649d77dda62eadc6b87a74baf92ad9dac7a31416"
        },
        "date": 1782770263303,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 774,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 1038,
            "range": "± 27",
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
            "value": 657,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2357,
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
          "id": "4808afd73958c2b87c37bb16936ccbe0c37349e6",
          "message": "feat: add Nexus core module with model reflection, registry, and Axum routing infrastructure",
          "timestamp": "2026-06-29T19:55:13-03:00",
          "tree_id": "c7155fa9f458607e15e732de35119769d85e74bf",
          "url": "https://github.com/Rullst/Rullst/commit/4808afd73958c2b87c37bb16936ccbe0c37349e6"
        },
        "date": 1782773809673,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 761,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 999,
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
            "value": 630,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2343,
            "range": "± 55",
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
          "id": "95609252df11f7a8f1954801512fd654bbc4787f",
          "message": "feat: initialize Nexus admin panel with core modules for schema management, security, and AI-driven studio interactions.",
          "timestamp": "2026-06-29T20:07:45-03:00",
          "tree_id": "7779578e5c8c879b68505fb3cea11a5e1fc6af95",
          "url": "https://github.com/Rullst/Rullst/commit/95609252df11f7a8f1954801512fd654bbc4787f"
        },
        "date": 1782774567716,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 737,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 1010,
            "range": "± 17",
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
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2349,
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
          "id": "9e976b6e0a4f3539c6056512373da2b9088e59e0",
          "message": "feat: implement adaptive backpressure middleware and token-bucket rate limiting for system resilience",
          "timestamp": "2026-06-29T20:56:32-03:00",
          "tree_id": "f0798fa7be0a54fa9e2bc3e7bf10e6cfdd91a6ec",
          "url": "https://github.com/Rullst/Rullst/commit/9e976b6e0a4f3539c6056512373da2b9088e59e0"
        },
        "date": 1782777490959,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 826,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 1093,
            "range": "± 19",
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
            "value": 643,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2375,
            "range": "± 31",
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
          "id": "8689c8709dd3e49e14980ccb64235b5cedd85ff3",
          "message": "feat: implement Server builder API and add GitHub Action to enforce zero-panic policy",
          "timestamp": "2026-06-29T21:13:14-03:00",
          "tree_id": "35ae80c13ae95be7b380eff966a368fc23cbad8c",
          "url": "https://github.com/Rullst/Rullst/commit/8689c8709dd3e49e14980ccb64235b5cedd85ff3"
        },
        "date": 1782778510098,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 757,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 1011,
            "range": "± 43",
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
            "value": 632,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2342,
            "range": "± 24",
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
          "id": "805252e8fc740f08f0465560778541e0be98d21c",
          "message": "test: add comprehensive integration tests for error console middleware, source context extraction, and access control handlers",
          "timestamp": "2026-06-29T21:23:44-03:00",
          "tree_id": "08fbc5c51ea5a28d009e2c44061210cabfd9fc08",
          "url": "https://github.com/Rullst/Rullst/commit/805252e8fc740f08f0465560778541e0be98d21c"
        },
        "date": 1782779131019,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 621,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 900,
            "range": "± 17",
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
            "value": 614,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 1909,
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
          "id": "805252e8fc740f08f0465560778541e0be98d21c",
          "message": "test: add comprehensive integration tests for error console middleware, source context extraction, and access control handlers",
          "timestamp": "2026-06-29T21:23:44-03:00",
          "tree_id": "08fbc5c51ea5a28d009e2c44061210cabfd9fc08",
          "url": "https://github.com/Rullst/Rullst/commit/805252e8fc740f08f0465560778541e0be98d21c"
        },
        "date": 1782779721545,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 762,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 1013,
            "range": "± 18",
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
            "value": 613,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2333,
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
          "id": "1e70398f6ffaa410f144cd4f0f22e20ed494cfd6",
          "message": "feat: add Rullst edge runtime support and modular feature flagging system",
          "timestamp": "2026-06-29T22:07:39-03:00",
          "tree_id": "b4f343df8c307fcea1ac2a575cb5ce3be941f403",
          "url": "https://github.com/Rullst/Rullst/commit/1e70398f6ffaa410f144cd4f0f22e20ed494cfd6"
        },
        "date": 1782781767946,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 734,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 1017,
            "range": "± 28",
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
            "value": 668,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2347,
            "range": "± 24",
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
          "id": "ae63cd4f516a55756ae15d9ad8293b43980e5d03",
          "message": "feat: implement core Server builder with optional database, scheduler, and hot-reloading capabilities",
          "timestamp": "2026-06-29T22:19:46-03:00",
          "tree_id": "5bda8e7d88d6ddf6d3d5b16b8b3f9168a3de8042",
          "url": "https://github.com/Rullst/Rullst/commit/ae63cd4f516a55756ae15d9ad8293b43980e5d03"
        },
        "date": 1782782493257,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 724,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 1029,
            "range": "± 30",
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
            "value": 655,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2387,
            "range": "± 61",
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
          "id": "ca52a045276d98dfdd011822886815a37463a197",
          "message": "test: add feature, nexus, and studio integration test suites",
          "timestamp": "2026-06-29T22:42:25-03:00",
          "tree_id": "5eab0e4c3f7073b5980e042db4ebcbc70f395d6a",
          "url": "https://github.com/Rullst/Rullst/commit/ca52a045276d98dfdd011822886815a37463a197"
        },
        "date": 1782783856145,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 723,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 979,
            "range": "± 19",
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
            "value": 654,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2313,
            "range": "± 28",
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
          "id": "34e2f18e6ee8a2e8bf6f4a3d079f2b8b07b1a918",
          "message": "test: add HTML macro snapshot tests and update documentation for version 4.0.2",
          "timestamp": "2026-06-29T23:01:27-03:00",
          "tree_id": "2de61e6bf334a1ac441c61679145b7b715868975",
          "url": "https://github.com/Rullst/Rullst/commit/34e2f18e6ee8a2e8bf6f4a3d079f2b8b07b1a918"
        },
        "date": 1782784996070,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 764,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 1008,
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
            "value": 628,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2344,
            "range": "± 27",
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
          "id": "d24b47a04280c0e466a4a08d5db906f7ad25f957",
          "message": "feat: implement core WebAuthn passkey authentication and registration structures",
          "timestamp": "2026-06-30T18:39:04-03:00",
          "tree_id": "8d77ec4e6b85c834fbc30b974d818147bc874a78",
          "url": "https://github.com/Rullst/Rullst/commit/d24b47a04280c0e466a4a08d5db906f7ad25f957"
        },
        "date": 1782855812853,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 627,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 917,
            "range": "± 32",
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
            "value": 646,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 1767,
            "range": "± 25",
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
          "id": "b871845b8b0e8e279c0b4b5f83593940da9c6582",
          "message": "fix: restore html_macro_static performance via explicit inline",
          "timestamp": "2026-06-30T19:00:10-03:00",
          "tree_id": "a8b6c0412f7a839ef6f79d6efdf356699438f562",
          "url": "https://github.com/Rullst/Rullst/commit/b871845b8b0e8e279c0b4b5f83593940da9c6582"
        },
        "date": 1782856920590,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 734,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 981,
            "range": "± 28",
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
            "value": 662,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2250,
            "range": "± 35",
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
          "id": "80a6844ba08c1eac61595e89b923c3d8178b9e1a",
          "message": "fix: ignore queue tests under miri due to sqlite ffi",
          "timestamp": "2026-06-30T19:22:44-03:00",
          "tree_id": "7500e60113e88b1e2eb1ca14a2b93933d23719ac",
          "url": "https://github.com/Rullst/Rullst/commit/80a6844ba08c1eac61595e89b923c3d8178b9e1a"
        },
        "date": 1782858269710,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 734,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 992,
            "range": "± 15",
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
            "value": 657,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2296,
            "range": "± 32",
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
          "id": "44ee7f87f7c82f12e36349ae07123026b2dbc189",
          "message": "fix: ignore server_tests using reqwest/TLS under miri",
          "timestamp": "2026-06-30T19:57:52-03:00",
          "tree_id": "f0a40c3a7a864ea6c47de53caa203720ec5e9b80",
          "url": "https://github.com/Rullst/Rullst/commit/44ee7f87f7c82f12e36349ae07123026b2dbc189"
        },
        "date": 1782860376629,
        "tool": "cargo",
        "benches": [
          {
            "name": "router_match_simple",
            "value": 741,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "router_match_nested_params",
            "value": 1024,
            "range": "± 43",
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
            "value": 652,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "waf_middleware_overhead",
            "value": 2280,
            "range": "± 41",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}