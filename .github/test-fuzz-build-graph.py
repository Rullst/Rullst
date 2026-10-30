#!/usr/bin/env python3
"""Every fail-closed branch of the fuzz build graph, its scanner and the Cargo cross-check."""

import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

from fuzz_evidence_inputs import ROOT, Snapshot
from fuzz_source_scan import source_facts

_SPEC = importlib.util.spec_from_file_location(
    "fuzz_input_fixture", Path(__file__).with_name("test-fuzz-evidence-inputs.py"))
_FIXTURE = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_FIXTURE)
REACHES = _FIXTURE.REACHES


class FailClosedTests(_FIXTURE.Fixture):
    def closed_case(self, files, expected, reason):
        """Write `files` on the original base; the scope must close with `reason`."""
        self.base = self.original
        self.reset()
        for path, text in files.items():
            if text.startswith("+"):
                self.append(path, text[1:])
            else:
                self.write(path, text)
        closed = Snapshot(self.commit(), self.root)
        self.assertEqual(set(closed.fail_closed), expected)
        self.assertEqual({d for d, reached in closed.reached.items() if reached is None}, expected)
        self.assertTrue(any(reason in item for reasons in closed.fail_closed.values() for item in reasons),
                        closed.fail_closed)
        # Any later change, even an unreferenced document, now reruns it.
        self.base = closed
        self.write("docs/src/guide.md", "unrelated documentation\n")
        self.assertEqual(self.changed(), expected)

    def test_every_fail_closed_branch(self):
        self.original = self.base
        iot, orm, lib = {"rullst-iot/fuzz"}, {"rullst-orm/fuzz"}, "rullst-iot/src/lib.rs"
        manifest = '[package]\nname="rullst-iot"\n'
        helper = {"rullst-iot/Cargo.toml": manifest + '[dependencies]\nhelper = { path = "../helper" }\n'}
        macros = "rullst-orm-macros/src/lib.rs"
        fwd = 'macro_rules! fwd { ($p:literal) => { include_str!($p) }; }\n'
        cases = {
            "missing manifest": ({"rullst-iot/Cargo.toml": manifest + '[dependencies]\nhole = { path = "../hole" }\n'},
                                 iot, "path dependency without a package manifest"),
            "explicit workspace": ({**helper, "helper/Cargo.toml": '[package]\nname="helper"\nworkspace = "../x"\n'},
                                   iot, "explicit workspace root is not a workspace"),
            "unparsable ancestor": ({"rullst-iot/Cargo.toml": manifest + '[dependencies]\ni = { path = "../nest/i" }\n',
                                     "nest/Cargo.toml": "not toml [", "nest/i/Cargo.toml": '[package]\nname="i"\n'},
                                    iot, "unparsable manifest nest/Cargo.toml"),
            "excluded": ({**helper,
                          "Cargo.toml": '[workspace]\nexclude = ["helper"]\n[workspace.dependencies]\n'
                                        'rullst-core = { path = "rullst-core" }\nrullst-mail = { path = "rullst-mail" }\n'},
                         iot, "excluded from the workspace at ."),
            "inheritance without workspace": ({"Cargo.toml": "# no workspace\n"}, REACHES["rullst-core"],
                                              "workspace inheritance without a workspace root"),
            "build-dependencies": ({"rullst-iot/Cargo.toml": manifest + '[build-dependencies]\ncc = "1"\n'},
                                   iot, "build-dependencies run at build time"),
            "target build-dependencies": ({"rullst-iot/Cargo.toml": manifest + '[target."cfg(unix)".build-dependencies]\ncc = "1"\n'},
                                          iot, "build-dependencies run at build time"),
            "build.rs": ({"rullst-iot/build.rs": "fn main() {}\n"}, iot, "build script runs at build time"),
            "build setting": ({"rullst-iot/Cargo.toml": manifest.replace('"\n', '"\nbuild = "tools/b.rs"\n', 1)},
                              iot, "build script runs at build time"),
            "malformed table": ({"rullst-iot/Cargo.toml": 'dependencies = "x"\n' + manifest}, iot, "malformed dependency table"),
            "unresolved inherited": ({"rullst-iot/Cargo.toml": manifest + '[dependencies]\nnope = { workspace = true }\n'},
                                     iot, "unresolved inherited dependency nope"),
            "unsupported dependency": ({"rullst-iot/Cargo.toml": manifest + '[dependencies]\nodd = 5\n'},
                                       iot, "unsupported dependency odd"),
            "dependency escape": ({"rullst-iot/Cargo.toml": manifest + '[dependencies]\nfar = { path = "../../far" }\n'},
                                  iot, "dependency far path cannot be resolved"),
            "dependency placeholder": ({"rullst-iot/Cargo.toml": manifest + '[dependencies]\nv = { path = "{d}/../helper" }\n'},
                                       iot, "dependency v path cannot be resolved"),
            "malformed target": ({"rullst-iot/Cargo.toml": 'bin = ["x"]\n' + manifest}, iot, "malformed [bin] target"),
            "target outside": ({"rullst-iot/Cargo.toml": manifest + '[lib]\npath = "../shared/lib.rs"\n'},
                               iot, "[lib] source outside the package"),
            "cargo config": ({".cargo/config.toml": 'paths = ["../vendor"]\n[build]\njobs=2\n'},
                             self.base.directories, "unsupported Cargo configuration key 'paths'"),
            "root module tree": ({lib: 'include!("../../gen.rs");\n', "gen.rs": "mod inner;\n"}, iot, "root-level module tree"),
            "inclusion depth": ({lib: 'include!("../../shared/c0.rs");\n',
                                 **{f"shared/c{n}.rs": f'include!("c{n + 1}.rs");\n' for n in range(10)}},
                                iot, "too many nested source inclusions"),
            "harness I/O": ({"rullst-iot/fuzz/fuzz_targets/fuzz_ota.rs": 'fn main() { std::fs::read("x"); }\n'},
                            iot, "fuzz harness accesses files"),
            "proc-macro I/O": ({macros: '+fn read() { let _ = std::fs::read("x"); }\n'}, orm, "unreviewed compile-time access (fs"),
            "proc-macro exists probe": ({macros: '+fn f() -> bool { std::path::Path::new("x").exists() }\n'},
                                        orm, "unreviewed compile-time access (exists)"),
            "proc-macro module outside": ({macros: '+#[path = "../../shared/io.rs"]\nmod io;\n',
                                           "shared/io.rs": 'fn f() { std::fs::read("x"); }\n'},
                                          orm, "shared/io.rs: unreviewed compile-time access"),
            "proc-macro module tree outside": ({macros: '+#[path = "../../shared/pm/io.rs"]\nmod io;\n',
                                                "shared/pm/io.rs": "mod inner;\n",
                                                "shared/pm/inner.rs": 'fn f() { std::fs::read("x"); }\n'},
                                               orm, "shared/pm/inner.rs: unreviewed compile-time access"),
            "crate-type proc macro": ({**helper, "helper/Cargo.toml": '[package]\nname="helper"\n[lib]\ncrate-type = ["proc-macro"]\n',
                                       "helper/src/lib.rs": 'fn f() { std::fs::read("x"); }\n'},
                                      iot, "helper/src/lib.rs: unreviewed compile-time access"),
            "proc-macro climbing path": ({macros: '+fn f() -> &\'static str { "../shared" }\n'}, orm, "proc macro uses path '../shared'"),
            "lexer error": ({lib: "fn f() { (\n"}, iot, "cannot scan source"),
            "macro-defining macro": ({lib: 'macro_rules! make { ($n:ident) => { macro_rules! $n { () => {} } }; }\n'},
                                     iot, "macro_rules! that defines macros"),
            "renamed include": ({lib: "use std::include_str as inc;\n"}, iot, "built-in include_str! is renamed"),
            "file-reading macro": ({lib: "static M: () = sqlx::migrate!();\n"}, iot, "migrate! reads files"),
            "file-reading attribute": ({lib: '#[derive(Embed)]\n#[folder = "assets"]\nstruct A;\n'}, iot, "#[folder] reads files"),
            "climbing registry argument": ({lib: 'reg!("../x");\n'}, iot, "path-like argument '../x' of reg"),
            "absolute registry argument": ({lib: 'reg!("/etc/x");\n'}, iot, "path-like argument '/etc/x' of reg"),
            "climbing attribute argument": ({lib: '#[reg_attr(path = "../x")]\nfn f() {}\n'}, iot, "of reg_attr"),
            "opaque macro_rules": ({lib: 'macro_rules! m { ($p:literal) => { reg!($p) }; }\nm!("../x");\n'},
                                   iot, "path-like argument '../x' of m"),
            "climbing workspace proc macro": ({macros: '+#[proc_macro]\npub fn catalog(i: TokenStream) -> TokenStream { i }\n',
                                               "rullst-orm/src/lib.rs": 'catalog!(dir = "../locales");\n'},
                                              orm, "path-like argument '../locales' of catalog"),
            "absolute include": ({lib: 'const T: &str = include_str!("/etc/hostname");\n'}, iot, "unresolvable data path '/etc/hostname'"),
            "escaping include": ({lib: 'const T: &str = include_str!("../../../out");\n'}, iot, "data path '../../../out' cannot be resolved"),
            "placeholder include": ({lib: 'const T: &str = include_str!("{d}/../x.txt");\n'}, iot, "data path '{d}/../x.txt' cannot be resolved"),
            "placeholder module": ({lib: '#[path = "$D/../m.rs"]\nmod m;\n'}, iot, "mod path '$D/../m.rs' cannot be resolved"),
            "generated include": ({lib: 'fn f() { quote! { include_str!("../x") }; }\n'}, iot, "generated data path '../x'"),
            "computed include": ({lib: 'const T: &str = include_str!(concat!("a", "b"));\n'}, iot, "data path cannot be resolved statically"),
            "OUT_DIR climbing": ({lib: 'include!(concat!(env!("OUT_DIR"), "/../x.rs"));\n'}, iot, "code path cannot be resolved statically"),
            "manifest include in macro": ({lib: 'macro_rules! m { () => { include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/x")) }; }\n'},
                                          iot, "data path cannot be resolved statically"),
            "manifest without slash": ({lib: 'const T: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "x"));\n'},
                                       iot, "CARGO_MANIFEST_DIR data path 'x' cannot be resolved"),
            "manifest in outside code": ({lib: 'include!("../../shared/gen.rs");\n',
                                          "shared/gen.rs": 'const T: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/x"));\n'},
                                         iot, "CARGO_MANIFEST_DIR data path '/x' cannot be resolved"),
            "metavariable outside macro": ({lib: "fn f() { include_str!($p); }\n"}, iot, "macro variable outside macro_rules!"),
            "renamed literal macro": ({lib: 'macro_rules! inc { () => { include_str!("x.txt") }; }\nuse inc as other;\nconst T: &str = other!();\n'},
                                      iot, "inside macro inc! has unknown call sites"),
            "literal macro from quote": ({lib: 'macro_rules! inc { () => { include_str!("x.txt") }; }\nfn f() { quote! { inc!() }; }\n'},
                                         iot, "inside macro inc! has unknown call sites"),
            "renamed forwarding macro": ({lib: fwd + 'pub(crate) use fwd as renamed;\nconst C: &str = renamed!("data.txt");\n'},
                                         iot, "argument of renamed macro fwd!"),
            "nested forwarded call": ({lib: fwd.replace("literal", "expr") + 'macro_rules! p { () => { "data.txt" }; }\nconst C: &str = fwd!(p!());\n'},
                                      iot, "argument of macro fwd! cannot be resolved"),
            "forwarded variable": ({lib: fwd + 'macro_rules! outer { ($q:literal) => { fwd!($q) }; }\nconst C: &str = outer!("data.txt");\n'},
                                   iot, "argument of macro fwd! cannot be resolved"),
            "forwarding call in quote": ({lib: fwd + 'fn f() { quote! { fwd!("data.txt") }; }\n'},
                                         iot, "argument of macro fwd! cannot be resolved"),
            "forwarding call in renamed macro": ({lib: fwd + 'macro_rules! outer { () => { fwd!("data.txt") }; }\nuse outer as o;\nconst C: &str = o!();\n'},
                                                 iot, "argument of macro fwd! cannot be resolved"),
        }
        macro_manifest = '[package]\nname="rullst-orm-macros"\n[lib]\nproc-macro = true\n[dependencies]\n'
        cases.update({
            "config in the fuzz directory": ({"rullst-iot/fuzz/.cargo/config.toml": '[env]\nX = "y"\n'},
                                             iot, "rullst-iot/fuzz/.cargo/config.toml: unsupported Cargo configuration key 'env'"),
            "config in a parent directory": ({"rullst-iot/.cargo/config.toml": '[patch.crates-io]\nx = { path = "x" }\n'},
                                             iot, "rullst-iot/.cargo/config.toml: unsupported Cargo configuration key 'patch'"),
            "harness environment": ({"rullst-iot/fuzz/fuzz_targets/fuzz_ota.rs": 'fn main() { std::env::var("X"); }\n'},
                                    iot, "fuzz harness accesses"),
            "harness process": ({"rullst-iot/fuzz/fuzz_targets/fuzz_ota.rs": 'fn main() { std::process::exit(0); }\n'},
                                iot, "fuzz harness accesses"),
            "harness command": ({"rullst-iot/fuzz/fuzz_targets/fuzz_ota.rs": 'fn main() { Command::new("x"); }\n'},
                                iot, "fuzz harness accesses"),
            "proc-macro climbing path in a call": ({macros: '+fn f() -> String { format!("{}", "../shared") }\n'},
                                                   orm, "proc macro uses path '../shared'"),
            "proc-macro path dependency": ({"rullst-orm-macros/Cargo.toml": macro_manifest + 'helper = { path = "../helper" }\n'},
                                           orm, "proc macro has path dependency helper"),
            "proc-macro unreviewed crate": ({"rullst-orm-macros/Cargo.toml": macro_manifest + 'fs-err = "3"\n'},
                                            orm, "proc macro depends on unreviewed crate fs-err"),
            "proc-macro renamed crate": ({"rullst-orm-macros/Cargo.toml": macro_manifest + 'syn = { package = "dotenvy", version = "1" }\n'},
                                         orm, "proc macro depends on unreviewed crate dotenvy"),
            "proc-macro target crate": ({"rullst-orm-macros/Cargo.toml": macro_manifest + '[target."cfg(unix)".dependencies]\ncargo_metadata = "1"\n'},
                                        orm, "proc macro depends on unreviewed crate cargo_metadata"),
            "cfg_attr file attribute": ({lib: '#[cfg_attr(all(), folder = "assets")]\nstruct A;\n'}, iot, "#[folder] reads files"),
            "cfg_attr registry attribute": ({lib: '#[cfg_attr(all(), reg_attr(path = "../x"))]\nfn f() {}\n'},
                                            iot, "path-like argument '../x' of reg_attr"),
            "qualified std-named attribute": ({lib: '#[sqlx::test(migrations = "../m")]\nasync fn t() {}\n'},
                                              iot, "path-like argument '../m' of sqlx::test"),
            "imported std-named attribute": ({lib: 'use sqlx::test;\n#[test(migrations = "../m")]\nfn t() {}\n'},
                                             iot, "path-like argument '../m' of test"),
            "qualified std-named macro": ({lib: 'fn f() { other::concat!("../x"); }\n'}, iot, "of other::concat"),
            "built-in under a registry macro": ({lib: 'reg!(concat!("../a", "b"));\n'}, iot, "path-like argument '../a' of reg"),
            "path from a nested macro": ({lib: 'macro_rules! p { () => { "../x" }; }\nconst T: &str = reg!(p!());\n'},
                                         iot, "reg! receives a path from a nested macro"),
            "environment placeholder": ({lib: 'reg!("$GITHUB_WORKSPACE/shared");\n'}, iot, "'$GITHUB_WORKSPACE/shared' of reg"),
            "windows placeholder": ({lib: 'reg!("%APPDATA%/shared");\n'}, iot, "'%APPDATA%/shared' of reg"),
            "renamed file-reading macro": ({lib: "use sqlx::query as q;\n"}, iot, "file-reading query is renamed"),
            "renamed file-reading attribute": ({lib: "use rust_embed::folder as f;\n"}, iot, "file-reading folder is renamed"),
            "unimported registry macro": ({lib: 'fn f() { json!({"a": "/x"}); }\n'}, iot, "path-like argument '/x' of json"),
            "registry path named like an export": ({macros: '+#[proc_macro]\npub fn page(i: TokenStream) -> TokenStream { i }\n',
                                                    "rullst-orm/src/lib.rs": 'other::page!("/a");\n'},
                                                   orm, "path-like argument '/a' of other::page"),
            "registry path named like local macro_rules": ({lib: 'macro_rules! t { ($p:literal) => { stringify!($p) }; }\n'
                                                                 'other::t!("../x");\n'},
                                                           iot, "path-like argument '../x' of other::t"),
            "opaque through an attribute": ({lib: 'macro_rules! m { ($p:literal) => { #[reg_attr($p)] fn f() {} }; }\nm!("../x");\n'},
                                            iot, "path-like argument '../x' of m"),
        })
        for call in ("is_file", "is_dir", "is_symlink", "metadata", "try_exists", "symlink_metadata"):
            cases[f"proc-macro {call} probe"] = ({macros: f'+fn f() {{ let _ = std::path::Path::new("x").{call}(); }}\n'},
                                                orm, f"unreviewed compile-time access ({call})")
        for name, (files, expected, reason) in cases.items():
            with self.subTest(name):
                self.closed_case(files, expected, reason)

    def test_patterns_that_read_nothing_stay_bounded(self):
        macros = "rullst-orm-macros/src/lib.rs"
        cases = {
            "std macro and plain literals": {"rullst-iot/src/lib.rs": 'const WAF: &str = "../../../../etc/passwd";\n'
                                             'fn f() { assert_eq!("../x", "/y"); println!("/z"); }\n'},
            "transparent macro_rules": {"rullst-iot/src/lib.rs": 'macro_rules! r { ($p:expr) => { route($p) }; }\nr!("/a");\n'},
            "workspace proc macro with a route": {macros: '+#[proc_macro]\npub fn page(i: TokenStream) -> TokenStream { i }\n',
                                                  "rullst-orm/src/lib.rs": 'page!("/a");\n#[page(path = "/b")]\nfn f() {}\n'},
            "renamed workspace proc macro": {macros: '+#[proc_macro]\npub fn page(i: TokenStream) -> TokenStream { i }\n',
                                             "rullst-orm/src/lib.rs": 'pub use rullst_orm_macros::page as route;\nroute!("/a");\n'
                                                                      'extern crate self as orm;\norm::route!("/b");\n'},
            "reviewed proc-macro dependencies": {"rullst-orm-macros/Cargo.toml": '[package]\nname="rullst-orm-macros"\n[lib]\n'
                                                 'proc-macro = true\n[dependencies]\nsyn = "2"\nquote = "1"\nproc-macro2 = "1"\n'
                                                 'toml = "1"\n[dev-dependencies]\ntrybuild = "1"\n'},
            "qualified built-ins": {"rullst-iot/src/lib.rs": 'fn f() { std::assert!(x("../a")); ::core::concat!("../b"); quote::quote!("../c"); }\n'},
            "reviewed registry macros": {"rullst-iot/src/lib.rs": 'use serde_json::json;\n'
                                         'fn f() { json!({"a": "/x"}); libfuzzer_sys::fuzz_target!(|d| { "/" }); }\n'},
            "segment comparison in a proc macro": {macros: '+fn ok(s: &str) -> bool { s != ".." && ".." != s }\n'},
            "generated runtime code": {macros: '+fn g() { quote! { std::fs::read("../x") }; }\n'},
            "out-dir include": {"rullst-iot/src/lib.rs": 'include!(concat!(env!("OUT_DIR"), "/gen.rs"));\n'},
            "field named like a probe": {macros: '+struct S { exists: bool, metadata: u8 }\n'},
        }
        for name, files in cases.items():
            with self.subTest(name):
                self.reset()
                for path, text in files.items():
                    self.append(path, text[1:]) if text.startswith("+") else self.write(path, text)
                self.assertEqual(Snapshot(self.commit(), self.root).fail_closed, {})

    def test_reviewed_proc_macro_reader_is_accepted_only_at_its_blob(self):
        self.append("rullst-orm-macros/src/lib.rs", 'fn read() { let _ = std::fs::read("x"); }\n')
        commit = self.commit()
        oid = self.run_git("rev-parse", f"{commit}:rullst-orm-macros/src/lib.rs")
        with patch.dict("fuzz_build_graph.REVIEWED_COMPILE_IO", {"rullst-orm-macros/src/lib.rs": {oid}}):
            self.assertEqual(Snapshot(commit, self.root).fail_closed, {})
            self.append("rullst-orm-macros/src/lib.rs", "// edited after review\n")
            self.assertIn("rullst-orm/fuzz", Snapshot(self.commit(), self.root).fail_closed)

    def test_cargo_metadata_cross_check_reports_what_the_scope_misses(self):
        spec = importlib.util.spec_from_file_location("check_graph", ROOT / ".github/check-fuzz-build-graph.py")
        check = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(check)

        def metadata(*packages, links=()):
            items = [{"id": path, "name": path, "source": None, "manifest_path": str(self.root / path / "Cargo.toml"),
                      "targets": [{"kind": list(kinds)}]} for path, kinds in packages]
            items += [{"id": name, "name": name, "source": "registry", "manifest_path": "/x",
                       "targets": [{"kind": ["lib"]}]} for name in ("syn", "fs-err", "trybuild")]
            nodes = [{"id": item["id"], "deps": []} for item in items]
            for node in nodes:
                if node["id"] == "rullst-orm-macros":
                    node["deps"] = [{"pkg": name, "dep_kinds": [{"kind": kind}]} for name, kind in links]
            return {"packages": items, "resolve": {"nodes": nodes}}

        fuzz = "rullst-orm/fuzz"
        reached = ((fuzz, ["bin"]), ("rullst-orm", ["lib"]), ("rullst-orm-macros", ["proc-macro"]))
        consistent = metadata(*reached, links=(("syn", None), ("trybuild", "dev")))
        self.assertEqual(check.problems(self.base, fuzz, consistent, self.root), [])
        for links, message in (((("rullst-orm", None),), "proc macro rullst-orm-macros links path package rullst-orm"),
                               ((("fs-err", None),), "proc macro rullst-orm-macros links unreviewed crate fs-err")):
            with self.subTest(message=message):
                found = check.problems(self.base, fuzz, metadata(*reached, links=links), self.root)
                self.assertTrue(any(message in item for item in found), found)
        for packages, message in (
                ([("helper", ["lib"])], "Cargo builds helper, which the scope does not reach"),
                ([("rullst-orm", ["custom-build", "lib"])], "rullst-orm has a build script"),
                ([("rullst-orm", ["proc-macro"])], "rullst-orm is a proc macro the scope does not check")):
            with self.subTest(message=message):
                found = check.problems(self.base, fuzz, metadata((fuzz, ["bin"]), *packages), self.root)
                self.assertTrue(any(message in item for item in found), found)


