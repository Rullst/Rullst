#!/usr/bin/env python3
"""Tests for the public-API listing rendered by public-api.py."""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("public-api.py")
SPEC = importlib.util.spec_from_file_location("public_api", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

NO_GENERICS = {"params": [], "where_predicates": []}
HEADER = {"is_const": False, "is_async": False, "is_unsafe": False, "abi": "Rust"}


def item(name, inner, visibility="public", hidden_attrs=None):
    return {"name": name, "visibility": visibility, "attrs": hidden_attrs or [],
            "deprecation": None, "inner": inner}


def function(inputs, output):
    return {"function": {"sig": {"inputs": inputs, "output": output, "is_c_variadic": False},
                         "generics": NO_GENERICS, "header": HEADER, "has_body": True}}


def impl(trait, items, synthetic=False):
    return {"impl": {"is_unsafe": False, "generics": NO_GENERICS, "provided_trait_methods": [],
                     "trait": trait, "for": {"resolved_path": {"path": "Point", "id": 1, "args": None}},
                     "items": items, "is_negative": False, "is_synthetic": synthetic, "blanket_impl": None}}


def crate(format_version=MODULE.FORMAT_VERSION):
    point = {"resolved_path": {"path": "Point", "id": 1, "args": None}}
    send = {"path": "Send", "id": 101, "args": None}
    freeze = {"path": "Freeze", "id": 102, "args": None}
    return {
        "format_version": format_version,
        "root": 0,
        "index": {
            "0": item("demo", {"module": {"is_crate": True, "items": [1, 3, 5, 7, 11], "is_stripped": False}}),
            "1": item("Point", {"struct": {"kind": {"plain": {"fields": [2], "has_stripped_fields": True}},
                                           "generics": NO_GENERICS, "impls": [8, 9, 12]}},
                      hidden_attrs=["non_exhaustive"]),
            "2": item("x", {"struct_field": {"primitive": "i32"}}),
            "3": item("origin", function([], point)),
            "5": item("geometry", {"module": {"is_crate": False, "items": [6], "is_stripped": False}}),
            "6": item(None, {"use": {"source": "crate::Point", "name": "Point", "id": 1, "is_glob": False}}),
            "7": item(None, {"use": {"source": "serde_json::Value", "name": "Value", "id": 100,
                                     "is_glob": False}}),
            "8": item(None, impl(None, [10]), visibility="default"),
            "9": item(None, impl(send, [], synthetic=True), visibility="default"),
            "10": item("norm", function(
                [["self", {"borrowed_ref": {"lifetime": None, "is_mutable": False, "type": {"generic": "Self"}}}],
                 ["scale", {"primitive": "f64"}]],
                {"primitive": "f64"})),
            "11": item("private_helper", function([], None), visibility="crate"),
            "12": item(None, impl(freeze, [], synthetic=True), visibility="default"),
        },
        "paths": {
            "1": {"crate_id": 0, "path": ["demo", "Point"], "kind": "struct"},
            "100": {"crate_id": 2, "path": ["serde_json", "value", "Value"], "kind": "enum"},
            "101": {"crate_id": 1, "path": ["core", "marker", "Send"], "kind": "trait"},
            "102": {"crate_id": 1, "path": ["core", "marker", "Freeze"], "kind": "trait"},
        },
    }


class ListingTests(unittest.TestCase):
    def test_items_are_listed_once_at_their_shortest_path(self) -> None:
        self.assertEqual(MODULE.Listing(crate()).render(), [
            "#[non_exhaustive] pub struct demo::Point",
            "pub demo::Point::x: i32",
            "impl demo::Point",
            "pub fn demo::Point::norm(&self, scale: f64) -> f64",
            "impl core::marker::Send for demo::Point",
            "pub use demo::Value = enum serde_json::value::Value",
            "pub mod demo::geometry",
            "pub use demo::geometry::Point = struct demo::Point",
            "pub fn demo::origin() -> demo::Point",
        ])

    def test_a_prelude_path_is_never_preferred(self) -> None:
        krate = crate()
        krate["index"]["5"]["name"] = "prelude"
        krate["index"]["0"]["inner"]["module"]["items"] = [5, 3]
        lines = MODULE.Listing(krate).render()
        self.assertIn("#[non_exhaustive] pub struct demo::prelude::Point", lines)
        krate["index"]["0"]["inner"]["module"]["items"] = [5, 3, 1]
        lines = MODULE.Listing(krate).render()
        self.assertIn("#[non_exhaustive] pub struct demo::Point", lines)
        self.assertIn("pub use demo::prelude::Point = struct demo::Point", lines)

    def test_another_rustdoc_format_is_refused(self) -> None:
        with self.assertRaises(SystemExit):
            MODULE.Listing(crate(MODULE.FORMAT_VERSION + 1))

    def test_the_docs_rs_features_select_the_build(self) -> None:
        package = {"name": "demo", "metadata": {"docs": {"rs": {
            "features": ["a", "b"], "rustdoc-args": ["--cfg", "docsrs"]}}}}
        command = MODULE.docs_rs_command(package)
        self.assertEqual(command[:2], ["cargo", f"+{MODULE.TOOLCHAIN}"])
        self.assertIn("a,b", command)
        self.assertEqual(command[command.index("--") + 1:][:2], ["--cfg", "docsrs"])
        package["metadata"]["docs"]["rs"] = {"all-features": True}
        self.assertIn("--all-features", MODULE.docs_rs_command(package))


if __name__ == "__main__":
    unittest.main()
