#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from importlib.machinery import SourceFileLoader

ci = SourceFileLoader('ci_plan', str(Path(__file__).resolve().parents[1] / 'scripts' / 'ci-change-plan.py')).load_module()


class TestClassification(unittest.TestCase):
    def check(self, expected, *paths):
        self.assertEqual(ci.classify([('M', [p]) for p in paths]), expected)

    def test_docs(self):
        self.check('docs', 'README.md', 'docs/INSTALLATION.md')

    def test_release_workflow(self):
        self.check('workflow', '.github/workflows/docker-publish.yml')

    def test_app(self):
        self.check('full', 'crates/energy-store/src/gb_import.rs')

    def test_dependency(self):
        self.check('full', 'Cargo.lock')

    def test_schema(self):
        self.check('full', 'crates/energy-store/migrations/0009.sql')

    def test_build(self):
        self.check('full', 'docker/Dockerfile.prod')

    def test_scripts(self):
        self.check('full', 'scripts/package-prod-release.sh')

    def test_mixed(self):
        self.check('full', 'README.md', 'crates/gb/src/inspect.rs')

    def test_unknown(self):
        self.check('full', 'mystery.toml')

    def test_rename_both_paths(self):
        self.assertEqual(ci.classify(ci.parse_diff(b'R100\0docs/old.md\0docs/new.md\0')), 'docs')
        self.assertEqual(ci.classify(ci.parse_diff(b'R100\0docs/old.md\0Cargo.toml\0')), 'full')

    def test_deleted_docs(self):
        self.assertEqual(ci.classify(ci.parse_diff(b'D\0docs/old.md\0')), 'docs')

    def test_script_is_not_docs_only(self):
        self.check('full', 'scripts/check-doc-release.py')

    def test_workflow_mixed_with_docs_is_full(self):
        self.check('full', 'README.md', '.github/workflows/ci.yml')

    def test_unexpected_status_fails_closed(self):
        self.assertEqual(ci.classify([('Z', ['README.md'])]), 'full')

    def test_empty(self):
        self.assertEqual(ci.classify([]), 'full')

    def test_fail_closed(self):
        self.check('full', '../unexpected.md')


if __name__ == '__main__':
    unittest.main()
