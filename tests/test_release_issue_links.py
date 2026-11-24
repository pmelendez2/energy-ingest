"""Black-box regression tests for release-note issue references."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / 'scripts/check-release-notes.py'
PREFIX = 'https://github.com/pmelendez2/energy-ingest/issues/'
SECTIONS = ('Highlights', 'Fixes', 'Compatibility and upgrade notes',
            'Verification', 'Known limitations', 'Installation')


class TestReleaseIssueLinks(unittest.TestCase):
    def check_notes(self, section):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / 'release').mkdir()
            (root / 'release/VERSION').write_text('0.1.11\n')
            (root / 'docs/releases').mkdir(parents=True)
            content = '# Energy Ingest 0.1.11\n\n'
            content += '\n'.join(f'## {name}\n\nVerified.\n'
                                 for name in SECTIONS)
            content += '\n## Issues resolved\n\n' + section + '\n'
            (root / 'docs/releases/0.1.11.md').write_text(content)
            (root / 'scripts').mkdir()
            (root / 'scripts/check-release-notes.py').write_bytes(
                CHECKER.read_bytes())
            return subprocess.run(
                [sys.executable, str(root / 'scripts/check-release-notes.py')],
                capture_output=True, text=True, check=False)

    def test_valid_link(self):
        self.assertEqual(self.check_notes(
            f'- [#41 — Fix]({PREFIX}41): fixed').returncode, 0)

    def test_explicit_none(self):
        self.assertEqual(self.check_notes('None').returncode, 0)

    def test_bare_issue_rejected(self):
        self.assertNotEqual(self.check_notes('- Fixes #41').returncode, 0)

    def test_mismatch_rejected(self):
        self.assertNotEqual(self.check_notes(
            f'- [#41 — Fix]({PREFIX}42): fixed').returncode, 0)

    def test_duplicate_rejected(self):
        line = f'- [#41 — Fix]({PREFIX}41): fixed'
        self.assertNotEqual(self.check_notes(line + '\n' + line).returncode,
                            0)

    def test_none_with_link_rejected(self):
        self.assertNotEqual(self.check_notes(
            f'None\n- [#41]({PREFIX}41)').returncode, 0)

    def test_other_repository_rejected(self):
        self.assertNotEqual(self.check_notes(
            '- [#41](https://github.com/other/repo/issues/41)').returncode,
            0)


if __name__ == '__main__':
    unittest.main()
