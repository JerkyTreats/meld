//! Integration tests for workspace commands: ignore list, scan, validate.
//!
//! Covers meld workspace ignore (list/add), meld scan (idempotency, force,
//! ignore list and .gitignore sync), and meld workspace validate (passed,
//! not scanned, JSON format).

use clap::Parser;
use meld::cli::{Cli, Commands, DangerCommands, RunContext, WorkspaceCommands};
use meld::ignore;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

use crate::integration::with_xdg_data_home;

fn expected_workspace_log_path(state_home: &Path, workspace: &Path) -> PathBuf {
    let canonical = workspace.canonicalize().unwrap();
    let mut base = state_home.join("meld");
    for component in canonical.components() {
        match component {
            std::path::Component::RootDir
            | std::path::Component::Prefix(_)
            | std::path::Component::CurDir
            | std::path::Component::ParentDir => {}
            std::path::Component::Normal(name) => {
                base = base.join(name);
            }
        }
    }
    base.join("meld.log")
}

fn expected_workspace_data_root(data_home: &Path, workspace: &Path) -> PathBuf {
    let canonical = workspace.canonicalize().unwrap();
    let mut base = data_home.join("meld");
    for component in canonical.components() {
        match component {
            std::path::Component::RootDir
            | std::path::Component::Prefix(_)
            | std::path::Component::CurDir
            | std::path::Component::ParentDir => {}
            std::path::Component::Normal(name) => {
                base = base.join(name);
            }
        }
    }
    base
}

#[test]
fn test_danger_flush_positional_path_parses() {
    let cli = Cli::try_parse_from(["meld", "danger", "flush", "./workspace", "--dry-run"]).unwrap();

    match cli.command {
        Commands::Danger {
            command:
                DangerCommands::Flush {
                    path,
                    path_positional,
                    dry_run,
                    yes,
                },
        } => {
            assert!(path.is_none());
            assert_eq!(path_positional, Some(PathBuf::from("./workspace")));
            assert!(dry_run);
            assert!(!yes);
        }
        _ => panic!("expected danger flush command"),
    }
}

#[test]
fn test_danger_flush_removes_workspace_state_but_keeps_logs() {
    let temp_dir = TempDir::new().unwrap();
    let state_home = temp_dir.path().join("state");
    let data_home = temp_dir.path().join("data");
    let config_home = temp_dir.path().join("config");
    let home = temp_dir.path().join("home");
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir_all(&state_home).unwrap();
    fs::create_dir_all(&data_home).unwrap();
    fs::create_dir_all(&config_home).unwrap();
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&workspace).unwrap();
    fs::write(workspace.join("a.txt"), "a").unwrap();

    let bin = env!("CARGO_BIN_EXE_meld");
    let status_output = Command::new(bin)
        .env("XDG_STATE_HOME", state_home.as_os_str())
        .env("XDG_DATA_HOME", data_home.as_os_str())
        .env("XDG_CONFIG_HOME", config_home.as_os_str())
        .env("HOME", home.as_os_str())
        .arg("--workspace")
        .arg(&workspace)
        .arg("status")
        .output()
        .unwrap();

    assert!(
        status_output.status.success(),
        "status should succeed: stderr={}",
        String::from_utf8_lossy(&status_output.stderr)
    );

    let workspace_data_root = expected_workspace_data_root(&data_home, &workspace);
    assert!(workspace_data_root.exists());
    let log_path = expected_workspace_log_path(&state_home, &workspace);
    assert!(log_path.exists());

    let dry_run_output = Command::new(bin)
        .env("XDG_STATE_HOME", state_home.as_os_str())
        .env("XDG_DATA_HOME", data_home.as_os_str())
        .env("XDG_CONFIG_HOME", config_home.as_os_str())
        .env("HOME", home.as_os_str())
        .arg("danger")
        .arg("flush")
        .arg("--path")
        .arg(&workspace)
        .arg("--dry-run")
        .output()
        .unwrap();

    assert!(
        dry_run_output.status.success(),
        "dry run should succeed: stderr={}",
        String::from_utf8_lossy(&dry_run_output.stderr)
    );
    assert!(String::from_utf8_lossy(&dry_run_output.stdout).contains("Would remove"));
    assert!(workspace_data_root.exists());
    assert!(log_path.exists());

    let flush_output = Command::new(bin)
        .env("XDG_STATE_HOME", state_home.as_os_str())
        .env("XDG_DATA_HOME", data_home.as_os_str())
        .env("XDG_CONFIG_HOME", config_home.as_os_str())
        .env("HOME", home.as_os_str())
        .arg("danger")
        .arg("flush")
        .arg("--path")
        .arg(&workspace)
        .arg("--yes")
        .output()
        .unwrap();

    assert!(
        flush_output.status.success(),
        "flush should succeed: stderr={}",
        String::from_utf8_lossy(&flush_output.stderr)
    );
    assert!(String::from_utf8_lossy(&flush_output.stdout).contains("Logs preserved"));
    assert!(!workspace_data_root.exists());
    assert!(log_path.exists());
}

