"""What the first-hour pages say happens, checked around their marked blocks.

Hooks are keyed by the page and step of a marked block (first_hour_docs.py):

- RUN runs the block itself, for a page that documents a failure and a retry;
- APPLY applies an `apply` block as the page's prose says;
- AFTER runs after the block, for commands and results the prose states;
- SERVER runs once the server the block started answers on port 3000.

Each hook quotes or names the sentence of the page it follows.
"""

from __future__ import annotations

import http.client
import json
from pathlib import Path
import re
import secrets
import sqlite3
import urllib.parse

README = "README.md"
GETTING_STARTED = "docs/src/1-getting-started.md"
HELLO = "docs/src/tutorials/01-hello-world.md"
REST = "docs/src/tutorials/rest-api-quickstart.md"
CLI_GENERATORS = "docs/src/tutorials/02-cli-generators.md"
ACTIVE_RECORD = "docs/src/tutorials/03-active-record-crud.md"
MIGRATIONS = "docs/src/tutorials/05-migrations-and-seeds.md"
AUTHENTICATION = "docs/src/tutorials/11-authentication-system.md"
PORT = 3000
ORIGIN = f"http://127.0.0.1:{PORT}"


class Failure(Exception):
    """A documented step did not do what its page says; the message says which."""


def log(message: str) -> None:
    print(message, flush=True)


def expect(condition: bool, message: str) -> None:
    if not condition:
        raise Failure(message)
    log(f"ok: {message}")


class Browser:
    """Plain HTTP to the local server, keeping cookies as a browser on localhost does."""

    def __init__(self) -> None:
        self.cookies: dict[str, str] = {}

    def request(self, method: str, path: str, *, form: dict[str, str] | None = None,
                body: bytes | None = None, headers: dict[str, str] | None = None
                ) -> tuple[int, dict[str, str], str]:
        headers = dict(headers or {})
        if form is not None:
            token = self.cookies.get("rullst_csrf", "")
            headers.update({"Content-Type": "application/x-www-form-urlencoded",
                            "Origin": ORIGIN, "X-CSRF-Token": token})
            body = urllib.parse.urlencode({**form, "_token": token}).encode()
        if self.cookies:
            headers["Cookie"] = "; ".join(f"{k}={v}" for k, v in self.cookies.items())
        connection = http.client.HTTPConnection("127.0.0.1", PORT, timeout=30)
        try:
            connection.request(method, path, body=body, headers=headers)
            response = connection.getresponse()
            text = response.read(2 * 1024 * 1024).decode(errors="replace")
        except OSError as error:
            raise Failure(f"{method} {path} failed: {error}") from error
        finally:
            connection.close()
        for cookie in response.headers.get_all("Set-Cookie") or []:
            name, _, rest = cookie.partition("=")
            value = rest.split(";", 1)[0]
            if value and "max-age=0" not in cookie.lower():
                self.cookies[name.strip()] = value
            else:
                self.cookies.pop(name.strip(), None)
        return response.status, {k.lower(): v for k, v in response.getheaders()}, text


def get(path: str, browser: Browser | None = None) -> tuple[int, dict[str, str], str]:
    return (browser or Browser()).request("GET", path)


def check_health() -> None:
    code, _, body = get("/health")
    expect(code == 200, f"GET /health answers {code}")
    json.loads(body)


def generated_home(session, block) -> None:
    """The starter's welcome page and probes ("open the local URL")."""
    check_health()
    code, headers, body = get("/")
    expect(code == 200 and "Welcome to" in body, f"GET / shows the welcome page ({code})")
    expect("Database connected." in body, "the welcome page reaches the SQLite database")
    expect(headers.get("x-content-type-options") == "nosniff", "the security headers are set")


def next_steps(session, block) -> None:
    """"prints three numbered next steps: `cd my_app`, `cargo rullst dev` and the local URL"."""
    for text in ("cd my_app", "cargo rullst dev", "http://127.0.0.1:3000"):
        expect(text in session.output, f"`cargo rullst new` prints the next step `{text}`")


