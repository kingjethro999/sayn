#!/usr/bin/env python3
"""
Sayn Version Management Utility
Enforces the custom version rollover rules specified in .versionrule:
- Format: MAJOR.MINOR.PATCH
- If PATCH < 9: PATCH += 1
- If PATCH == 9 and MINOR < 99: PATCH = 0, MINOR += 1
- If PATCH == 9 and MINOR == 99: PATCH = 0, MINOR = 0, MAJOR += 1
"""

import argparse
import re
import sys
from pathlib import Path

VERSION_PATTERN = re.compile(r"^(\d+)\.(\d+)\.(\d+)$")

def parse_version(v_str: str):
    v_str = v_str.strip()
    m = VERSION_PATTERN.match(v_str)
    if not m:
        raise ValueError(f"Invalid version format: '{v_str}'. Expected MAJOR.MINOR.PATCH")
    major, minor, patch = int(m.group(1)), int(m.group(2)), int(m.group(3))
    if patch > 9:
        raise ValueError(f"Invalid patch '{patch}' in '{v_str}': patch must be 0-9")
    if minor > 99:
        raise ValueError(f"Invalid minor '{minor}' in '{v_str}': minor must be 0-99")
    return major, minor, patch

def format_version(major: int, minor: int, patch: int) -> str:
    return f"{major}.{minor}.{patch}"

def calculate_next(major: int, minor: int, patch: int) -> str:
    if patch < 9:
        return format_version(major, minor, patch + 1)
    # patch == 9
    if minor < 99:
        return format_version(major, minor + 1, 0)
    # minor == 99 and patch == 9
    return format_version(major + 1, 0, 0)

def get_repo_root() -> Path:
    current = Path(__file__).resolve().parent
    if (current.parent / "Cargo.toml").exists():
        return current.parent
    return Path.cwd()

def read_current_version(repo_root: Path) -> str:
    version_file = repo_root / "VERSION"
    if not version_file.exists():
        # Fallback to Cargo.toml or 0.0.1
        cargo_file = repo_root / "Cargo.toml"
        if cargo_file.exists():
            content = cargo_file.read_text()
            m = re.search(r'\[workspace\.package\][^\[]*?version\s*=\s*"([^"]+)"', content, re.DOTALL)
            if m:
                return m.group(1).strip()
        return "0.0.1"
    return version_file.read_text().strip()

def sync_files(repo_root: Path, new_version: str, dry_run: bool = False):
    parse_version(new_version) # Validate before writing
    version_file = repo_root / "VERSION"
    cargo_file = repo_root / "Cargo.toml"

    print(f"Syncing version to {new_version}")

    if not dry_run:
        version_file.write_text(f"{new_version}\n")
        print(f"  Updated {version_file}")

        if cargo_file.exists():
            content = cargo_file.read_text()
            updated = re.sub(
                r'(\[workspace\.package\][^\[]*?version\s*=\s*)"[^"]+"',
                rf'\g<1>"{new_version}"',
                content,
                flags=re.DOTALL
            )
            cargo_file.write_text(updated)
            print(f"  Updated {cargo_file}")
    else:
        print("  [dry-run] Would update VERSION and Cargo.toml")

def check_consistency(repo_root: Path) -> bool:
    version_file = repo_root / "VERSION"
    cargo_file = repo_root / "Cargo.toml"

    if not version_file.exists():
        print("ERROR: VERSION file does not exist", file=sys.stderr)
        return False

    v_content = version_file.read_text().strip()
    try:
        parse_version(v_content)
    except ValueError as e:
        print(f"ERROR in VERSION file: {e}", file=sys.stderr)
        return False

    if cargo_file.exists():
        content = cargo_file.read_text()
        m = re.search(r'\[workspace\.package\][^\[]*?version\s*=\s*"([^"]+)"', content, re.DOTALL)
        if not m:
            print("ERROR: Could not find [workspace.package].version in Cargo.toml", file=sys.stderr)
            return False
        c_ver = m.group(1).strip()
        if c_ver != v_content:
            print(f"ERROR: VERSION ({v_content}) and Cargo.toml ({c_ver}) do not match", file=sys.stderr)
            return False

    print(f"Version consistency check passed: {v_content}")
    return True

def main():
    parser = argparse.ArgumentParser(description="Sayn Version Management")
    parser.add_argument("--current", action="store_true", help="Print current version")
    parser.add_argument("--next", action="store_true", help="Print next calculated version")
    parser.add_argument("--bump", action="store_true", help="Bump version and sync files")
    parser.add_argument("--check", action="store_true", help="Check version consistency")
    parser.add_argument("--dry-run", action="store_true", help="Dry run without writing files")
    parser.add_argument("--set-version", type=str, help="Manually set a specific valid version")

    args = parser.parse_args()
    repo_root = get_repo_root()

    cur_ver = read_current_version(repo_root)
    major, minor, patch = parse_version(cur_ver)
    next_ver = calculate_next(major, minor, patch)

    if args.current:
        print(cur_ver)
        return 0

    if args.next:
        print(next_ver)
        return 0

    if args.check:
        return 0 if check_consistency(repo_root) else 1

    if args.set_version:
        sync_files(repo_root, args.set_version, dry_run=args.dry_run)
        return 0

    if args.bump:
        sync_files(repo_root, next_ver, dry_run=args.dry_run)
        return 0

    # Default action: print status
    print(f"Current version: {cur_ver}")
    print(f"Next version:    {next_ver}")
    return 0

if __name__ == "__main__":
    sys.exit(main())