#[test]
fn test_ignore_list_empty_missing_file() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_data_home(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        let ctx = RunContext::new(workspace_root.clone(), None).unwrap();
        let out = ctx
            .execute(&Commands::Workspace {
                command: WorkspaceCommands::Ignore {
                    path: None,
                    dry_run: false,
                    format: "text".to_string(),
                },
            })
            .unwrap();
        assert!(out.contains("empty") || out.contains("Ignore list"));
    });
}

#[test]
fn test_workspace_ignore_add_and_list() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_data_home(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        let sub = workspace_root.join("ignored_dir");
        fs::create_dir_all(&sub).unwrap();
        let ctx = RunContext::new(workspace_root.clone(), None).unwrap();

        let out = ctx
            .execute(&Commands::Workspace {
                command: WorkspaceCommands::Ignore {
                    path: Some(PathBuf::from("ignored_dir")),
                    dry_run: false,
                    format: "text".to_string(),
                },
            })
            .unwrap();
        assert!(out.contains("Added") && out.contains("ignored_dir"));

        let list_out = ctx
            .execute(&Commands::Workspace {
                command: WorkspaceCommands::Ignore {
                    path: None,
                    dry_run: false,
                    format: "text".to_string(),
                },
            })
            .unwrap();
        assert!(list_out.contains("ignored_dir"));
    });
}

#[test]
fn test_workspace_ignore_dry_run() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_data_home(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        let f = workspace_root.join("would_ignore.txt");
        fs::write(&f, "x").unwrap();
        let ctx = RunContext::new(workspace_root.clone(), None).unwrap();
        let out = ctx
            .execute(&Commands::Workspace {
                command: WorkspaceCommands::Ignore {
                    path: Some(PathBuf::from("would_ignore.txt")),
                    dry_run: true,
                    format: "text".to_string(),
                },
            })
            .unwrap();
        assert!(out.contains("Would add"));
        let list_path = ignore::ignore_list_path(&workspace_root).unwrap();
        assert!(
            !list_path.exists()
                || fs::read_to_string(&list_path)
                    .unwrap_or_default()
                    .is_empty()
        );
    });
}

#[test]
fn test_scan_then_validate_passed() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_data_home(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        fs::write(workspace_root.join("a.txt"), "a").unwrap();
        let ctx = RunContext::new(workspace_root.clone(), None).unwrap();
        ctx.execute(&Commands::Scan { force: true }).unwrap();
        let out = ctx
            .execute(&Commands::Workspace {
                command: WorkspaceCommands::Validate {
                    format: "text".to_string(),
                },
            })
            .unwrap();
        assert!(out.contains("Validation passed"));
        assert!(out.contains("All checks passed"));
    });
}

#[test]
fn test_validate_not_scanned_warning() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_data_home(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        fs::write(workspace_root.join("b.txt"), "b").unwrap();
        let ctx = RunContext::new(workspace_root.clone(), None).unwrap();
        let out = ctx
            .execute(&Commands::Workspace {
                command: WorkspaceCommands::Validate {
                    format: "text".to_string(),
                },
            })
            .unwrap();
        assert!(out.contains("Root node not found") || out.contains("not be scanned"));
    });
}

#[test]
fn test_validate_format_json() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_data_home(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        fs::write(workspace_root.join("c.txt"), "c").unwrap();
        let ctx = RunContext::new(workspace_root.clone(), None).unwrap();
        ctx.execute(&Commands::Scan { force: true }).unwrap();
        let out = ctx
            .execute(&Commands::Workspace {
                command: WorkspaceCommands::Validate {
                    format: "json".to_string(),
                },
            })
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(parsed
            .get("valid")
            .and_then(|v| v.as_bool())
            .unwrap_or(false));
        assert!(parsed.get("root_hash").is_some());
        assert!(parsed.get("node_count").is_some());
        assert!(parsed.get("frame_count").is_some());
        assert!(parsed.get("errors").unwrap().as_array().unwrap().is_empty());
    });
}

#[test]
fn test_scan_without_force_already_exists() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_data_home(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        fs::write(workspace_root.join("d.txt"), "d").unwrap();
        let ctx = RunContext::new(workspace_root.clone(), None).unwrap();
        ctx.execute(&Commands::Scan { force: true }).unwrap();
        let out = ctx.execute(&Commands::Scan { force: false }).unwrap();
        assert!(out.contains("already exists") && out.contains("--force"));
    });
}

#[test]
fn test_scan_with_force_repopulates() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_data_home(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        fs::write(workspace_root.join("e.txt"), "e").unwrap();
        let ctx = RunContext::new(workspace_root.clone(), None).unwrap();
        let out1 = ctx.execute(&Commands::Scan { force: true }).unwrap();
        fs::write(workspace_root.join("f.txt"), "f").unwrap();
        let out2 = ctx.execute(&Commands::Scan { force: true }).unwrap();
        assert!(out1.contains("Scanned"));
        assert!(out2.contains("Scanned"));
        assert!(out1 != out2 || out2.contains("nodes"));
    });
}