def toolchain(session, block) -> None:
    """"Rullst v13 requires Rust 1.96.0 or newer"."""
    found = re.search(r"rustc (\d+)\.(\d+)\.(\d+)", session.output)
    expect(bool(found) and tuple(map(int, found.groups())) >= (1, 96, 0),
           f"rustc --version reports 1.96.0 or newer ({found and found[0]})")


def cli_version(session, block) -> None:
    """"The second command prints `rullst 13.0.0-alpha.N`"."""
    expect(f"rullst {session.version}" in session.output,
           f"`cargo rullst --version` prints rullst {session.version}")


def hello_page(session, block) -> None:
    """"When the terminal prints `Rullst framework serving on http://127.0.0.1:3000`"."""
    code, _, body = get("/")
    expect(code == 200 and "Hello, Rullst!" in body, f"GET / shows the first route ({code})")
    expect("Rullst framework serving on http://127.0.0.1:3000" in session.server_log(),
           "the terminal prints `Rullst framework serving on http://127.0.0.1:3000`")


def json_health(session, block) -> None:
    """"The secure-header layer is included explicitly"."""
    code, headers, body = get("/api/health")
    expect(code == 200 and json.loads(body) == {"status": "ok", "framework": "Rullst"},
           f"GET /api/health answers the typed JSON ({code})")
    expect(headers.get("x-content-type-options") == "nosniff", "the secure-header layer is applied")


def api_starter(session, block) -> None:
    """`GET /` welcome JSON, bearer-protected `POST /api/messages`, `/health` and `/ready`."""
    code, _, body = get("/")
    expect(code == 200 and "Welcome to Rullst REST API" in json.loads(body)["message"],
           f"GET / answers the JSON welcome message ({code})")
    for probe in ("/health", "/ready"):
        code, _, _ = get(probe)
        expect(code == 200, f"GET {probe} answers {code}")
    token = dotenv(session.cwd / ".env").get("API_TOKEN", "")
    expect(len(token) >= 32, "`.env` holds a random API_TOKEN")
    message = json.dumps({"text": "first hour"}).encode()
    json_header = {"Content-Type": "application/json"}
    code, _, _ = Browser().request("POST", "/api/messages", body=message, headers=json_header)
    expect(code == 401, f"POST /api/messages without the bearer token is refused ({code})")
    code, _, body = Browser().request("POST", "/api/messages", body=message,
                                      headers={**json_header, "Authorization": f"Bearer {token}"})
    expect(code == 201 and json.loads(body) == {"text": "first hour"},
           f"POST /api/messages with the API_TOKEN bearer token answers 201 ({code})")


def dotenv(path: Path) -> dict[str, str]:
    values = {}
    for line in path.read_text().splitlines():
        key, separator, value = line.partition("=")
        if separator and not key.lstrip().startswith("#"):
            values[key.strip()] = value.strip().strip('"')
    return values


def files_exist(*paths: str):
    def check(session, block) -> None:
        for pattern in paths:
            expect(len(list(session.cwd.glob(pattern))) == 1, f"{block.step} wrote {pattern}")
    return check


def contains(path: str, *texts: str):
    def check(session, block) -> None:
        source = (session.cwd / path).read_text()
        for text in texts:
            expect(text in source, f"{path} contains `{text}`")
    return check


def both(*checks):
    def check(session, block) -> None:
        for each in checks:
            each(session, block)
    return check


def inspected(session, block) -> None:
    for text in ("/products", "Product", "Category"):
        expect(text in session.output, f"`cargo rullst inspect` lists {text}")


def database(session) -> sqlite3.Connection:
    return sqlite3.connect(f"file:{session.cwd / 'db.sqlite'}?mode=ro", uri=True)


def tables(session) -> set[str]:
    with database(session) as db:
        return {row[0] for row in db.execute("SELECT name FROM sqlite_master WHERE type = 'table'")}


