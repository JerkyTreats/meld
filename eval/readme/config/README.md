# Local Eval Overrides

Local provider-request overrides live under `config/local/` and are intentionally gitignored.

Create `config/local/run.local.json` for local runtime selection defaults.

Example:

```json
{
  "provider": "local"
}
```

Create `config/local/additional_json.local.json` for machine-specific or environment-specific request fields.

Example:

```json
{
  "max_tool_turns": 24
}
```

The eval harness always injects `lmserver_disable_auto_web_search: true` by default unless explicitly disabled.

For tactical experiments, you can also pass runtime flags to `evaluate_suite.py`:

- `--provider-overwrite-file /path/to/provider.toml` (temporary provider replacement)
- `--workflow-variant-dir /path/to/workflows` (temporary workflow overlay into fixture `config/workflows`)

Both are restored after each case run.
