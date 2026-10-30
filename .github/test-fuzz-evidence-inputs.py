#!/usr/bin/env python3
"""Exercise input identity against real isolated Git histories, not path mocks."""

import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from fuzz_evidence_inputs import ROOT, Snapshot, execution_contract, reviewed_document_blobs

# Fixture graph, mirroring the real shape: most parents depend on a shared
# core crate, the facade also reaches mail, the ORM reaches its macro crate.
CORE_USERS = ("rullst", "rullst-ai", "rullst-capital", "rullst-mail", "rullst-nexus",
              "rullst-security", "rullst-studio")
PARENT_DEPENDENCIES = {
    **{parent: ['rullst-core = { workspace = true }'] for parent in CORE_USERS},
    "rullst": ['rullst-core = { workspace = true }', 'rullst-mail = { workspace = true }'],
    "rullst-orm": ['rullst-orm-macros = { path = "../rullst-orm-macros" }'],
}
REACHES = {
    "rullst-core": {f"{parent}/fuzz" for parent in CORE_USERS},
    "rullst-mail": {"rullst-mail/fuzz", "rullst/fuzz"},
    "rullst-orm-macros": {"rullst-orm/fuzz"},
}
PARSER = "rullst-orm-macros/src/parser.rs"


class Fixture(unittest.TestCase):
    """A real Git history shaped like the workspace; shared with test-fuzz-build-graph.py."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="rullst-fuzz-inputs-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.run_git("init", "-q")
        self.run_git("config", "user.email", "fixture@example.invalid")
        self.run_git("config", "user.name", "Fixture")
        # A detached `git gc --auto` can still be writing objects/pack when the
        # temporary directory is removed (ENOTEMPTY), so keep git in the foreground.
        self.run_git("config", "gc.auto", "0")
        self.run_git("config", "maintenance.auto", "false")
        self.v13_inventory = json.loads((ROOT / ".github/fuzz-targets.json").read_text())
        self.inventory = [item for item in self.v13_inventory if item["dir"] != "rullst-privacy/fuzz"]
        self.write(".github/fuzz-targets.json", json.dumps(self.inventory))
        self.write(".github/release-required-workflows.json",
                   '{"schema_version":2,"required_branch":"main"}')
        self.write(".github/workflows/fuzzing.yml", (ROOT / ".github/workflows/fuzzing.yml").read_text())
        self.write(".github/validate-fuzz-targets.py", "print('inventory')\n")
        self.write("Cargo.toml", '[workspace]\nmembers = ["rullst-*"]\n\n[workspace.dependencies]\n'
                   'rullst-core = { path = "rullst-core" }\nrullst-mail = { path = "rullst-mail" }\n'
                   'helper = { path = "helper" }\nrand = "0.10"\n')
        self.write(".cargo/config.toml", "[build]\njobs=2\n")
        self.write("Cargo.lock", "root lock")
        self.write("rust-toolchain.toml", '[toolchain]\nchannel = "1.99.1"\n')
        for path in ("CHANGELOG.md", "README.md", "docs/src/guide.md", "docs/src/tutorial.md",
                     "cargo-rullst/src/main.rs", "examples/blog/src/main.rs", ".github/workflows/ci.yml",
                     "shared/table.txt", "helper/src/lib.rs"):
            self.write(path, "original\n")
        self.write("cargo-rullst/Cargo.toml", '[package]\nname="cargo-rullst"\n[dependencies]\n'
                   'rullst-core = { workspace = true }\n')
        self.write("helper/Cargo.toml", '[package]\nname="helper"\n')
        self.crate("rullst-core", [])
        self.crate("rullst-orm-macros", [], lib='[lib]\nproc-macro = true\n')
        self.write(PARSER, "pub fn parse() {}\n")
        for parent in sorted({item["dir"].removesuffix("/fuzz") for item in self.inventory}):
            self.crate(parent, PARENT_DEPENDENCIES.get(parent, []))
        self.write("rullst-mail/Cargo.toml", '[package]\nname="mail"\nversion="1.0.0"\n'
                   '[dependencies]\nrullst-core = { workspace = true }\n')
        for item in self.inventory:
            directory = item["dir"]
            self.write(f"{directory}/Cargo.toml", '[package]\nname="fixture"\n'
                       '[workspace]\nmembers = ["."]\n[dependencies]\nparent={path=".."}\n')
            self.write(f"{directory}/Cargo.lock", "locked graph")
            self.write(f'{directory}/fuzz_targets/{item["target"]}.rs', "fn main() {}\n")
        self.write("rullst-orm/fuzz/fuzz_targets/fuzz_parser.rs",
                   '#[path = "../../../rullst-orm-macros/src/parser.rs"]\nmod parser;\nfn main() {}\n')
        self.base = Snapshot(self.commit(), self.root)

    def crate(self, directory, dependencies, lib=""):
        self.write(f"{directory}/Cargo.toml", f'[package]\nname="{directory}"\n{lib}'
                   "[dependencies]\n" + "".join(f"{line}\n" for line in dependencies))
        self.write(f"{directory}/src/lib.rs", "pub fn shared() {}\n")

    def run_git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, stderr=subprocess.PIPE).decode().strip()

    def write(self, path, text):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)

    def append(self, path, text):
        with (self.root / path).open("a") as output:
            output.write(text)

    def touch(self, path):
        """Change a file's blob without changing its meaning (or create it)."""
        if (self.root / path).exists():
            self.append(path, "\n# changed\n" if path.endswith((".toml", ".py", ".gitattributes"))
                        else "\n// changed\n")
        else:
            self.write(path, "fn main() {}\n" if path.endswith(".rs") else "new\n")

    def commit(self):
        self.run_git("add", "-A", ".")
        self.run_git("commit", "-qm", "test(fuzz): change fixture", "--allow-empty")
        return self.run_git("rev-parse", "HEAD")

    def changed(self):
        current = Snapshot(self.commit(), self.root)
        return {directory for directory in self.base.directories
                if self.base.fingerprint(directory) != current.fingerprint(directory)}

    def rebase(self):
        """Use the current commit as the evidence source for the next change."""
        self.base = Snapshot(self.commit(), self.root)
        return self.base

    def reset(self):
        self.run_git("reset", "-q", "--hard", self.base.sha)
        self.run_git("clean", "-qfd")


