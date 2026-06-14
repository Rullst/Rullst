import re
import os
import sys

frameworks = [
    {"id": "rullst", "name": "Rullst", "lang": "Rust"},
    {"id": "axum", "name": "Axum", "lang": "Rust"},
    {"id": "actix", "name": "Actix-web", "lang": "Rust"},
    {"id": "loco", "name": "Loco", "lang": "Rust"},
    {"id": "rocket", "name": "Rocket", "lang": "Rust"},
    {"id": "leptos", "name": "Leptos", "lang": "Rust"},
    {"id": "gin", "name": "Gin (Go)", "lang": "Go"},
    {"id": "fiber", "name": "Fiber (Go)", "lang": "Go"},
    {"id": "django", "name": "Django", "lang": "Python"},
    {"id": "laravel", "name": "Laravel Octane", "lang": "PHP"},
    {"id": "nextjs", "name": "Next.js", "lang": "JavaScript"},
    {"id": "nestjs", "name": "NestJS", "lang": "JavaScript"},
    {"id": "zap", "name": "Zap (Zig)", "lang": "Zig"},
    {"id": "springboot", "name": "Spring Boot", "lang": "Java"},
    {"id": "rails", "name": "Ruby on Rails", "lang": "Ruby"},
]

def parse_content(content):
    # Extract Reqs/sec
    # Reqs/sec     51200.54
    req_match = re.search(r"Reqs/sec\s+([0-9.]+)", content)
    reqs = float(req_match.group(1)) if req_match else 0.0
    
    # Extract Latency Avg
    # Latency        2.41ms
    lat_match = re.search(r"Latency\s+([0-9.]+[a-zA-Zμ]+)", content)
    latency = lat_match.group(1) if lat_match else "N/A"
    
    return reqs, latency

def parse_file(filepath):
    if not os.path.exists(filepath):
        return 0.0, "N/A"

    # Try reading as UTF-16 first, since PowerShell redirection defaults to UTF-16 LE
    try:
        with open(filepath, "r", encoding="utf-16") as f:
            content = f.read()
            if "Reqs/sec" in content or "Latency" in content:
                return parse_content(content)
    except Exception:
        pass
        
    try:
        with open(filepath, "r", encoding="utf-8", errors="ignore") as f:
            content = f.read()
            return parse_content(content)
    except Exception:
        return 0.0, "N/A"

def main():
    results_dir = "results"
    if len(sys.argv) > 1:
        results_dir = sys.argv[1]

    table = []
    table.append("| Framework | Language | Plaintext (Reqs/s) | Plaintext Latency (Avg) | JSON (Reqs/s) | JSON Latency (Avg) |")
    table.append("|---|---|---|---|---|---|")

    for fw in frameworks:
        p_path = os.path.join(results_dir, f"{fw['id']}_plaintext.txt")
        j_path = os.path.join(results_dir, f"{fw['id']}_json.txt")
        
        p_reqs, p_lat = parse_file(p_path)
        j_reqs, j_lat = parse_file(j_path)
        
        p_reqs_str = f"{p_reqs:,.2f}" if p_reqs > 0 else "N/A"
        j_reqs_str = f"{j_reqs:,.2f}" if j_reqs > 0 else "N/A"
        
        table.append(f"| **{fw['name']}** | {fw['lang']} | {p_reqs_str} | {p_lat} | {j_reqs_str} | {j_lat} |")

    markdown_table = "\n".join(table)
    print("\nGenerated Benchmark Results Table:\n")
    print(markdown_table)
    print("\n")

    output_path = os.path.join(results_dir, "RESULTS.md")
    with open(output_path, "w", encoding="utf-8") as f:
        f.write("# Black-Box HTTP Load Test Results\n\n")
        f.write("Generated using `bombardier` version 2.0.2 stress-testing each containerized service in isolation.\n\n")
        f.write(markdown_table)
        f.write("\n")
    print(f"Results written to {output_path}")

if __name__ == "__main__":
    main()
