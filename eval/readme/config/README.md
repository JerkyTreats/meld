# Local Eval Overrides

Local provider-request overrides live under `config/local/` and are intentionally gitignored.

Create `config/local/additional_json.local.json` for machine-specific or environment-specific request fields.

Example:

```json
{
  "max_tool_turns": 24
}
```

The eval harness always injects `lmserver_disable_auto_web_search: true` by default unless explicitly disabled.
