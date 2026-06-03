#!/usr/bin/env bash
# scripts/sp_descriptor_facade_drift.sh
#
# Descriptor<->facade drift audit for the chip-0057 silent-payments
# bindings. Exits 0 if no drift detected; non-zero with diagnostics if any
# method declared in bindings/silent_payments.json lacks a matching facade
# `pub fn` symbol, OR if any facade `pub fn` on the SP types lacks a JSON
# entry.
#
# Runtime: <5 seconds (pure jq + awk + comm).
# Dependencies: jq, awk, bash 4+.
#
# Usage: bash scripts/sp_descriptor_facade_drift.sh
# CI usage: add to a pre-merge gate; non-zero exit fails the gate.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
JSON="$REPO_ROOT/bindings/silent_payments.json"
RUST="$REPO_ROOT/crates/chia-sdk-bindings/src/silent_payments.rs"

if [[ ! -f "$JSON" ]]; then
  echo "ERROR: $JSON not found" >&2
  exit 2
fi
if [[ ! -f "$RUST" ]]; then
  echo "ERROR: $RUST not found" >&2
  exit 2
fi

# --- Extract method names from JSON ----------------------------------------
# For each top-level class, collect "ClassName::method_name" tuples.
# Skip enum entries (they have no methods).
JSON_METHODS=$(mktemp)
RUST_METHODS=$(mktemp)
trap 'rm -f "$JSON_METHODS" "$RUST_METHODS"' EXIT

jq -r '
  to_entries
  | map(select(.value.type == "class"))
  | .[]
  | .key as $class
  | (.value.methods // {}) | keys[] | "\($class)::\(.)"
' "$JSON" | sort -u > "$JSON_METHODS"

# --- Extract method names from Rust facade ---------------------------------
# Match `impl ClassName {` blocks + `pub fn name(` inside them.
# Tracks the most recent `impl X {` declaration via awk; `pub fn` lines are
# attributed to that impl block.
awk '
  /^impl [A-Z][A-Za-z0-9_]+ \{/ {
    match($0, /impl [A-Z][A-Za-z0-9_]+/);
    current = substr($0, RSTART+5, RLENGTH-5);
    next
  }
  /^[[:space:]]+pub fn [a-z_][a-zA-Z0-9_]*\(/ {
    if (current != "") {
      match($0, /pub fn [a-z_][a-zA-Z0-9_]*/);
      fname = substr($0, RSTART+7, RLENGTH-7);
      print current "::" fname
    }
  }
' "$RUST" | sort -u > "$RUST_METHODS"

# --- Diff ------------------------------------------------------------------
JSON_NOT_RUST=$(comm -23 "$JSON_METHODS" "$RUST_METHODS")
RUST_NOT_JSON=$(comm -13 "$JSON_METHODS" "$RUST_METHODS")

EXIT_CODE=0

if [[ -n "$JSON_NOT_RUST" ]]; then
  echo "DRIFT: methods in silent_payments.json with NO matching facade pub fn:" >&2
  echo "$JSON_NOT_RUST" | sed 's/^/  - /' >&2
  echo "" >&2
  EXIT_CODE=1
fi

if [[ -n "$RUST_NOT_JSON" ]]; then
  # Known-OK additions (From impls, internal helpers, etc.) would show up
  # here. For now, all facade `pub fn` methods inside impl blocks must have
  # JSON entries — the facade hosts only the bindy-exposed surface.
  echo "DRIFT: facade pub fn methods with NO matching silent_payments.json entry:" >&2
  echo "$RUST_NOT_JSON" | sed 's/^/  - /' >&2
  echo "" >&2
  EXIT_CODE=1
fi

if [[ $EXIT_CODE -eq 0 ]]; then
  COUNT=$(wc -l < "$JSON_METHODS")
  echo "No drift detected ($COUNT methods on both sides)."
fi

exit $EXIT_CODE
