use clap::{CommandFactory, Parser};
use merkle::tooling::cli::Cli;

use crate::phase1::support::assert_tokens_from_fixture;

#[test]
fn parse_valid_command_matrix() {
    let cases: Vec<Vec<&str>> = vec![
        vec!["merkle", "scan"],
        vec!["merkle", "workspace", "status"],
        vec!["merkle", "workspace", "validate"],
        vec!["merkle", "status", "--workspace-only", "--format", "json"],
        vec!["merkle", "validate"],
        vec!["merkle", "agent", "list"],
        vec!["merkle", "provider", "list"],
        vec![
            "merkle",
            "watch",
            "--debounce-ms",
            "120",
            "--batch-window-ms",
            "80",
        ],
        vec!["merkle", "init", "--list"],
        vec!["merkle", "context", "get", "--path", "./foo.txt"],
        vec!["merkle", "context", "generate", "--path", "./foo.txt"],
    ];

    for args in cases {
        let parsed = Cli::try_parse_from(args.clone());
        assert!(parsed.is_ok(), "expected valid parse for args: {args:?}");
    }
}

#[test]
fn parse_rejects_conflicting_context_targets() {
    let generate_conflict = Cli::try_parse_from([
        "merkle",
        "context",
        "generate",
        "--node",
        "deadbeef",
        "--path",
        "./foo.txt",
    ]);
    assert!(generate_conflict.is_err());

    let get_conflict = Cli::try_parse_from([
        "merkle",
        "context",
        "get",
        "--node",
        "deadbeef",
        "--path",
        "./foo.txt",
    ]);
    assert!(get_conflict.is_err());
}

#[test]
fn parse_rejects_removed_async_flag() {
    let parse_result = Cli::try_parse_from([
        "merkle",
        "context",
        "generate",
        "--path",
        "./foo.txt",
        "--async",
    ]);

    assert!(parse_result.is_err());
}

#[test]
fn top_level_help_tokens_match_snapshot_fixture() {
    let mut command = Cli::command();
    let mut output = Vec::new();
    command.write_long_help(&mut output).unwrap();
    let output = String::from_utf8(output).unwrap();

    assert_tokens_from_fixture(&output, "tests/fixtures/phase1/help/top_level.tokens");
}

#[test]
fn context_generate_help_tokens_match_snapshot_fixture() {
    let mut command = Cli::command();
    let context = command
        .find_subcommand_mut("context")
        .expect("context subcommand should exist");
    let generate = context
        .find_subcommand_mut("generate")
        .expect("context generate subcommand should exist");

    let mut output = Vec::new();
    generate.write_long_help(&mut output).unwrap();
    let output = String::from_utf8(output).unwrap();

    assert_tokens_from_fixture(
        &output,
        "tests/fixtures/phase1/help/context_generate.tokens",
    );
}
