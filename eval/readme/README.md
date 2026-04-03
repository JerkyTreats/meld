# README Eval Harness

This harness evaluates `meld` README generation quality with a local, reproducible loop.

## Layout

- `fixtures/<case_id>/input_fs/` - frozen input filesystem for a case.
- `fixtures/<case_id>/expected/README.md` - golden expected README.
- `fixtures/<case_id>/fixture_meta.yaml` - provenance and intent metadata.
- `rubrics/readme_quality.yaml` - scoring weights and pass thresholds.
- `scripts/run_case.py` - run one fixture through `meld` and capture output.
- `scripts/evaluate_suite.py` - run and score a fixture set, emit reports.
- `results/<run_id>/` - generated outputs and eval reports.

## Prerequisites

- `meld` CLI available on `PATH` (override with `--meld-bin`).
- A configured writer agent/provider pair that can execute README generation.
- Optional local runtime defaults in `eval/readme/config/local/run.local.json` (gitignored).

## Quickstart

Run one fixture:

`python3 eval/readme/scripts/run_case.py --case-id sample_nested --provider test-provider --agent docs-writer`

If `eval/readme/config/local/run.local.json` exists with `{ "provider": "local" }`, `--provider` can be omitted.

Tactical workflow/provider overrides for eval runs:

`python3 eval/readme/scripts/evaluate_suite.py --agent docs-writer --workflow-id docs_writer_thread_v1 --provider-model qwen3-coder-next --workflow-variant-dir eval/readme/variants/workflow_candidate`

These are passed as runtime flags to `meld context generate` and do not mutate XDG provider files.

Run suite + scoring:

`python3 eval/readme/scripts/evaluate_suite.py --provider test-provider --agent docs-writer`

Dry corpus validation (no model calls):

`python3 eval/readme/scripts/evaluate_suite.py --provider placeholder --skip-generate`

Provider preflight before full suite:

`python3 eval/readme/scripts/evaluate_suite.py --provider local --agent docs-writer --preflight-provider-test`

Set lmserver tool-turn cap for eval runs:

`python3 eval/readme/scripts/evaluate_suite.py --provider local --agent docs-writer --lmserver-max-tool-turns 24`

By default eval runs also inject:

`lmserver_disable_auto_web_search = true`

This keeps eval behavior stable and limits unwanted web-side variability.

Use local additional JSON overrides (gitignored) via:

`eval/readme/config/local/additional_json.local.json`

or pass an explicit file:

`python3 eval/readme/scripts/evaluate_suite.py --provider local --agent docs-writer --additional-json-file /path/to/overrides.json`

The harness writes a per-case runtime JSON file and passes it via:

`--provider-additional-json-file <path>`

on `meld context generate`.

## Workflow tuning loop

1. Edit external workflow/prompt variant files under `eval/readme/variants/`.
2. Run eval suite and inspect `eval/readme/results/<run_id>/report.md`.
3. Keep variant changes that improve score without regressions.
4. Promote a winning variant to runtime config only after repeated passes.

## Notes

- `run_case.py` executes from each fixture's `input_fs` directory and runs:
  - `meld scan --force`
  - `meld context generate ...`
  - `meld context get ... --format json`
- The first frame content returned by `context get` is treated as generated README text.
- Scoring uses:
  - Accuracy score (`score`) from similarity/heading coverage/length proximity.
  - Speed score from `generate_elapsed_ms` vs rubric target.
  - Utility score that rewards speed only when accuracy is above `optimization.accuracy_floor`.
- Initial `github/docs` corpus cases are:
  - `ghdocs_deployments`
  - `ghdocs_observability`
  - `ghdocs_links`
  - `ghdocs_events`
  - `ghdocs_rest_fixture`
- Golden files for these cases are currently seeded from upstream subtree `README.md` and should be curated over time.
