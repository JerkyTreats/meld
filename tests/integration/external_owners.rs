//! Select the actual package executable for ordinary CLI fixtures.

pub fn docs_config(config: &str) -> String {
    let mut config: toml::Value = toml::from_str(config).unwrap();
    let executable = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("meld-docs-owner");
    let bytes =
        std::fs::read(&executable).expect("build workspace binaries before integration tests");
    let selected = meld::runtime::owners::catalog::OwnerSelectionV1 {
        executable: meld::runtime::owners::OwnerExecutableV1 {
            path: executable,
            content_hash: blake3::hash(&bytes).to_hex().to_string(),
        },
        limits: meld::runtime::owners::OwnerConnectionLimitsV1 {
            request_timeout_ms: 30_000,
            max_message_bytes: 16 * 1024 * 1024,
        },
        binding_ids: ["workspace", "provider", "agent", "subject"]
            .into_iter()
            .map(String::from)
            .collect(),
    };
    let binding =
        meld::config::PhysicalBindingRef::ConfigRef(serde_json::to_string(&selected).unwrap());
    config["stewardship"]["declarations"]["docs"]
        .as_table_mut()
        .unwrap()
        .insert(
            "bindings".into(),
            toml::Value::try_from(std::collections::BTreeMap::from([("owner::docs", binding)]))
                .unwrap(),
        );
    toml::to_string(&config).unwrap()
}
