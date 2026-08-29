#!/usr/bin/env bash
# Gate the Test Matrix against the suite it describes.
#
# The failure this exists for: `spec/tests.md` claimed ✅ on all 102 of its rows
# while `quire coverage` could back none of them, because the Test Case Summary
# table minted no `TC-NNN` id at all (agent-ix/quire-code-rs#8). Line coverage
# was 95.8% throughout, so no test-side signal could have caught it.
#
# Asserted here as a shape, never as a percentage: a ratio drifts down one row
# at a time and nobody notices, while "every row is either backed or explained
# by its own declared verification method" is a claim that breaks the moment it
# stops being true.
set -euo pipefail

QUIRE="${QUIRE:-quire}"
MODULES="${IX_FILAMENT_MODULES_PATH:-}"

if ! command -v "$QUIRE" >/dev/null 2>&1; then
  echo "check_coverage: '$QUIRE' is not on PATH." >&2
  echo "  Build it from agent-ix/quire-cli, or set QUIRE=/path/to/quire." >&2
  exit 127
fi

if [ -z "$MODULES" ]; then
  echo "check_coverage: IX_FILAMENT_MODULES_PATH is unset." >&2
  echo "  Without a module declaring a traceability model the run reports" >&2
  echo "  0/0 rows backed and passes — a false green, so this is an error." >&2
  exit 2
fi

report="$(mktemp)"
trap 'rm -f "$report"' EXIT
"$QUIRE" coverage --scope . --json >"$report" 2>/dev/null

python3 - "$report" <<'PY'
import json, sys

report = json.load(open(sys.argv[1]))
totals = report["totals"]
lies = report["status_lies"]
unbacked = report["unbacked_rows"]
explained = {(r["document"], r["row_id"]) for r in report["no_symbol_rows"]}

print(f"coverage: {totals['backed']}/{totals['total']} targets backed, "
      f"{len(unbacked)} unbacked row(s), {len(explained)} explained by method")

failed = False

if totals["total"] == 0:
    print("FAIL: nothing was reconciled — the matrix or the module path is wrong")
    failed = True

for row in lies:
    print(f"FAIL: {row['row_id']} claims a passing status and is unbacked "
          f"({row['document']})")
    failed = True

for row in unbacked:
    if (row["document"], row["row_id"]) not in explained:
        print(f"FAIL: {row['row_id']} is unbacked and its verification method "
              f"does not explain why ({row['document']})")
        failed = True

for d in report["diagnostics"]:
    # A declaration that matches nothing reports a confident zero, which is the
    # exact shape of the defect this gate exists for.
    if d["reason"] in {
        "id-column-matches-nothing",
        "archetype-matches-nothing",
        "section-holds-no-table",
        "marker-form-mismatch",
        "hollow-denominator",
    }:
        print(f"FAIL: {d['reason']}: {' '.join(d['message'].split())[:160]}")
        failed = True

sys.exit(1 if failed else 0)
PY