def products_migrated(session, block) -> None:
    """"Apply the migration with `cargo rullst db:migrate` ... before using the model"."""
    session.run(["cargo", "rullst", "db:migrate"])
    with database(session) as db:
        columns = {row[1] for row in db.execute("PRAGMA table_info(products)")}
    expect({"id", "name", "price_cents", "created_at", "updated_at"} <= columns,
           f"the products table has the tutorial's columns ({sorted(columns)})")


def rolled_back(session, block) -> None:
    """`db:status` lists the migration; `db:rollback` reverts only the last batch."""
    expect("create_catalog_items_table" in session.output, "db:status lists the new migration")
    present = tables(session)
    expect("catalog_items" not in present and {"products", "categories"} <= present,
           "db:rollback dropped catalog_items and kept the earlier batch")


def declare_seeds(session, block) -> None:
    """"declare `mod seeds;` in `src/main.rs`"."""
    main = session.cwd / "src/main.rs"
    main.write_text("mod seeds;\n" + main.read_text())
    log("declared `mod seeds;` in src/main.rs")


def seeded(session, block) -> None:
    """The seeder adds one product; "a second `db:seed` finds the row and adds nothing"."""
    def count() -> int:
        with database(session) as db:
            return db.execute("SELECT COUNT(*) FROM products WHERE name = 'Starter kit'").fetchone()[0]
    expect(count() == 1, "db:seed inserted the Starter kit product")
    session.run(["cargo", "rullst", "db:seed"])
    expect(count() == 1, "a second db:seed adds nothing")


STARTER_USERS = "src/migrations/m20260601000000_create_users_table.rs"
WORKAROUND = "delete that migration file"


def auth_generator(session, block) -> None:
    """`cargo rullst auth` in the Tutorial 02 project, including the retry the
    page documents for starters that already have a `users` migration."""
    page = session.page_text(block.page)
    if WORKAROUND not in page:
        session.run_commands(block)
        return
    first = session.run_commands(block, check=False)
    expect(first.returncode != 0 and "make:migration" in first.output
           and not (session.cwd / "src/models/user.rs").exists(),
           "the first `cargo rullst auth` refuses the starter's users migration and writes nothing")
    # "delete that migration file; if `cargo rullst dev` or `cargo rullst
    # db:migrate` already ran, also delete the local SQLite database (`db.sqlite`,
    # with any `db.sqlite-wal`/`db.sqlite-shm` beside it)"
    for name in (STARTER_USERS, "db.sqlite", "db.sqlite-wal", "db.sqlite-shm"):
        (session.cwd / name).unlink(missing_ok=True)
    log(f"followed the page: deleted {STARTER_USERS} and db.sqlite")
    session.run_commands(block)


def wire_routes(session, block) -> None:
    """"Put its `use` lines at the top of `src/main.rs` ... and the `let`
    statements in `router()`", then "Add each group to the starter's router in
    `router()` (`src/main.rs`) with `.merge_axum(public_auth.into_axum())` and
    so on, before its `.layer(rullst::server::from_fn(rullst::security::csrf_middleware))` line"."""
    main = session.cwd / "src/main.rs"
    lines = main.read_text().splitlines()
    code = block.code.splitlines()
    imports = [line for line in code if line.startswith("use ")]
    statements = [line for line in code if not line.startswith("use ")]
    while statements and not statements[0].strip():
        statements.pop(0)
    first = next(i for i, line in enumerate(lines) if line.startswith("use "))
    last = next(i for i in range(first, len(lines)) if not lines[i].startswith("use "))
    lines[last:last] = imports
    start = next(i for i, line in enumerate(lines) if line.startswith("fn router() -> Router {"))
    body = next(i for i in range(start, len(lines)) if lines[i].strip() == "routes![")
    lines[body:body] = ["    " + line if line.strip() else "" for line in statements]
    groups = re.findall(r"^let (\w+) =", block.code, re.M)
    csrf = next(i for i in range(start, len(lines)) if "security::csrf_middleware" in lines[i])
    lines[csrf:csrf] = [f"    .merge_axum({group}.into_axum())" for group in groups]
    main.write_text("\n".join(lines) + "\n")
    log(f"imported {len(imports)} paths and mounted {', '.join(groups)} in router() before the CSRF layer")


