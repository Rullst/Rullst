// src/ui/help.rs — CLI help groups and full command reference.

use colored::*;

pub fn get_help_groups() -> Vec<(&'static str, Vec<(&'static str, &'static str)>)> {
    vec![
        (
            "🗂️  PROJECT & COMMUNITY",
            vec![
                ("cargo rullst new [name]", "Create a new Rullst application"),
                (
                    "cargo rullst dev [--ts-sync]",
                    "Start dev server (optional auto TS SDK sync)",
                ),
                (
                    "cargo rullst pkg add <name>",
                    "Install RullstPackage community extension",
                ),
                ("cargo rullst upgrade", "Upgrade Rullst with safe codemods"),
                (
                    "cargo rullst eject",
                    "Zero lock-in framework escape hatch to pure Axum",
                ),
            ],
        ),
        (
            "🛠️  SCAFFOLDING",
            vec![
                (
                    "cargo rullst make:resource <Name>",
                    "Full CRUD stack (Model, Migration, Controller, Views)",
                ),
                ("cargo rullst make:controller <Name>", "New controller"),
                (
                    "cargo rullst make:model <Name> -m",
                    "New model (+migration)",
                ),
                ("cargo rullst make:middleware <Name>", "New middleware"),
                ("cargo rullst make:worker <Name>", "New background worker"),
                ("cargo rullst make:migration <name>", "Blank migration"),
                (
                    "cargo rullst make:island <name>",
                    "New interactive Wasm Island",
                ),
                (
                    "cargo rullst make:live <Name>",
                    "Scaffold LiveView Server-Driven UI component",
                ),
                (
                    "cargo rullst make:grpc <Name>",
                    "Scaffold gRPC microservice & Protobuf schema",
                ),
                (
                    "cargo rullst make:k8s",
                    "Generate Kubernetes manifests & health probes",
                ),
                (
                    "cargo rullst make:scalar",
                    "Scaffold interactive Scalar OpenAPI docs",
                ),
                (
                    "cargo rullst make:iot <Name>",
                    "Scaffold bare-metal IoT edge sensor node",
                ),
                (
                    "cargo rullst generate:models",
                    "Reverse-engineer live DB to Models",
                ),
            ],
        ),
        (
            "🤖  AI & AGENTS",
            vec![
                (
                    "cargo rullst make:chat-session",
                    "Scaffold stateful AI chat & memory",
                ),
                (
                    "cargo rullst generate:ai-context",
                    "Generate .llms.txt context file",
                ),
            ],
        ),
        (
            "🗄️  DATABASE",
            vec![
                ("cargo rullst db:migrate", "Run pending migrations"),
                ("cargo rullst db:rollback", "Rollback last batch"),
                ("cargo rullst db:status", "Show migration status"),
                ("cargo rullst db:seed", "Run seeders"),
                (
                    "cargo rullst make:migration:auto",
                    "Auto-diff ORM models & DB schema",
                ),
                ("cargo rullst studio", "Open DB studio browser"),
            ],
        ),
        (
            "🔐  AUTH & BILLING",
            vec![
                ("cargo rullst auth", "Scaffold full auth system"),
                ("cargo rullst make:billing", "Scaffold Stripe billing"),
                ("cargo rullst make:cors", "Add CORS middleware"),
                ("cargo rullst make:jwt", "Add JWT middleware"),
            ],
        ),
        (
            "🖥️  DESKTOP & MOBILE (OMNI)",
            vec![
                (
                    "cargo rullst make:omni",
                    "Scaffold Omni desktop & mobile app wrapper",
                ),
                (
                    "cargo rullst omni [target]",
                    "Run Omni app (desktop, android, ios)",
                ),
            ],
        ),
        (
            "🚀  DEPLOY",
            vec![
                (
                    "cargo rullst deploy [--platform=...]",
                    "1-Click PaaS deploy (Fly, Railway, Render, VPS Caddy)",
                ),
                ("cargo rullst dockerize", "Generate Docker files"),
                (
                    "cargo rullst generate:buildah",
                    "Generate rootless OCI build script",
                ),
                ("cargo rullst nixify", "Generate Nix environment files"),
                ("cargo rullst foundry:init", "Create Foundry.toml manifest"),
                ("cargo rullst foundry:deploy", "Deploy via SSH pipeline"),
            ],
        ),
        (
            "📦  BUILD & DOCS",
            vec![
                (
                    "cargo rullst build",
                    "Production binary + Brotli/Zstd assets",
                ),
                ("cargo rullst build:client", "Compile Wasm Islands"),
                ("cargo rullst generate:openapi", "Generate OpenAPI spec"),
                (
                    "cargo rullst generate:diagram",
                    "Generate Mermaid ER diagram",
                ),
                (
                    "cargo rullst generate:ai-context",
                    "Generate AI context (.llms.txt)",
                ),
                (
                    "cargo rullst inspect [target]",
                    "Inspect expanded macro code & route schemas",
                ),
                (
                    "cargo rullst dash",
                    "Start interactive Ratatui Dev Dashboard",
                ),
                ("cargo rullst docs dev", "Live docs preview server"),
                ("cargo rullst docs build", "Build static docs site"),
            ],
        ),
    ]
}

pub fn show_help_reference() {
    print!("\x1B[2J\x1B[1;1H");
    println!(
        "\n  {}",
        "╔══════════════════════════════════════════╗"
            .bright_cyan()
            .bold()
    );
    println!(
        "  {}  💡 Rullst CLI - Full Command Reference  {}",
        "║".bright_cyan().bold(),
        "║".bright_cyan().bold()
    );
    println!(
        "  {}",
        "╚══════════════════════════════════════════╝"
            .bright_cyan()
            .bold()
    );

    for (group_name, cmds) in get_help_groups() {
        println!("  {}", group_name.bright_yellow().bold());
        for (cmd, desc) in cmds {
            println!("    {:<35} {}", cmd.bright_cyan(), desc.white());
        }
        println!();
    }
}
