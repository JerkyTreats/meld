use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

use super::with_xdg_env;

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn runtime_status_reads_cache_while_foreground_host_owns_product_stores() {
    let temp = tempfile::TempDir::new().unwrap();
    with_xdg_env(&temp, || {
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        let binary = env!("CARGO_BIN_EXE_meld");
        let child = Command::new(binary)
            .args([
                "--quiet",
                "--workspace",
                workspace.to_str().unwrap(),
                "runtime",
                "run",
                "--instance-id",
                "process-isolation-owner",
                "--tick-ms",
                "10",
                "--duration-ms",
                "5000",
                "--format",
                "json",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut child = ChildGuard(child);
        let latest = wait_for_cache(&workspace, &mut child.0);
        assert!(latest.exists());

        let started = Instant::now();
        let output = Command::new(binary)
            .args([
                "--quiet",
                "--workspace",
                workspace.to_str().unwrap(),
                "runtime",
                "status",
                "--format",
                "json",
            ])
            .output()
            .unwrap();
        let elapsed = started.elapsed();

        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(elapsed < Duration::from_secs(2), "status took {elapsed:?}");
        assert!(child.0.try_wait().unwrap().is_none());
        let status: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(status["instance"]["instance_id"], "process-isolation-owner");
        assert!(status["cache_state"].get("fresh").is_some());
        assert!(status["runtimes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["runtime_id"] == "world_model.graph_replay"));
    });
}

fn wait_for_cache(workspace: &std::path::Path, child: &mut Child) -> std::path::PathBuf {
    let config = meld::config::ConfigLoader::load(workspace).unwrap();
    let product_root = config
        .system
        .storage
        .resolve_product_root(workspace)
        .unwrap();
    let latest =
        meld::runtime::contracts::RuntimeStatusCacheLayout::from_product_root(product_root).latest;
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if latest.exists() {
            return latest;
        }
        assert!(
            child.try_wait().unwrap().is_none(),
            "runtime host exited before publishing cache"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("runtime host did not publish cache before timeout");
}