def shop_with_accounts(session, block) -> None:
    """The routes of tutorials 02 and 11, and an account's first sign-up and sign-in."""
    check_health()
    code, _, _ = get("/products")
    expect(code == 200, f"GET /products shows the generated placeholder page ({code})")
    browser, password = Browser(), secrets.token_urlsafe(18)
    email, name = "first-hour@example.com", "First Hour"
    code, _, _ = browser.request("GET", "/register")
    expect(code == 200 and "rullst_csrf" in browser.cookies, f"GET /register sets the CSRF cookie ({code})")
    code, headers, _ = browser.request("POST", "/register",
                                       form={"name": name, "email": email, "password": password})
    expect(code in (302, 303) and headers.get("location") == "/dashboard",
           f"POST /register signs the account in ({code} -> {headers.get('location')})")
    code, _, body = browser.request("GET", "/dashboard")
    expect(code == 200 and name in body, f"the dashboard shows the account ({code})")
    code, headers, _ = browser.request("POST", "/logout", form={})
    expect(code in (302, 303) and headers.get("location") == "/login", f"POST /logout signs out ({code})")
    code, headers, _ = browser.request("GET", "/dashboard")
    expect(code in (302, 303, 307, 401), f"the dashboard needs a session after logout ({code})")
    browser.request("GET", "/login")
    code, headers, _ = browser.request("POST", "/login", form={"email": email, "password": password})
    expect(code in (302, 303) and headers.get("location") == "/dashboard",
           f"POST /login signs the account in again ({code} -> {headers.get('location')})")
    code, _, _ = browser.request("GET", "/password/change")
    expect(code == 200, f"GET /password/change answers a signed-in account ({code})")


RUN = {
    (AUTHENTICATION, "auth"): auth_generator,
}

APPLY = {
    (AUTHENTICATION, "auth-routes"): wire_routes,
    (AUTHENTICATION, "password-routes"): wire_routes,
}

AFTER = {
    (GETTING_STARTED, "create"): next_steps,
    (HELLO, "toolchain"): toolchain,
    (HELLO, "cli"): cli_version,
    (CLI_GENERATORS, "create"): files_exist(
        "Cargo.toml", "Rullst.toml", ".env", "src/main.rs", "src/migrations/m*_create_users_table.rs"),
    (CLI_GENERATORS, "controller"): both(
        files_exist("src/controllers/products_controller.rs"),
        contains("src/main.rs", "pub mod controllers;")),
    (CLI_GENERATORS, "model"): both(
        files_exist("src/models/product.rs", "src/migrations/m*_create_products.rs"),
        contains("src/migrations/mod.rs", "_create_products")),
    (CLI_GENERATORS, "resource"): files_exist(
        "src/models/category.rs", "src/migrations/m*_create_categories*.rs",
        "src/controllers/category_controller.rs", "views/category/index.html",
        "views/category/form.html"),
    (CLI_GENERATORS, "inspect"): inspected,
    (ACTIVE_RECORD, "model"): products_migrated,
    (MIGRATIONS, "make-migration"): both(
        files_exist("src/migrations/m*_create_catalog_items_table.rs"),
        contains("src/migrations/mod.rs", "_create_catalog_items_table")),
    (MIGRATIONS, "migrate"): rolled_back,
    (MIGRATIONS, "seeder"): declare_seeds,
    (MIGRATIONS, "seed"): seeded,
}

SERVER = {
    (README, "quickstart"): generated_home,
    (GETTING_STARTED, "run"): generated_home,
    (GETTING_STARTED, "first-app"): generated_home,
    (HELLO, "run"): hello_page,
    (REST, "run"): json_health,
    (REST, "generated"): api_starter,
    (AUTHENTICATION, "run"): shop_with_accounts,
}

HOOKS = {"run": RUN, "apply": APPLY, "after": AFTER, "server": SERVER}