class SourceScanTests(unittest.TestCase):
    def refs(self, source):
        facts = source_facts(source.encode())
        self.assertIsNone(facts.error)
        return [(kind, form, value) for kind, form, value, _ in facts.refs]

    def test_comments_and_strings_never_count_as_includes(self):
        self.assertEqual(self.refs('// include_str!("../x")\n/* /* include!("../y") */ */\n'
                                   'const S: &str = "include_str!(\\"../z\\")";\n'
                                   'const R: &str = r#"#[path = "../w"]"#;\n'), [])

    def test_include_forms_and_path_attributes_are_classified(self):
        self.assertEqual(self.refs('#![cfg_attr(docsrs, doc = include_str!("../README.md"))]\n'
                                   '#[cfg_attr(unix, path = "unix.rs")] mod os;\n'
                                   'const B: &[u8] = include_bytes!(r"data.bin");\n'
                                   'include!(concat!(env!("OUT_DIR"), "/gen.rs"));\n'
                                   'const C: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/a", "/b"));\n'
                                   'catalog!(default = "en", dir = "../locales");\n'
                                   '#[r#path = "raw.rs"] mod raw;\n'
                                   'const R: &str = r#include_str!("raw.txt");\n'),
                         [("data", "lit", "../README.md"), ("mod", "lit", "unix.rs"),
                          ("data", "lit", "data.bin"), ("code", "out", None),
                          ("data", "manifest", "/a/b"), ("mod", "lit", "raw.rs"), ("data", "lit", "raw.txt")])

    def test_path_like_literals_record_their_innermost_owner(self):
        facts = source_facts(b'catalog!(dir = "../l");\n#[crate::reg(path = "/a")] fn f() {}\n'
                             b'fn g() { assert!(x("../y")); let w = "../z"; s != ".."; }\n')
        self.assertEqual([(chain, value, compared) for chain, value, _, compared in facts.literals],
                         [((("call", "catalog"),), "../l", False), ((("attr", "crate::reg"),), "/a", False),
                          ((("call", "assert"),), "../y", False), ((), "../z", False), ((), "..", True)])
        self.assertTrue(source_facts(b"macro_rules! m { ($n:ident) => { macro_rules! $n {} } }").defines_macros)
        self.assertEqual(source_facts(b"#[proc_macro]\npub fn html(i: T) -> T { i }\n"
                                      b"#[proc_macro_derive(Model, attributes(orm))] pub fn d() {}").exports,
                         {"html", "orm"})

    def test_generated_code_is_not_proc_macro_io_but_computed_includes_are(self):
        facts = source_facts(b'fn f() { quote! { std::fs::read("x") }; }')
        self.assertEqual(facts.compile_io, [])
        facts = source_facts(b"fn f() { quote! { include_bytes!(#path) }; }")
        self.assertEqual(facts.compile_io, ["generated data path"])
        self.assertEqual(source_facts(b'fn f() { std::env::var("X"); }').compile_io, ["env"])

    def test_unbalanced_source_is_reported(self):
        self.assertIsNotNone(source_facts(b"fn f() { (").error)
        self.assertIsNotNone(source_facts(b"/* unterminated").error)

if __name__ == "__main__":
    unittest.main()
