#!/usr/bin/env bash
set -euo pipefail

# Sayn automated task commit script
# Enforces custom version rollover and commit standards

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

if [ $# -lt 1 ]; then
    echo "Usage: $0 <commit-message> [files...]"
    echo "Example: $0 'feat(cli): implement project commands'"
    exit 1
fi

COMMIT_MSG="$1"
shift

# Validate commit message format
if ! echo "${COMMIT_MSG}" | grep -qE '^(feat|fix|refactor|perf|docs|test|chore|ci|build|security)(\([a-zA-Z0-9_-]+\))?: .+'; then
    echo "ERROR: Commit message must follow conventional commits: type(scope): description"
    echo "Allowed types: feat, fix, refactor, perf, docs, test, chore, ci, build, security"
    exit 1
fi

export PATH="$HOME/.cargo/bin:$PATH"

# Run tests/checks if crates exist
if [ -d "${REPO_ROOT}/crates" ]; then
    echo "Running workspace tests..."
    cargo test --workspace --locked 2>/dev/null || cargo test --workspace
fi

# Bump version
echo "Advancing project version according to .versionrule..."
python3 "${REPO_ROOT}/scripts/version.py" --bump

NEW_VER="$(python3 "${REPO_ROOT}/scripts/version.py" --current)"

# If specific files provided, stage them plus version files; otherwise stage all tracked/modified
if [ $# -gt 0 ]; then
    git add "$@" "${REPO_ROOT}/VERSION" "${REPO_ROOT}/Cargo.toml"
    if [ -f "${REPO_ROOT}/Cargo.lock" ]; then
        git add "${REPO_ROOT}/Cargo.lock"
    fi
else
    git add -A
fi

# Create commit
git commit -m "${COMMIT_MSG}"

COMMIT_SHA="$(git rev-parse --short HEAD)"
echo "=========================================="
echo "✔ Successfully committed task"
echo "  Version:    v${NEW_VER}"
echo "  Commit SHA: ${COMMIT_SHA}"
echo "  Message:    ${COMMIT_MSG}"
echo "=========================================="