#[test]
fn test_workspace_ignore_path_outside_workspace_errors() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_data_home(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        let outside = temp_dir.path().join("other").join("path");
        fs::create_dir_all(&outside).unwrap();
        let ctx = RunContext::new(workspace_root.clone(), None).unwrap();
        let result = ctx.execute(&Commands::Workspace {
            command: WorkspaceCommands::Ignore {
                path: Some(outside),
                dry_run: false,
                format: "text".to_string(),
            },
        });
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("outside") || err.to_string().contains("Path"));
    });
}

#[test]
fn test_scan_default_uses_gitignore_when_ignore_list_missing() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_data_home(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        fs::write(workspace_root.join("keep.txt"), "keep").unwrap();
        fs::write(workspace_root.join(".gitignore"), "ignore_me\n").unwrap();
        fs::create_dir_all(workspace_root.join("ignore_me")).unwrap();
        fs::write(workspace_root.join("ignore_me").join("x"), "x").unwrap();
        let ctx = RunContext::new(workspace_root.clone(), None).unwrap();
        ctx.execute(&Commands::Scan { force: true }).unwrap();
        let records = ctx.api().node_store().list_all().unwrap();
        let paths: Vec<String> = records
            .iter()
            .map(|r| r.path.to_string_lossy().into_owned())
            .collect();
        assert!(paths.iter().any(|p| p.contains("keep")));
        assert!(!paths.iter().any(|p| p.contains("ignore_me")));
    });
}

#[test]
fn test_scan_syncs_gitignore_to_ignore_list() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_data_home(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        fs::write(workspace_root.join("a.txt"), "a").unwrap();
        fs::write(workspace_root.join(".gitignore"), "synced_ignore\n*.log\n").unwrap();
        fs::create_dir_all(workspace_root.join("synced_ignore")).unwrap();
        let ctx = RunContext::new(workspace_root.clone(), None).unwrap();
        ctx.execute(&Commands::Scan { force: true }).unwrap();
        let list_path = meld::ignore::ignore_list_path(&workspace_root).unwrap();
        let contents = fs::read_to_string(&list_path).unwrap();
        assert!(contents.contains("# .gitignore"));
        assert!(contents.contains("# end .gitignore"));
        assert!(contents.contains("synced_ignore"));
        assert!(contents.contains("*.log"));
        let records = ctx.api().node_store().list_all().unwrap();
        let paths: Vec<String> = records
            .iter()
            .map(|r| r.path.to_string_lossy().into_owned())
            .collect();
        assert!(!paths.iter().any(|p| p.contains("synced_ignore")));
    });
}

#[test]
fn test_scan_respects_ignore_list() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_data_home(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        fs::write(workspace_root.join("keep.txt"), "keep").unwrap();
        let skip_dir = workspace_root.join("skip_me");
        fs::create_dir_all(&skip_dir).unwrap();
        fs::write(skip_dir.join("file.txt"), "x").unwrap();
        let ctx = RunContext::new(workspace_root.clone(), None).unwrap();
        ctx.execute(&Commands::Workspace {
            command: WorkspaceCommands::Ignore {
                path: Some(PathBuf::from("skip_me")),
                dry_run: false,
                format: "text".to_string(),
            },
        })
        .unwrap();
        ctx.execute(&Commands::Scan { force: true }).unwrap();
        let records = ctx.api().node_store().list_all().unwrap();
        let paths: Vec<String> = records
            .iter()
            .map(|r| r.path.to_string_lossy().into_owned())
            .collect();
        assert!(paths.iter().any(|p| p.contains("keep")));
        assert!(!paths.iter().any(|p| p.contains("skip_me")));
    });
}

/// Regression: after `meld scan`, `meld status` must show the tree as scanned.
/// Guards against status using a different root computation (e.g. ignore config) than scan,
/// which would make the stored root not found and show "Scanned: no".
#[test]
fn test_scan_then_status_shows_scanned() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_data_home(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        fs::write(workspace_root.join("kept.txt"), "content").unwrap();
        fs::write(workspace_root.join(".gitignore"), "ignored\n").unwrap();
        fs::create_dir_all(workspace_root.join("ignored")).unwrap();
        fs::write(workspace_root.join("ignored").join("x"), "x").unwrap();

        let ctx = RunContext::new(workspace_root.clone(), None).unwrap();
        ctx.execute(&Commands::Scan { force: true }).unwrap();

        let out = ctx
            .execute(&Commands::Status {
                format: "text".to_string(),
                workspace_only: true,
                agents_only: false,
                providers_only: false,
                breakdown: false,
                test_connectivity: false,
            })
            .unwrap();
        assert!(
            out.contains("Scanned: yes"),
            "status must show tree as scanned after scan; got: {}",
            out
        );
        assert!(!out.contains("Scanned: no"));
    });
}
