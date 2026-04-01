#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <provider> [agent]"
  exit 2
fi

provider="$1"
agent="${2:-docs-writer}"
run_id="$(date -u +%Y%m%dT%H%M%SZ)"

python3 eval/readme/scripts/evaluate_suite.py \
  --provider "${provider}" \
  --agent "${agent}" \
  --run-id "${run_id}"

echo "wrote report: eval/readme/results/${run_id}/report.md"
