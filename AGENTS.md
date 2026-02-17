# AGENTS.md

## Domain Architecture Rule

- Organize code by domain first.
- Keep each domain concern under `src/<domain>/`.
- Inside a domain, name submodules by behavior, for example `query`, `mutation`, `orchestration`, `queue`, `sessions`, `sinks`.
- Keep adapters thin. `tooling` and `api` may parse, route, format, and delegate only.
- Cross domain calls must use explicit domain contracts.
- Do not reach into another domain internal modules.
- Avoid generic primary folders named by technical layer.
- For migrations, use compatibility wrappers and require characterization and parity tests before removing old paths.
