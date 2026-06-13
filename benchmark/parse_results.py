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
    {"id": "zap", "name": "Zap (Zig)", "lang": "Zig"},
    {"id": "gin", "name": "Gin (Go)", "lang": "Go"},
    {"id": "fiber", "name": "Fiber (Go)", "lang": "Go"},
    {"id": "springboot", "name": "Spring Boot", "lang": "Java"},
    {"id": "nestjs", "name": "NestJS", "lang": "JavaScript"},
    {"id": "django", "name": "Django", "lang": "Python"},
    {"id": "rails", "name": "Ruby on Rails", "lang": "Ruby"},
    {"id": "nextjs", "name": "Next.js", "lang": "JavaScript"},
    {"id": "laravel", "name": "Laravel Octane", "lang": "PHP"},
]

def parse_content(content):
    req_match = re.search(r"Reqs/sec\s+([0-9.]+)", content)
    reqs = float(req_match.group(1)) if req_match else 0.0
    
    lat_match = re.search(r"Latency\s+([0-9.]+[a-zA-Zμ]+)", content)
    latency = lat_match.group(1) if lat_match else "N/A"
    
    return reqs, latency

def parse_file(filepath):
    if not os.path.exists(filepath):
        return 0.0, "N/A"

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

    # Process and sort frameworks
    for fw in frameworks:
        p_path = os.path.join(results_dir, f"{fw['id']}_plaintext.txt")
        j_path = os.path.join(results_dir, f"{fw['id']}_json.txt")
        
        p_reqs, p_lat = parse_file(p_path)
        j_reqs, j_lat = parse_file(j_path)
        
        fw['p_reqs'] = p_reqs
        fw['p_lat'] = p_lat
        fw['j_reqs'] = j_reqs
        fw['j_lat'] = j_lat

    # Sort by plaintext requests per second descending
    frameworks.sort(key=lambda x: x['p_reqs'], reverse=True)

    table = []
    table.append("| Framework | Language | Plaintext (Reqs/s) | Plaintext Latency (Avg) | JSON (Reqs/s) | JSON Latency (Avg) |")
    table.append("|---|---|---|---|---|---|")

    for fw in frameworks:
        p_reqs_str = f"{fw['p_reqs']:,.2f}" if fw['p_reqs'] > 0 else "N/A"
        j_reqs_str = f"{fw['j_reqs']:,.2f}" if fw['j_reqs'] > 0 else "N/A"
        
        table.append(f"| **{fw['name']}** | {fw['lang']} | {p_reqs_str} | {fw['p_lat']} | {j_reqs_str} | {fw['j_lat']} |")

    markdown_table = "\n".join(table)
    print("\nGenerated Benchmark Results Table:\n")
    print(markdown_table)
    print("\n")

    output_path = os.path.join(results_dir, "RESULTS.md")
    with open(output_path, "w", encoding="utf-8") as f:
        f.write("# Black-Box HTTP Load Test Results\n\n")
        f.write("Generated using `bombardier` stress-testing each containerized service in isolation.\n\n")
        f.write(markdown_table)
        f.write("\n")
    print(f"Results written to {output_path}")

if __name__ == "__main__":
    main()
