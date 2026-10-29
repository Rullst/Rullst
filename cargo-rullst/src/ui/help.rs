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
                    "Add a rullst-* dependency to Cargo.toml",
                ),
                (
                    "cargo rullst upgrade",
                    "Plan/apply a transactional framework upgrade",
                ),
                (
                    "cargo rullst eject",
                    "Generate a reviewable Axum/Tokio entry-point snapshot",
                ),
                (
                    "cargo rullst doctor [--fix] [--json]",
                    "Grouped toolchain, project, database and security checks",
                ),
                (
                    "cargo rullst info [--json]",
                    "CLI version, toolchain and detected project",
                ),
                (
                    "cargo rullst completions <shell>",
                    "Shell completions (bash, zsh, fish, powershell, elvish)",
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
                    "cargo rullst make:k8s",
                    "Generate Kubernetes manifests & health probes",
                ),
                (
                    "cargo rullst make:scalar",
                    "Scaffold interactive Scalar OpenAPI docs",
                ),
                (
                    "cargo rullst make:iot <Name>",
                    "Scaffold a local IoT telemetry module",
                ),
                (
                    "cargo rullst make:mail <Name>",
                    "Scaffold Mailable struct & email template",
                ),
                (
                    "cargo rullst make:mail-invoice [Name]",
                    "Scaffold bounded payment-receipt mail",
                ),
                (
                    "cargo rullst make:mail-dunning [Name]",
                    "Scaffold explicit D+1/D+3/D+7 recovery mail",
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
            "🔐  AUTH & SECURITY",
            vec![
                ("cargo rullst auth", "Scaffold full auth system"),
                (
                    "cargo rullst make:mfa",
                    "Scaffold 2FA TOTP authentication system",
                ),
                (
                    "cargo rullst audit [--ai] [--compliance] [--idor] [--geiger] [--json] [--report md|html|json]",
                    "Run bounded vulnerability, IDOR, unsafe and evidence checks",
                ),
                ("cargo rullst make:billing", "Scaffold Stripe billing"),
                ("cargo rullst make:cors", "Add CORS middleware"),
                ("cargo rullst make:jwt", "Add JWT middleware"),
            ],
        ),
        (
            "📱  INSTALLABLE APP (PWA)",
            vec![
                (
                    "cargo rullst make:pwa",
                    "Manifest, icons, service worker and offline page",
                ),
                (
                    "cargo rullst make:pwa --android --host <host> --package-id <id>",
                    "Also a Google Play (Bubblewrap) configuration",
                ),
            ],
        ),
        (
            "🚀  DEPLOY",
            vec![
                (
                    "cargo rullst deploy",
                    "Dockerfile + compose.yaml + Caddyfile for a Linux VM",
                ),
                ("cargo rullst dockerize", "Generate the Dockerfile only"),
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
                    "cargo rullst inspect [target] [--json]",
                    "Inspect expanded macro code & route schemas",
                ),
                (
                    "cargo rullst dash",
                    "Start interactive Ratatui Dev Dashboard",
                ),
            ],
        ),
        (
            "🧩  EXTRAS (cargo install cargo-rullst-extras)",
            vec![
                (
                    "cargo rullst ai [goal]",
                    "Terminal assistant that knows Rullst and your project",
                ),
                (
                    "cargo rullst green [--bench]",
                    "Measured size, memory and CPU; optional SCI estimate",
                ),
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
