# External semantic owners

Docs and Dependency Security are separately selected executables. The Meld binary contains native runtime mechanisms and domain contracts. Each package owns its policy, observations and capability implementations, and publishes through the host's existing Events authority.

Build the example packages and host before running their guides or executable integration tests:

```sh
cargo build --workspace --bins
```

Add an owner selection to the corresponding declaration. Replace the executable path and digest below. `content_hash` is the lowercase BLAKE3 digest of the exact executable bytes, calculated with your BLAKE3 tool. The selection is a JSON value carried by a TOML `config_ref`.

```toml
[stewardship.declarations.docs.bindings."owner::docs"]
config_ref = '''{
  "executable": {
    "path": "/absolute/path/to/meld-docs-owner",
    "content_hash": "REPLACE_WITH_EXECUTABLE_BLAKE3"
  },
  "limits": {
    "request_timeout_ms": 180000,
    "max_message_bytes": 16777216
  },
  "binding_ids": ["workspace", "provider", "agent", "subject"]
}'''
```

For Security, use `owner::dependency-security`, select `meld-dependency-security-owner`, and grant `workspace`, `agent`, `subject`, `dependency-security.cargo` and `dependency-security.advisories`. The declaration must supply the Cargo and advisory references described in the [Security guide](../theory/dependency_security_mitigation/README.md). The native Code Change mechanism receives its own proposal binding; it is not a Security child callback.

These are explicit physical grants. A package cannot obtain an undeclared binding by requesting its name. Keep product state outside the target workspace. The host retains executable bytes by digest for exact predecessor recovery. Replacing the file at an operator path does not replace an already prepared implementation.

Install package revisions through `world init`. The exact transitive receipt closure must contain one authored `runtime.product-topology.v1` component. Its Agent positions, directives and participant plan determine preparation. The first host binds one Agent per declaration; use separate named declarations for independent assignments, including assignments addressing the same workspace. Select one with `--assignment NAME`. Embedded hosts can use `ProductRuntimeAssembly::load_assignment` to share their existing Event authority and Graph instance.

Each owner connection serializes native requests in a separate process. This does not provide network denial or whole-runtime process isolation. Preparation refuses those declarations, concurrency above one request and unsupported per-activation budget overrides. Native supervisor work budgets continue to apply.

Preparation does not start observation or admit Tasks. Native startup acquires its leases, drains and releases any exact predecessor, readies the successor and then opens admission. Shutdown signals let the outstanding owner reply finish before native drain. Process loss supplies no semantic completion or release receipt.

Legacy compiled owner state and legacy unscoped prepared heads currently produce an explicit incompatible-history refusal. Preserve those records. An importer and arbitrary Agent genesis migration are outside this first host contract. A separately assigned Agent can use a revised package in the same product root without substituting that revision into an existing assignment.
