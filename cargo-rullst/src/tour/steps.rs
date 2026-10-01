//! The tour's content as data, so `--list`, the guided screens and the tests
//! share one source.

/// A read-only command a step offers to run, only after the user picks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Example {
    pub(crate) arguments: &'static [&'static str],
    /// What running it does; shown next to the offer.
    pub(crate) effect: &'static str,
    /// Run instead when this CLI build lacks the first subcommand.
    pub(crate) fallback: Option<&'static [&'static str]>,
}

/// One stop of the tour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Step {
    /// The command family, e.g. `new` or `make:*`.
    pub(crate) command: &'static str,
    pub(crate) title: &'static str,
    pub(crate) explanation: &'static [&'static str],
    pub(crate) try_commands: &'static [&'static str],
    pub(crate) example: Example,
}

pub(crate) const STEPS: [Step; 7] = [
    Step {
        command: "new",
        title: "Create a project",
        explanation: &[
            "Pick a blueprint, a database and optional features, then review the",
            "file tree and the commands it will run before anything is written.",
            "Every answer has a flag; with --default it never prompts (CI, scripts).",
        ],
        try_commands: &[
            "cargo rullst new my_app",
            "cargo rullst new my_app --default --blueprint blog --database sqlite",
        ],
        example: Example {
            arguments: &["new", "tour-demo", "--default", "--dry-run"],
            effect: "previews a project; creates nothing",
            fallback: None,
        },
    },
    Step {
        command: "dev",
        title: "Run it with live reload",
        explanation: &[
            "Builds the app, applies pending migrations and serves it, by default on",
            "http://127.0.0.1:3000. Saving a file rebuilds and restarts it; a failed",
            "build keeps the last good version serving.",
        ],
        try_commands: &["cargo rullst dev"],
        example: Example {
            arguments: &["dev", "--help"],
            effect: "read-only: lists its options",
            fallback: None,
        },
    },
    Step {
        command: "dash",
        title: "Watch it in a dashboard",
        explanation: &[
            "The same development loop in a terminal dashboard, with the build",
            "output, application logs and keyboard controls on one screen.",
        ],
        try_commands: &["cargo rullst dash"],
        example: Example {
            arguments: &["dash", "--help"],
            effect: "read-only: lists its options",
            fallback: None,
        },
    },
    Step {
        command: "make:*",
        title: "Generate code",
        explanation: &[
            "Scaffold models with migrations, controllers, full resources,",
            "middleware, workers and more. The output is plain Rust you own.",
        ],
        try_commands: &[
            "cargo rullst make:model Post -m",
            "cargo rullst make:controller Posts",
            "cargo rullst make:resource Product",
        ],
        example: Example {
            arguments: &["make:model", "--help"],
            effect: "read-only: lists its options",
            fallback: None,
        },
    },
    Step {
        command: "db:*",
        title: "Evolve the database",
        explanation: &[
            "Apply, inspect and roll back migrations and run seeders against the",
            "database configured in .env or Rullst.toml.",
        ],
        try_commands: &[
            "cargo rullst db:migrate",
            "cargo rullst db:status",
            "cargo rullst db:rollback",
        ],
        example: Example {
            arguments: &["db:status", "--help"],
            effect: "read-only: lists its options",
            fallback: None,
        },
    },
    Step {
        command: "doctor",
        title: "Check your environment",
        explanation: &[
            "Checks the Rust toolchain and the optional tools Rullst uses.",
            "It changes nothing unless you pass --fix.",
        ],
        try_commands: &["cargo rullst doctor"],
        example: Example {
            arguments: &["doctor", "--help"],
            effect: "read-only: lists its options",
            fallback: None,
        },
    },
    Step {
        command: "ai",
        title: "Work with AI",
        explanation: &[
            "`cargo rullst ai` chats about your project in the terminal once a",
            "provider is connected; `generate:ai-context` writes .llms.txt so any",
            "assistant can see the project's layout.",
        ],
        try_commands: &["cargo rullst ai", "cargo rullst generate:ai-context"],
        example: Example {
            arguments: &["ai", "--help"],
            effect: "read-only: lists its options",
            fallback: Some(&["generate:ai-context", "--help"]),
        },
    },
];

/// `arguments` as the user would type them.
pub(crate) fn shown(arguments: &[&str]) -> String {
    let mut words = vec!["cargo", "rullst"];
    words.extend_from_slice(arguments);
    words.join(" ")
}
