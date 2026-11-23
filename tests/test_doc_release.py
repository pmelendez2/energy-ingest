#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
import unittest
from importlib.machinery import SourceFileLoader
from pathlib import Path

script = Path(__file__).resolve().parents[1] / 'scripts/check-doc-release.py'
mod = SourceFileLoader('doc_release', str(script)).load_module()


class TestReleaseClaims(unittest.TestCase):
    def test_stale_current(self):
        self.assertEqual(len(mod.check_text('The current stable release is 0.1.8.', '0.1.9')), 1)

    def test_correct_current(self):
        self.assertFalse(mod.check_text('The current stable release is 0.1.9.', '0.1.9'))

    def test_wrapped_current(self):
        self.assertEqual(len(mod.check_text('Current stable release is\n0.1.8.', '0.1.9')), 1)

    def test_history_preserved(self):
        self.assertFalse(mod.check_text('## 0.1.8 — 2026-10-30', '0.1.9'))


if __name__ == '__main__':
    unittest.main()
