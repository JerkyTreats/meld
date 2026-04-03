#!/usr/bin/env python3
import argparse
import datetime as dt
import json
import pathlib
import shutil
import shlex
import subprocess
import sys
import time
from typing import Any


def run_cmd(command_template: str, tokens: dict, cwd: pathlib.Path) -> subprocess.CompletedProcess:
    command = command_template.format(**tokens)
    return subprocess.run(
        shlex.split(command),
        cwd=str(cwd),
        capture_output=True,
        text=True,
        check=False,
    )


def ensure_dir(path: pathlib.Path) -> None:
    path.mkdir(parents=True, exist_ok=True)


def load_json_file(path: pathlib.Path) -> dict[str, Any]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError("additional_json file must contain a top-level JSON object")
    return payload


def load_local_run_config(harness_root: pathlib.Path) -> dict[str, Any]:
    path = harness_root / "config" / "local" / "run.local.json"
    if not path.exists():
        return {}
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError("run.local.json must contain a top-level JSON object")
    return payload


def main() -> int:
    parser = argparse.ArgumentParser(description="Run one README eval fixture through meld.")
    parser.add_argument("--case-id", required=True, help="Fixture case id under eval/readme/fixtures/")
    parser.add_argument("--provider", default=None, help="Provider name used by meld context generate")
    parser.add_argument(
        "--provider-overwrite-file",
        default=None,
        help="Deprecated: provider overwrite now supported via meld runtime flags.",
    )
    parser.add_argument(
        "--workflow-variant-dir",
        default=None,
        help="Optional directory copied to fixture config/workflows for this run and then restored.",
    )
    parser.add_argument("--agent", default="docs-writer", help="Agent id used for generation")
    parser.add_argument("--workflow-id", default=None, help="Optional runtime workflow_id override")
    parser.add_argument("--provider-model", default=None, help="Optional runtime provider model override")
    parser.add_argument("--meld-bin", default="meld", help="Meld executable path")
    parser.add_argument("--run-id", default=dt.datetime.now(dt.UTC).strftime("%Y%m%dT%H%M%SZ"))
    parser.add_argument(
        "--lmserver-max-tool-turns",
        type=int,
        default=None,
        help="If set, temporarily injects default_options.additional_json.lmserver_max_tool_turns for the provider during this case run.",
    )
    parser.add_argument(
        "--additional-json-file",
        default=None,
        help="Optional JSON object file merged into provider default_options.additional_json for this case run.",
    )
    parser.set_defaults(disable_auto_web_search=True)
    search_group = parser.add_mutually_exclusive_group()
    search_group.add_argument(
        "--disable-auto-web-search",
        dest="disable_auto_web_search",
        action="store_true",
        help="Inject lmserver_disable_auto_web_search=true (default).",
    )
    search_group.add_argument(
        "--allow-auto-web-search",
        dest="disable_auto_web_search",
        action="store_false",
        help="Do not inject lmserver_disable_auto_web_search=true.",
    )
    parser.add_argument(
        "--harness-root",
        default="eval/readme",
        help="Harness root path relative to repository root",
    )
    parser.add_argument(
        "--scan-cmd-template",
        default="{meld_bin} scan --force",
        help="Template for scan command",
    )
    parser.add_argument(
        "--generate-cmd-template",
        default=(
            "{meld_bin} context generate --path . --agent {agent} --provider {provider} --force"
            "{workflow_flag}{provider_model_flag}{provider_additional_json_file_flag}"
        ),
        help="Template for generate command",
    )
    parser.add_argument(
        "--get-cmd-template",
        default="{meld_bin} context get --path . --agent {agent} --format json --max-frames 1 --ordering recency",
        help="Template for context get command",
    )
    args = parser.parse_args()

    repo_root = pathlib.Path.cwd()
    harness_root = (repo_root / args.harness_root).resolve()
    run_config = load_local_run_config(harness_root)
    provider = args.provider or run_config.get("provider")
    if not provider:
        print(
            "provider is required (pass --provider or set eval/readme/config/local/run.local.json)",
            file=sys.stderr,
        )
        return 2

    fixture_root = harness_root / "fixtures" / args.case_id
    input_fs = fixture_root / "input_fs"

    if not input_fs.exists():
        print(f"input_fs not found for case '{args.case_id}': {input_fs}", file=sys.stderr)
        return 2

    case_result_dir = harness_root / "results" / args.run_id / args.case_id
    ensure_dir(case_result_dir)

    tokens = {
        "meld_bin": args.meld_bin,
        "provider": provider,
        "agent": args.agent,
        "workflow_flag": f" --workflow-id {shlex.quote(args.workflow_id)}" if args.workflow_id else "",
        "provider_model_flag": f" --provider-model {shlex.quote(args.provider_model)}"
        if args.provider_model
        else "",
        "provider_additional_json_file_flag": "",
    }

    steps = [
        ("scan", args.scan_cmd_template),
        ("generate", args.generate_cmd_template),
        ("get", args.get_cmd_template),
    ]

    command_log = []
    get_stdout = ""
    workflow_target_dir = input_fs / "config" / "workflows"
    workflow_backup_dir = case_result_dir / "_backup_workflows"
    workflow_overlay_active = False
    runtime_additional_json_path = case_result_dir / "_runtime_additional_json.json"
    runtime_additional_json_used = False

    try:
        if args.workflow_variant_dir is not None:
            variant_dir = pathlib.Path(args.workflow_variant_dir)
            if not variant_dir.exists() or not variant_dir.is_dir():
                print(f"workflow variant dir not found or not a directory: {variant_dir}", file=sys.stderr)
                return 2
            if workflow_target_dir.exists():
                if workflow_backup_dir.exists():
                    shutil.rmtree(workflow_backup_dir)
                workflow_backup_dir.parent.mkdir(parents=True, exist_ok=True)
                shutil.copytree(workflow_target_dir, workflow_backup_dir)
                shutil.rmtree(workflow_target_dir)
            workflow_target_dir.parent.mkdir(parents=True, exist_ok=True)
            shutil.copytree(variant_dir, workflow_target_dir)
            workflow_overlay_active = True

        additional_json: dict[str, Any] = {}
        if args.disable_auto_web_search:
            additional_json["lmserver_disable_auto_web_search"] = True
        if args.lmserver_max_tool_turns is not None:
            additional_json["lmserver_max_tool_turns"] = args.lmserver_max_tool_turns

        additional_json_file = (
            pathlib.Path(args.additional_json_file)
            if args.additional_json_file
            else harness_root / "config" / "local" / "additional_json.local.json"
        )
        if additional_json_file.exists():
            additional_json.update(load_json_file(additional_json_file))
        elif args.additional_json_file:
            print(
                f"additional_json file not found: {additional_json_file}",
                file=sys.stderr,
            )
            return 2

        if additional_json:
            runtime_additional_json_path.write_text(
                json.dumps(additional_json, indent=2), encoding="utf-8"
            )
            runtime_additional_json_used = True
            tokens["provider_additional_json_file_flag"] = (
                f" --provider-additional-json-file {shlex.quote(str(runtime_additional_json_path))}"
            )

        for step_name, step_template in steps:
            started = time.perf_counter()
            result = run_cmd(step_template, tokens, input_fs)
            elapsed_ms = int((time.perf_counter() - started) * 1000)
            command_log.append(
                {
                    "step": step_name,
                    "command": step_template.format(**tokens),
                    "exit_code": result.returncode,
                    "elapsed_ms": elapsed_ms,
                    "stdout": result.stdout,
                    "stderr": result.stderr,
                }
            )
            if result.returncode != 0:
                (case_result_dir / "commands.json").write_text(
                    json.dumps(command_log, indent=2), encoding="utf-8"
                )
                failure = {
                    "case_id": args.case_id,
                    "status": "failed",
                    "failed_step": step_name,
                    "command": step_template.format(**tokens),
                    "stderr": result.stderr.strip(),
                }
                print(json.dumps(failure))
                print(f"{args.case_id}: step '{step_name}' failed", file=sys.stderr)
                return result.returncode
            if step_name == "get":
                get_stdout = result.stdout
    finally:
        if workflow_overlay_active:
            if workflow_target_dir.exists():
                shutil.rmtree(workflow_target_dir)
            if workflow_backup_dir.exists():
                workflow_target_dir.parent.mkdir(parents=True, exist_ok=True)
                shutil.copytree(workflow_backup_dir, workflow_target_dir)
                shutil.rmtree(workflow_backup_dir)
        if runtime_additional_json_path.exists():
            runtime_additional_json_path.unlink()

    parsed = json.loads(get_stdout)
    frames = parsed.get("frames", [])
    generated = frames[0].get("content", "") if frames else ""

    generated_path = case_result_dir / "generated_README.md"
    generated_path.write_text(generated, encoding="utf-8")
    (case_result_dir / "commands.json").write_text(json.dumps(command_log, indent=2), encoding="utf-8")

    run_meta = {
        "case_id": args.case_id,
        "provider": provider,
        "agent": args.agent,
        "workflow_id": args.workflow_id,
        "provider_model": args.provider_model,
        "provider_overwrite_file": args.provider_overwrite_file,
        "workflow_variant_dir": args.workflow_variant_dir,
        "run_id": args.run_id,
        "lmserver_max_tool_turns": args.lmserver_max_tool_turns,
        "disable_auto_web_search": args.disable_auto_web_search,
        "provider_additional_json_file": (
            str(runtime_additional_json_path) if runtime_additional_json_used else None
        ),
        "generated_path": str(generated_path),
        "frame_count": len(frames),
        "scan_elapsed_ms": next((s.get("elapsed_ms") for s in command_log if s.get("step") == "scan"), None),
        "generate_elapsed_ms": next(
            (s.get("elapsed_ms") for s in command_log if s.get("step") == "generate"), None
        ),
        "get_elapsed_ms": next((s.get("elapsed_ms") for s in command_log if s.get("step") == "get"), None),
        "total_elapsed_ms": sum(int(s.get("elapsed_ms", 0)) for s in command_log),
    }
    (case_result_dir / "run_meta.json").write_text(json.dumps(run_meta, indent=2), encoding="utf-8")
    print(json.dumps(run_meta))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
