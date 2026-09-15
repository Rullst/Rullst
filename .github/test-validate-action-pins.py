#!/usr/bin/env python3
"""Regression tests for the repository's GitHub Action pinning policy."""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("validate-action-pins.py")
SPEC = importlib.util.spec_from_file_location("validate_action_pins", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot import {SCRIPT}")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


PIN = "0123456789abcdef0123456789abcdef01234567"


class ActionPinPolicyTests(unittest.TestCase):
    def assert_valid(self, source: str) -> None:
        self.assertEqual(MODULE.validate_text(source), [])

    def assert_invalid(self, source: str) -> None:
        self.assertTrue(MODULE.validate_text(source))

    def test_accepts_pinned_remote_and_local_actions(self) -> None:
        self.assert_valid(
            f"""jobs:
  build:
    uses: owner/repository/.github/workflows/check.yml@{PIN}
    steps:
      - uses: actions/checkout@{PIN} # reviewed version
      - uses: './.github/actions/local-check'
      - uses: \"owner/action@{PIN}\"
"""
        )

    def test_rejects_mutable_shorthand_reference(self) -> None:
        self.assert_invalid("steps:\n  - uses: owner/action@main\n")

    def test_accepts_coherent_codeql_sub_actions(self) -> None:
        self.assert_valid(
            f"steps:\n  - uses: github/codeql-action/init@{PIN}\n"
            f"  - uses: 'github/codeql-action/analyze@{PIN.upper()}'\n"
            f'  - uses: "github/codeql-action/upload-sarif@{PIN}"\n'
            f"  - uses: another/action@{'a' * 40}\n"
        )

    def test_rejects_mixed_codeql_sub_action_versions(self) -> None:
        self.assert_invalid(
            f"steps:\n  - uses: github/codeql-action/init@{PIN}\n"
            f"  - uses: github/codeql-action/analyze@{'f' * 40}\n"
        )

    def test_codeql_consistency_ignores_shell_text(self) -> None:
        self.assert_valid(
            f"steps:\n  - uses: github/codeql-action/init@{PIN}\n"
            "  - run: |\n"
            f"      uses: github/codeql-action/analyze@{'f' * 40}\n"
        )

    def test_rejects_sha_text_that_exists_only_in_comment(self) -> None:
        self.assert_invalid(f"steps:\n  - uses: owner/action@v1 # @{PIN}\n")

    def test_rejects_flow_mapping_and_quoted_key(self) -> None:
        self.assert_invalid(f"steps: [{{ uses: owner/action@{PIN} }}]\n")
        self.assert_invalid(f"steps:\n  - 'uses': owner/action@{PIN}\n")

    def test_rejects_escaped_quoted_mapping_keys(self) -> None:
        self.assert_invalid(
            r'''steps:
  - "u\x73es": owner/action@main
'''
        )
        self.assert_invalid(
            r'''steps:
  - "u\u0073es": owner/action@main
'''
        )
        self.assert_invalid(
            r'''jobs: {"u\x73es": owner/workflow@main}
'''
        )

    def test_rejects_explicit_merge_anchor_alias_and_tag_syntax(self) -> None:
        self.assert_invalid("steps:\n  ? uses\n  : owner/action@main\n")
        self.assert_invalid("defaults: &shared\n  run: bash\n")
        self.assert_invalid("defaults: *shared\n")
        self.assert_invalid("defaults: {<<: *shared}\n")
        self.assert_invalid("value: !!str uses\n")

    def test_rejects_multiline_or_dynamic_reference(self) -> None:
        self.assert_invalid(f"steps:\n  - uses: >\n      owner/action@{PIN}\n")
        self.assert_invalid("steps:\n  - uses: ${{ matrix.action }}\n")

    def test_ignores_uses_text_inside_run_block_and_comments(self) -> None:
        self.assert_valid(
            """steps:
  # uses: owner/action@main
  - name: inspect source
    run: |
      printf 'uses: owner/action@main\\n'
      grep -REn 'uses:' .github/workflows
"""
        )

    def test_current_repository_workflows_pass(self) -> None:
        repository = Path(__file__).resolve().parent.parent
        errors: list[str] = []
        for path in MODULE.workflow_files(repository / ".github/workflows"):
            errors.extend(MODULE.validate_text(path.read_text(encoding="utf-8"), str(path)))
        self.assertEqual(errors, [])


if __name__ == "__main__":
    unittest.main()