class InputTests(Fixture):
    def test_fixture_graph_matches_cargo_path_dependencies(self):
        self.assertEqual(self.base.fail_closed, {})
        self.assertEqual(self.base.reached["rullst/fuzz"], ["rullst", "rullst-core", "rullst-mail", "rullst/fuzz"])
        self.assertEqual(self.base.reached["rullst-orm/fuzz"], ["rullst-orm", "rullst-orm-macros", "rullst-orm/fuzz"])
        self.assertEqual(self.base.reached["rullst-iot/fuzz"], ["rullst-iot", "rullst-iot/fuzz"])

    def test_release_line_is_read_from_the_selected_git_object(self):
        self.write(".github/release-required-workflows.json",
                   '{"schema_version":3,"required_major":13,"required_branch":"v13"}')
        for item in self.v13_inventory:
            if item not in self.inventory:
                directory = item["dir"]
                self.write(f"{directory}/Cargo.toml", '[package]\nname="privacy-fixture"\n[dependencies]\nparent={path=".."}\n')
                self.write(f"{directory}/Cargo.lock", "locked graph")
                self.write(f'{directory}/fuzz_targets/{item["target"]}.rs', "fn main() {}\n")
        self.write(".github/fuzz-targets.json", json.dumps(self.v13_inventory))
        current = Snapshot(self.commit(), self.root)
        self.assertEqual(len(current.inventory), 42)
        self.assertEqual(len(Snapshot(self.base.sha, self.root).inventory), 40)
        self.assertEqual(current.release_branch, "v13")
        self.assertEqual(Snapshot(self.base.sha, self.root).release_branch, "main")
        self.assertNotEqual(current.global_hash, self.base.global_hash)

    def test_documentation_changelog_readme_cli_examples_and_other_ci_files_are_not_inputs(self):
        cases = {
            "docs": ("docs/src/guide.md", "docs/src/new-page.md", "docs/book.toml"),
            "changelog/readme": ("CHANGELOG.md", "README.md", "ROADMAP.md"),
            "cli": ("cargo-rullst/src/main.rs", "cargo-rullst/Cargo.toml", "cargo-rullst/templates/x.rs"),
            "other": ("examples/blog/src/main.rs", ".github/workflows/ci.yml", ".github/new-check.py",
                      "new-unclassified-input", "deny.toml", "oss-fuzz/projects/rullst/build.sh"),
        }
        for name, paths in cases.items():
            with self.subTest(name):
                self.reset()
                for path in paths:
                    self.write(path, "changed\n")
                self.assertEqual(self.changed(), set())
        self.reset()
        (self.root / "docs/src/guide.md").unlink()
        (self.root / "CHANGELOG.md").chmod(0o755)
        self.assertEqual(self.changed(), set())

    def test_reviewed_fixture_and_dev_dependency_changes_preserve_fuzz_inputs(self):
        for path in (".github/mobile-ui-browser-smoke.mjs",
                     ".github/billing-csp-browser-smoke.mjs", "Cargo.lock", "WORKFLOWS.md",
                     ".github/audit-packages.sh", ".github/test-package-release.py",
                     ".github/test-packaged-distribution.sh",
                     "deny.toml", "docs/src/security-advisory-exceptions.md",
                     ".github/workflows/supply-chain.yml",
                     ".github/workflows/crypto-containment.yml",
                     ".github/workflows/pr-security-evidence.yml"):
            self.write(path, "changed test or non-input")
        self.append("rullst-mail/Cargo.toml", '[dev-dependencies]\nrand={workspace=true}\n')
        self.assertEqual(self.changed(), set())

    def test_former_in_package_exemptions_are_inputs_of_the_packages_that_reach_them(self):
        for path, expected in (("rullst-mail/tests/feedback.rs", REACHES["rullst-mail"]),
                               ("rullst-core/src/server/stack_tests.rs", REACHES["rullst-core"]),
                               ("rullst-core/src/server/dev_telemetry/n_plus_one_tests.rs",
                                REACHES["rullst-core"])):
            with self.subTest(path=path):
                self.reset()
                self.write(path, "pub fn changed() {}\n")
                self.assertEqual(self.changed(), expected)

    def document_review_fixture(self):
        # Exercise every reviewed path against real blobs in the fixture's Git
        # history. Stub only the trusted review table, not source discovery.
        # The core crate consumes README.md, so that document stays an input.
        review = {}
        for path in reviewed_document_blobs():
            self.write(path, "original documentation\n")
            before = self.run_git("hash-object", path)
            self.write(path, "reviewed publication documentation\n")
            after = self.run_git("hash-object", path)
            self.write(path, "original documentation\n")
            review[path] = [before, after]
        self.write("rullst-core/src/lib.rs", 'pub const TEXT: &str = include_str!("../../README.md");\n')
        policy = patch("fuzz_evidence_inputs.reviewed_document_blobs", return_value=review)
        policy.start()
        self.addCleanup(policy.stop)
        self.rebase()
        return review

    def test_exact_reviewed_document_contents_preserve_all_package_inputs(self):
        for path in self.document_review_fixture():
            self.write(path, "reviewed publication documentation\n")
        self.assertEqual(self.changed(), set())

    def test_consumed_document_third_blob_mode_and_deletion_invalidate_consumers(self):
        self.document_review_fixture()
        for operation in ("content", "delete", "executable"):
            with self.subTest(operation=operation):
                self.reset()
                target = self.root / "README.md"
                if operation == "content":
                    self.write("README.md", "unreviewed later document contents\n")
                elif operation == "delete":
                    target.unlink()
                else:
                    target.chmod(0o755)
                self.assertEqual(self.changed(), REACHES["rullst-core"])
        self.reset()
        self.write("docs/src/unreviewed.md", "reviewed publication documentation\n")
        self.assertEqual(self.changed(), set())

    def test_new_document_consumer_invalidates_the_packages_that_reach_it(self):
        self.write("rullst-iot/src/lib.rs", 'pub const TEXT: &str = include_str!("../../README.md");\n')
        self.assertEqual(self.changed(), {"rullst-iot/fuzz"})
        self.rebase()
        self.write("README.md", "now consumed by a fuzzed crate\n")
        self.assertEqual(self.changed(), {"rullst-iot/fuzz"})

    def test_reached_crate_change_invalidates_only_the_packages_that_reach_it(self):
        for package, expected in REACHES.items():
            for path in (f"{package}/src/lib.rs", f"{package}/Cargo.toml",
                         f"{package}/tests/unreviewed.rs", f"{package}/README.md"):
                with self.subTest(path=path):
                    self.reset()
                    if path.endswith("Cargo.toml"):
                        # The mail manifest's identity is its parsed content.
                        self.append(path, "\n[features]\nchanged = []\n")
                    else:
                        self.touch(path)
                    self.assertEqual(self.changed(), expected)
        self.reset()
        (self.root / "rullst-core/src/lib.rs").unlink()
        self.assertEqual(self.changed(), REACHES["rullst-core"])
        self.reset()
        (self.root / "rullst-core/src/lib.rs").chmod(0o755)
        self.assertEqual(self.changed(), REACHES["rullst-core"])

    def test_orm_macro_parser_still_invalidates_orm_fuzzing_through_either_route(self):
        self.write(PARSER, "pub fn parse() { let _ = 1; }\n")
        self.assertEqual(self.changed(), {"rullst-orm/fuzz"})
        # Without the Cargo edge, the harness `#[path]` alone still reaches it.
        self.reset()
        self.write("rullst-orm/Cargo.toml", '[package]\nname="rullst-orm"\n')
        self.rebase()
        self.assertEqual(self.base.reached["rullst-orm/fuzz"], ["rullst-orm", "rullst-orm/fuzz"])
        self.write(PARSER, "pub fn parse() { let _ = 2; }\n")
        self.assertEqual(self.changed(), {"rullst-orm/fuzz"})
        self.write("rullst-orm-macros/src/lib.rs", "pub fn unrelated() {}\n")
        self.assertEqual(self.changed(), {"rullst-orm/fuzz"})  # only the parser commit above
        self.rebase()
        self.write("rullst-orm-macros/src/lib.rs", "pub fn unrelated_again() {}\n")
        self.assertEqual(self.changed(), set())

    def test_global_inputs_invalidate_every_package(self):
        for path in ("Cargo.toml", "rust-toolchain.toml", "rust-toolchain", ".cargo/config.toml",
                     ".cargo/mutants.toml", ".gitattributes", "shared/.gitattributes",
                     "docs/.gitattributes", ".github/validate-fuzz-targets.py", ".github/release_line.py"):
            with self.subTest(path=path):
                self.reset()
                self.touch(path)
                self.assertEqual(self.changed(), self.base.directories)

    def test_new_path_dependency_brings_the_new_crate_into_the_scope(self):
        self.write("helper/src/lib.rs", "pub fn unrelated() {}\n")
        self.assertEqual(self.changed(), set())
        original = self.base
        for section, spec in (("dependencies", 'helper = { path = "../helper" }'),
                              ("dependencies", "helper = { workspace = true }"),
                              ("dev-dependencies", 'helper = { path = "../helper" }'),
                              ('target."cfg(unix)".dependencies', 'helper = { path = "../helper" }'),
                              ("patch.crates-io", 'helper = { path = "../helper" }'),
                              ("replace", '"helper:0.1.0" = { path = "../helper" }')):
            with self.subTest(section=section, spec=spec):
                self.base = original
                self.reset()
                self.write("rullst-iot/Cargo.toml", f'[package]\nname="rullst-iot"\n[{section}]\n{spec}\n')
                self.assertEqual(self.changed(), {"rullst-iot/fuzz"})
                self.assertIn("helper", self.rebase().reached["rullst-iot/fuzz"])
                self.write("helper/src/lib.rs", "pub fn now_fuzzed() {}\n")
                self.assertEqual(self.changed(), {"rullst-iot/fuzz"})

    def test_registry_dependency_changes_stay_inside_their_manifest(self):
        for section in ("dependencies", "build-dependencies", 'target."cfg(unix)".dependencies'):
            with self.subTest(section=section):
                self.reset()
                self.write("rullst-mail/Cargo.toml", '[package]\nname="mail"\nversion="1.0.0"\n'
                           f'[{section}]\nnew_dep="1"\n[features]\nrullst-core = []\n')
                self.assertEqual(self.changed(), REACHES["rullst-mail"])

    def test_include_outside_the_package_is_an_input_of_its_consumers(self):
        self.write("rullst-iot/src/lib.rs", 'pub const T: &str = r#include_str!("../../shared/table.txt");\n'
                   '#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/src/guide.md"))]\n'
                   "pub struct Documented;\n")
        self.rebase()
        for path in ("shared/table.txt", "docs/src/guide.md"):
            with self.subTest(path=path):
                self.reset()
                self.write(path, "changed\n")
                self.assertEqual(self.changed(), {"rullst-iot/fuzz"})
        self.reset()
        (self.root / "shared/table.txt").unlink()
        self.assertEqual(self.changed(), {"rullst-iot/fuzz"})
        self.reset()
        self.write("docs/src/tutorial.md", "unrelated documentation\n")
        self.assertEqual(self.changed(), set())

    def test_included_code_is_scanned_and_corpus_seeds_are_data(self):
        self.write("rullst-iot/src/lib.rs", 'include!("generated.rs.in");\n')
        self.write("rullst-iot/src/generated.rs.in", 'const T: &str = include_str!("../../shared/table.txt");\n')
        self.write("rullst-iot/fuzz/corpus/fuzz_ota/seed.rs", 'fn main() { std::fs::read("x"); (\n')
        self.rebase()
        self.assertEqual(self.base.fail_closed, {})
        self.write("shared/table.txt", "reached through included code\n")
        self.assertEqual(self.changed(), {"rullst-iot/fuzz"})

    def test_files_around_reached_and_included_code_are_inputs(self):
        lib = "rullst-iot/src/lib.rs"
        self.write("libs/helper/Cargo.toml", '[package]\nname="lib-helper"\n')
        self.write("libs/helper/src/lib.rs", "pub fn f() {}\n")
        self.write("nested/Cargo.toml", '[workspace]\nmembers = ["member"]\n[workspace.package]\nversion = "1.0.0"\n')
        self.write("nested/member/Cargo.toml", '[package]\nname="member"\nversion.workspace = true\n')
        self.write("nested/member/src/lib.rs", "pub fn g() {}\n")
        self.write("rullst-iot/Cargo.toml", '[package]\nname="rullst-iot"\n[dependencies]\n'
                   'h = { path = "../libs/helper" }\nm = { path = "../nested/member" }\n')
        # Module trees of included code outside every package, and every
        # inline-module depth of a `#[path]` in it.
        self.write(lib, 'include!("../../shared/tree/gen.rs");\ninclude!("../../shared/deep/x.rs");\n')
        self.write("shared/tree/gen.rs", "mod inner;\n")
        self.write("shared/tree/inner.rs", "pub fn i() {}\n")
        self.write("shared/deep/x.rs", 'mod outer { #[path = "../../y.rs"] mod m; }\n')
        self.write("shared/y.rs", "pub fn y() {}\n")
        self.rebase()
        self.assertEqual(self.base.fail_closed, {})
        for path in ("libs/rust-toolchain.toml", "libs/.cargo/config.toml", "nested/Cargo.toml",
                     "shared/tree/inner.rs", "shared/y.rs"):
            with self.subTest(path=path):
                self.reset()
                self.touch(path)
                self.assertEqual(self.changed(), {"rullst-iot/fuzz"})

    def test_macro_forwarded_book_includes_are_inputs_of_the_facade_only(self):
        self.write("rullst/src/book.rs", "macro_rules! tutorial {\n"
                   "    ($name:ident, $path:literal) => { #[doc = include_str!($path)] pub mod $name {} };\n}\n"
                   'tutorial!(guide, "../../docs/src/tutorial.md");\n')
        self.rebase()
        self.write("docs/src/tutorial.md", "changed tutorial\n")
        self.assertEqual(self.changed(), {"rullst/fuzz"})
        self.write("docs/src/guide.md", "changed unrelated guide\n")
        self.assertEqual(self.changed(), {"rullst/fuzz"})  # still only the tutorial commit
        self.rebase()
        self.write("docs/src/guide.md", "changed unrelated guide again\n")
        self.assertEqual(self.changed(), set())

    def test_every_package_harness_and_lock_invalidates_its_entire_package(self):
        for directory in self.base.directories:
            parent = directory.removesuffix("/fuzz")
            expected = {directory} | REACHES.get(parent, set())
            for suffix in ("Cargo.lock", "extra-fixture.txt"):
                with self.subTest(directory=directory, suffix=suffix):
                    self.reset()
                    self.write(f"{directory}/{suffix}", "changed")
                    self.assertEqual(self.changed(), expected)
        self.reset()
        item = self.inventory[0]
        self.write(f'{item["dir"]}/fuzz_targets/{item["target"]}.rs', "fn main() { let _ = 42; }")
        self.assertEqual(self.changed(), {item["dir"]})

    def test_shared_execution_contract_includes_budget_flags_tools_and_runner(self):
        path = ".github/workflows/fuzzing.yml"
        original = (self.root / path).read_text()
        for old, new in (("nightly-2026-08-21", "nightly-2026-08-22"),
                         ("-max_len=2048", "-max_len=1024"),
                         ("ubuntu-latest", "ubuntu-24.04"),
                         ("tool: cargo-fuzz@0.13.2", "tool: cargo-fuzz@0.13.3"),
                         ('-max_total_time="$CAMPAIGN_SECONDS"', '-max_total_time=1')):
            with self.subTest(change=old):
                self.assertNotEqual(execution_contract(original), execution_contract(original.replace(old, new)))
        legacy = original.replace("    if: needs.targets.outputs.selected_count != '0'\n", "")
        self.assertEqual(execution_contract(original), execution_contract(legacy))
        with_defaults = original.replace("\njobs:\n", "\ndefaults:\n  run:\n    shell: sh\n\njobs:\n")
        self.assertNotEqual(execution_contract(original), execution_contract(with_defaults))
        with_job = original + "\n  unknown:\n    runs-on: ubuntu-latest\n    steps: []\n"
        self.assertNotEqual(execution_contract(original), execution_contract(with_job))
        for unsupported in (original.replace("\njobs:\n", "\n'defaults': {}\n\njobs:\n"),
                            original.replace("  preflight:\n", "  'preflight':\n"),
                            original.replace("  workflow_dispatch:", "  workflow_dispatch: &hidden")):
            with self.assertRaises(ValueError):
                execution_contract(unsupported)

    def test_scripts_named_by_the_fuzz_workflow_are_global_inputs(self):
        self.write(".github/fuzz_helper.py", "VALUE = 1\n")
        self.write(".github/validate-fuzz-targets.py", "import fuzz_helper\n")
        self.rebase()
        self.assertIn(".github/fuzz_helper.py", [entry[0] for entry in self.base.global_entries])
        self.write(".github/fuzz_helper.py", "VALUE = 2\n")
        self.assertEqual(self.changed(), self.base.directories)

    def test_duplicate_target_or_unknown_inventory_shape_is_rejected(self):
        self.inventory[-1] = self.inventory[0]
        self.write(".github/fuzz-targets.json", json.dumps(self.inventory))
        with self.assertRaisesRegex(ValueError, "duplicate"):
            Snapshot(self.commit(), self.root)

    def test_v13_cannot_admit_the_legacy_forty_target_surface(self):
        self.write(".github/release-required-workflows.json",
                   '{"schema_version":3,"required_major":13,"required_branch":"v13"}')
        with self.assertRaisesRegex(ValueError, "exactly 42"):
            Snapshot(self.commit(), self.root)

    def test_v12_cannot_reinterpret_an_expanded_inventory(self):
        self.write(".github/fuzz-targets.json", json.dumps(self.v13_inventory))
        with self.assertRaisesRegex(ValueError, "exactly 40"):
            Snapshot(self.commit(), self.root)

    def test_symlink_and_missing_commit_fail_closed(self):
        (self.root / "outside").symlink_to("/tmp")
        with self.assertRaisesRegex(ValueError, "symlinks"):
            Snapshot(self.commit(), self.root)
        with self.assertRaises(ValueError):
            Snapshot("--help", self.root)


if __name__ == "__main__":
    unittest.main()
