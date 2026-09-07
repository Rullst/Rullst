# Your first idea, running in Rust

You do not need to understand every Rullst crate before writing your first
application. Start small, make one thing work, and learn what each layer does
as your product needs it.

> **You are exploring the v12 development preview.** It is not yet a supported
> production release. Use the instructions for this source revision—not the
> older CLI installed by an unversioned crates.io command. The
> [release program](v12.md) and [current audit](v12-release-audit.md) show the
> remaining gates.

## Pick your starting line

| What you want to do | Start here | Your first visible result |
| --- | --- | --- |
| “Give me an application I can explore.” | [CLI installation and blueprints](1-getting-started.md) | Generated Rust, a local web page and a development loop |
| “Show me how the framework actually works.” | [Zero to Hello Rullst](tutorials/01-hello-world.md) | One complete typed route you write yourself |
| “I need a backend for another client.” | [Your first JSON REST API](tutorials/rest-api-quickstart.md) | A real HTTP response you can inspect with curl |

For the simplest first experiment, choose **Blank + SQLite**, or Blank with
`--no-database` if you do not need persistence yet. The larger LMS and SaaS
blueprints have more moving parts and longer first builds. You can explore
them after your toolchain and local workflow are working.

## A small loop that teaches a lot

1. **Run it.** Reach the local page or JSON endpoint before adding features.
2. **Find the code.** Open `src/main.rs`, then the generated controllers, models
   and pages that exist in your chosen blueprint.
3. **Make one visible change.** Edit a heading or response field. With
   `cargo rullst dev`, watch the rebuild and process restart; with plain
   `cargo run`, stop and restart it yourself.
4. **Break something safely.** Introduce a small syntax error in your local
   experiment. Read the compiler diagnostic, correct it and save again. The
   supervisor keeps the old application running when compilation fails.
5. **Verify the result.** Reload the page or repeat the request. A successful
   build is only the first check; the response should do what you intended.

See [exact restart and state boundaries](tutorials/51-authenticated-hot-reload.md).
Do not use production databases or credentials for this exercise.

## Build your understanding, one layer at a time

| Next question | Guide |
| --- | --- |
| Where do generated files go? | [CLI generators](tutorials/02-cli-generators.md) |
| How do I save and read data? | [Active Record CRUD](tutorials/03-active-record-crud.md) and [migrations](tutorials/05-migrations-and-seeds.md) |
| How does the page become interactive? | [HTML and HTMX](tutorials/06-htmx-zero-bundle.md) |
| How do requests reach my handlers? | [Routes and middleware](tutorials/08-routing-and-middlewares.md) |
| Who is allowed to access a record? | [Ownership, RBAC and IDOR](tutorials/13-rbac-authorization.md) |
| How do I update the framework later? | [Assisted upgrades](tutorials/36-assisted-framework-upgrades.md) |

You can use an AI assistant while learning. Ask it to explain the generated
files, point to the exact APIs and show a failing test before a bug fix. Treat
its output as a proposed change, not as proof. Rullst's
[architecture specification](spec.md) is the common reference for both of you.

## When the first run does not work

| What you see | Check first |
| --- | --- |
| `cargo rullst` is unknown | Install the matching CLI, then reopen your terminal if its binary directory is not on `PATH`. |
| The first build is taking a long time | Source builds compile Rust dependencies. Keep the output visible; elapsed time is not a reliable failure signal. |
| A database connection fails | Confirm the selected backend and local `.env` values. PostgreSQL/MySQL/MariaDB need a running service; SQLite does not need a separate server. |
| The app cannot bind its port | Stop the conflicting process you own, or configure another port and restart the development command. |
| Optional persistence feels confusing | Selecting none is valid. Add only the specialized stores your application actually needs. |
| A command differs from a screenshot | Use the installed CLI's `--help` and the matching source documentation. Screenshots are recorded examples. |

If you are still stuck, share the command, operating system, Rust version,
framework commit and a minimal reproduction in an
[issue](https://github.com/Rullst/Rullst/issues) or on
[Discord](https://discord.com/invite/2ntKFtsSjw). Remove secrets and personal
data first. Send security vulnerabilities privately using the
[security policy](https://github.com/Rullst/Rullst/security/policy).

## Before this becomes a real product

A generated app is a foundation, not a deployment approval. Review
authentication, ownership, secrets, database backups and migrations, external
provider setup and release status. Start with the
[security architecture](security-architecture.md), choose features using the
[capability status](capability-status.md), and measure your own workload.

**Ready? [Create your first application →](1-getting-started.md)**
