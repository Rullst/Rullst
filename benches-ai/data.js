window.BENCHMARK_DATA = {
  "lastUpdate": 1786076830408,
  "repoUrl": "https://github.com/Rullst/Rullst",
  "entries": {
    "Rullst AI Benchmark": [
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
        "date": 1786076830027,
        "tool": "cargo",
        "benches": [
          {
            "name": "ai_tool_registry/export_openai_schema",
            "value": 1614,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "ai_message_context/message_json_serialization",
            "value": 399,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "ai_message_context/estimate_context_tokens",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "ai_pii_masking/mask_pii",
            "value": 755,
            "range": "± 3",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}