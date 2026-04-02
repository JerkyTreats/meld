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

## Quickstart

Run one fixture:

`python3 eval/readme/scripts/run_case.py --case-id sample_nested --provider test-provider --agent docs-writer`

Run suite + scoring:

`python3 eval/readme/scripts/evaluate_suite.py --provider test-provider --agent docs-writer`

Dry corpus validation (no model calls):

`python3 eval/readme/scripts/evaluate_suite.py --provider placeholder --skip-generate`

Provider preflight before full suite:

`python3 eval/readme/scripts/evaluate_suite.py --provider local --agent docs-writer --preflight-provider-test`

Set lmserver tool-turn cap for eval runs:

`python3 eval/readme/scripts/evaluate_suite.py --provider local --agent docs-writer --lmserver-max-tool-turns 24`

This flag temporarily patches `~/.config/meld/providers/<provider>.toml` to add:

`[default_options.additional_json]`
`lmserver_max_tool_turns = <N>`

and restores the original provider file after each case run.

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
- Initial `github/docs` corpus cases are:
  - `ghdocs_deployments`
  - `ghdocs_observability`
  - `ghdocs_links`
  - `ghdocs_events`
  - `ghdocs_rest_fixture`
- Golden files for these cases are currently seeded from upstream subtree `README.md` and should be curated over time.
