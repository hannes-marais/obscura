import base64
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from capture_chromium import reference_fonts


class ReferenceFontTests(unittest.TestCase):
    def test_absent_contract_does_not_change_reference_inputs(self):
        with tempfile.TemporaryDirectory() as root:
            self.assertEqual(reference_fonts((Path(root) / 'plain.html').as_uri()), [])
        self.assertEqual(reference_fonts('https://example.test/fixture.html'), [])

    def test_contract_preserves_exact_bytes_and_records_provenance(self):
        with tempfile.TemporaryDirectory(prefix='font fixture ') as root:
            root = Path(root)
            data = b'exact fixture font bytes'
            (root / 'font.ttf').write_bytes(data)
            (root / 'page.fonts.json').write_text(json.dumps({'fonts': [
                {'family': 'FixtureFace', 'file': 'font.ttf', 'selectors': ['input', 'textarea']}
            ]}))
            [font] = reference_fonts((root / 'page.html').as_uri())
            self.assertEqual(base64.b64decode(font['data_base64']), data)
            self.assertEqual(font['sha256'], hashlib.sha256(data).hexdigest())
            self.assertEqual(font['size'], len(data))
            self.assertEqual(font['selectors'], ['input', 'textarea'])

    def test_missing_font_and_empty_contract_fail_instead_of_falling_back(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            contract = root / 'page.fonts.json'
            contract.write_text(json.dumps({'fonts': []}))
            with self.assertRaises(ValueError):
                reference_fonts((root / 'page.html').as_uri())
            contract.write_text(json.dumps({'fonts': [
                {'family': 'FixtureFace', 'file': 'missing.ttf', 'selectors': ['input']}
            ]}))
            with self.assertRaises(FileNotFoundError):
                reference_fonts((root / 'page.html').as_uri())


if __name__ == '__main__':
    unittest.main()
