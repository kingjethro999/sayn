import unittest
import sys
from pathlib import Path

# Add scripts directory to path
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))
from version import calculate_next, parse_version, format_version

class TestVersionRollover(unittest.TestCase):
    def test_patch_increment(self):
        self.assertEqual(calculate_next(0, 0, 1), "0.0.2")
        self.assertEqual(calculate_next(0, 0, 8), "0.0.9")
        self.assertEqual(calculate_next(0, 7, 4), "0.7.5")
        self.assertEqual(calculate_next(1, 0, 0), "1.0.1")

    def test_minor_rollover(self):
        self.assertEqual(calculate_next(0, 0, 9), "0.1.0")
        self.assertEqual(calculate_next(0, 1, 9), "0.2.0")
        self.assertEqual(calculate_next(0, 9, 9), "0.10.0")
        self.assertEqual(calculate_next(0, 25, 9), "0.26.0")
        self.assertEqual(calculate_next(0, 98, 9), "0.99.0")
        self.assertEqual(calculate_next(1, 0, 9), "1.1.0")
        self.assertEqual(calculate_next(1, 1, 9), "1.2.0")

    def test_major_rollover(self):
        self.assertEqual(calculate_next(0, 99, 9), "1.0.0")
        self.assertEqual(calculate_next(1, 99, 9), "2.0.0")
        self.assertEqual(calculate_next(2, 99, 9), "3.0.0")

    def test_invalid_versions_rejected(self):
        with self.assertRaises(ValueError):
            parse_version("0.0.10")
        with self.assertRaises(ValueError):
            parse_version("0.100.0")
        with self.assertRaises(ValueError):
            parse_version("1.0.10")
        with self.assertRaises(ValueError):
            parse_version("invalid")
        with self.assertRaises(ValueError):
            parse_version("v0.0.1")

if __name__ == "__main__":
    unittest.main()
