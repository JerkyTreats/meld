//! Native initialization through the real binary with isolated user state.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn invoke(root: &Path, args: &[&str]) -> Output {
    let workspace = root.join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_meld"));
    command
        .args(args)
        .current_dir(workspace)
        .env_clear()
        .env("HOME", root.join("home"));
    for name in ["config", "data", "state", "cache", "runtime"] {
        let key = if name == "runtime" {
            "XDG_RUNTIME_DIR".into()
        } else {
            format!("XDG_{}_HOME", name.to_uppercase())
        };
        command.env(key, root.join(name));
    }
    command.output().unwrap()
}

fn receipt(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn native_init_prepares_once_without_a_workspace_or_provider() {
    let root = TempDir::new().unwrap();
    let first = receipt(invoke(root.path(), &["init", "--json"]));
    let second = receipt(invoke(root.path(), &["init", "--json"]));
    let first_stages = first["stage_reports"].as_array().unwrap();
    let second_stages = second["stage_reports"].as_array().unwrap();
    assert_eq!(first_stages.len(), 4);
    assert_eq!(second_stages.len(), 4);
    for (first, second) in first_stages.iter().zip(second_stages) {
        assert_eq!(first["record_ids"], second["record_ids"]);
        assert_eq!(second["disposition"], "Unchanged");
    }
    let config = root.path().join("config/meld");
    assert!(!config.join("agents/reader.toml").exists());
    assert!(!config.join("prompts/code-analyzer.md").exists());
    assert_eq!(
        fs::read_dir(root.path().join("workspace")).unwrap().count(),
        0
    );
    let account = receipt(invoke(
        root.path(),
        &[
            "runtime",
            "startup-account",
            "--agent-id",
            "startup-agent",
            "--format",
            "json",
        ],
    ));
    let positions = account["positions"].as_array().unwrap();
    for name in ["product_compiled", "agent_genesis"] {
        assert_eq!(
            positions
                .iter()
                .find(|item| item["position"] == name)
                .unwrap()["state"],
            "available"
        );
    }
    assert!(
        account["generation_id"].is_null(),
        "init must not activate the product"
    );
}

#[test]
fn legacy_configuration_is_preserved_and_explicit_fresh_setup_works() {
    let root = TempDir::new().unwrap();
    let config = root.path().join("config/meld");
    fs::create_dir_all(&config).unwrap();
    let old = "[system.storage]\nproduct_root = '/old/product'\n";
    fs::write(config.join("config.toml"), old).unwrap();
    let result = invoke(root.path(), &["init", "--json"]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("--config"));
    assert_eq!(fs::read_to_string(config.join("config.toml")).unwrap(), old);
    assert!(!root.path().join("data").exists());
    let alternate = root.path().join("separate/config.toml");
    receipt(invoke(
        root.path(),
        &["--config", alternate.to_str().unwrap(), "init", "--json"],
    ));
    assert_eq!(fs::read_to_string(config.join("config.toml")).unwrap(), old);
}

#[test]
fn init_refuses_concurrent_preparation_and_resumes_after_lock_release() {
    use fs2::FileExt;
    let root = TempDir::new().unwrap();
    let config = root.path().join("config/meld");
    fs::create_dir_all(&config).unwrap();
    let lock = fs::File::create(config.join(".init.lock")).unwrap();
    FileExt::lock_exclusive(&lock).unwrap();
    let result = invoke(root.path(), &["init", "--json"]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("another initialization"));
    assert!(!config.join("config.toml").exists());
    drop(lock);
    receipt(invoke(root.path(), &["init", "--json"]));
}

#[test]
fn tampered_bundled_package_is_not_overwritten_or_accepted() {
    let root = TempDir::new().unwrap();
    receipt(invoke(root.path(), &["init", "--json"]));
    let package = fs::read_dir(root.path().join("data/meld/packages"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.is_dir())
        .unwrap();
    let manifest = package.join("pds-package.json");
    fs::write(&manifest, "tampered").unwrap();
    let result = invoke(root.path(), &["init", "--json"]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("content differs"));
    assert_eq!(fs::read_to_string(manifest).unwrap(), "tampered");
}
