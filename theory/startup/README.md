# Startup

This package asks an Agent to realize one nonce for its current admission epoch and confirm it through native Curation and Belief before separately satisfying its Goal. It uses the runtime subject `runtime/instance/meld` and requires no workspace or model provider.

Use a single Startup declaration in the global Meld configuration at `$XDG_CONFIG_HOME/meld/config.toml`. Replace the storage path with an absolute product directory. Existing workspace declarations remain supported; selecting multiple declarations for the same invocation is rejected as ambiguous.

```toml
[system.storage]
product_root = "/absolute/path/to/startup-product"

[stewardship.declarations.startup]
expression = "startup"
subject = { domain_id = "runtime", object_kind = "instance", object_id = "meld" }
agent_id = "startup-agent"
principal_id = "runtime-owner"

[stewardship.declarations.startup.theory]
belief_family_id = "startup_realization"
evidence_mapping_id = "startup_realization_v1"
curation_rule_id = "startup_realization"
maintained_condition_id = "startup_realization"
strategy_theory_id = "startup_realization"
authority_policy_id = "startup_nonce_local"
```

From the repository root, install and prepare the product, then run the ordinary supervisor:

```sh
cargo run --bin meld -- world init --theory-source theory/startup --format json
cargo run --bin meld -- runtime run --duration-ms 3000 --format json
cargo run --bin meld -- runtime status --format json
```

Initialization prepares the product without activating it. Runtime Run opens the shared admission gate and advances the native owners. The bounded CLI integration proof verifies planned confirmation, returned Belief, separate Goal satisfaction, and reopening. Runtime status reports lifecycle state; it is not a substitute for the Agent's Goal evidence.

The runtime declares eight semantic participants. Shared CLI compatibility storage still exists under the product root. Confirmation can proceed from exact Graph-visible nonce evidence while Execution retains its outstanding return. The production regressions cover confirmation during continuing callback retries, recovery of an unreturned effect, and completion of a distinct nonce under a successor epoch.

While the foreground runtime is running, inspect Startup from another terminal:

```sh
cargo run --bin meld -- runtime startup-account --agent-id startup-agent --format json
```

The account names the first missing position from native preparation through separate Goal satisfaction. A durable nonce with lagging Graph projection names `graph_visibility`; an outstanding Execution callback remains a separate obligation even after the nonce Goal is satisfied. Native owner identities, current generation and admission epoch, perspective, branch scope and inspection positions are returned together. This account makes no whole-runtime health claim and never advances an owner.

The command uses the live process's existing stores, or reads them offline when no process holds them. After shutdown, the closed generation and epoch remain visible as historical evidence. Use `--generation-id`, `--admission-epoch` and `--nonce-id` to address an exact retained instance. A successor epoch cannot borrow its predecessor's nonce. Supplying a returned `--inspection-fence` requires the same owner positions; changed positions produce a stale read and require fresh inspection. The fence detects change rather than reconstructing an earlier snapshot.

The same read contract is served at `POST /v1/projections/startup_nonce_account`. Its envelope contains `product_root` and an intact `request` with `agent_id` and optional generation, epoch, nonce and inspection-fence fields. The live and offline paths use the same read-only projection.
