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

The runtime declares eight semantic participants. Shared CLI compatibility storage still exists under the product root. Confirmation can proceed from exact Graph-visible nonce evidence while Execution retains its outstanding return. Full successor-epoch completion and fairness under continuous retries remain under reconciliation work.
