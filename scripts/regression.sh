#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${REPO_ROOT}"

echo "[agentflow] running cargo fmt --check"
cargo fmt --all --check

echo "[agentflow] running cargo clippy"
cargo clippy --all-targets --all-features -- -D warnings

echo "[agentflow] running cargo test"
cargo test --all-targets --all-features

echo "[agentflow] regression suite completed successfully"

