#!/usr/bin/env python3
import argparse
import datetime as dt
import json
import os
import pathlib
import shlex
import subprocess
import sys
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


def provider_config_path(provider_name: str) -> pathlib.Path:
    xdg_config_home = os.environ.get("XDG_CONFIG_HOME")
    if xdg_config_home:
        base = pathlib.Path(xdg_config_home)
    else:
        base = pathlib.Path.home() / ".config"
    return base / "meld" / "providers" / f"{provider_name}.toml"


def toml_scalar(value: Any) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        return str(value)
    if isinstance(value, str):
        return json.dumps(value)
    raise ValueError(f"Unsupported additional_json value type: {type(value).__name__}")


def upsert_additional_json_toml(text: str, values: dict[str, Any]) -> str:
    lines = text.splitlines()
    section = "[default_options.additional_json]"

    section_start = None
    for idx, line in enumerate(lines):
        if line.strip() == section:
            section_start = idx
            break

    if section_start is None:
        if lines and lines[-1].strip() != "":
            lines.append("")
        lines.append(section)
        for key, value in sorted(values.items()):
            lines.append(f"{key} = {toml_scalar(value)}")
        return "\n".join(lines) + "\n"

    section_end = len(lines)
    for idx in range(section_start + 1, len(lines)):
        stripped = lines[idx].strip()
        if stripped.startswith("[") and stripped.endswith("]"):
            section_end = idx
            break

    for key, value in sorted(values.items()):
        target_line = f"{key} = {toml_scalar(value)}"
        replaced = False
        for idx in range(section_start + 1, section_end):
            stripped = lines[idx].strip()
            if stripped.startswith(f"{key} ") or stripped.startswith(f"{key}="):
                lines[idx] = target_line
                replaced = True
                break
        if not replaced:
            lines.insert(section_start + 1, target_line)
            section_end += 1
    return "\n".join(lines) + "\n"


def load_json_file(path: pathlib.Path) -> dict[str, Any]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError("additional_json file must contain a top-level JSON object")
    return payload


def main() -> int:
    parser = argparse.ArgumentParser(description="Run one README eval fixture through meld.")
    parser.add_argument("--case-id", required=True, help="Fixture case id under eval/readme/fixtures/")
    parser.add_argument("--provider", required=True, help="Provider name used by meld context generate")
    parser.add_argument("--agent", default="docs-writer", help="Agent id used for generation")
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
        default="{meld_bin} context generate --path . --agent {agent} --provider {provider} --force",
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
    fixture_root = harness_root / "fixtures" / args.case_id
    input_fs = fixture_root / "input_fs"

    if not input_fs.exists():
        print(f"input_fs not found for case '{args.case_id}': {input_fs}", file=sys.stderr)
        return 2

    case_result_dir = harness_root / "results" / args.run_id / args.case_id
    ensure_dir(case_result_dir)

    tokens = {
        "meld_bin": args.meld_bin,
        "provider": args.provider,
        "agent": args.agent,
    }

    steps = [
        ("scan", args.scan_cmd_template),
        ("generate", args.generate_cmd_template),
        ("get", args.get_cmd_template),
    ]

    command_log = []
    get_stdout = ""
    provider_cfg_path = provider_config_path(args.provider)
    original_provider_cfg_text = None
    provider_cfg_modified = False

    try:
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
            if not provider_cfg_path.exists():
                print(
                    f"provider config not found for '{args.provider}': {provider_cfg_path}",
                    file=sys.stderr,
                )
                return 2
            original_provider_cfg_text = provider_cfg_path.read_text(encoding="utf-8")
            updated = upsert_additional_json_toml(original_provider_cfg_text, additional_json)
            provider_cfg_path.write_text(updated, encoding="utf-8")
            provider_cfg_modified = True

        for step_name, step_template in steps:
            result = run_cmd(step_template, tokens, input_fs)
            command_log.append(
                {
                    "step": step_name,
                    "command": step_template.format(**tokens),
                    "exit_code": result.returncode,
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
        if provider_cfg_modified and original_provider_cfg_text is not None:
            provider_cfg_path.write_text(original_provider_cfg_text, encoding="utf-8")

    parsed = json.loads(get_stdout)
    frames = parsed.get("frames", [])
    generated = frames[0].get("content", "") if frames else ""

    generated_path = case_result_dir / "generated_README.md"
    generated_path.write_text(generated, encoding="utf-8")
    (case_result_dir / "commands.json").write_text(json.dumps(command_log, indent=2), encoding="utf-8")

    run_meta = {
        "case_id": args.case_id,
        "provider": args.provider,
        "agent": args.agent,
        "run_id": args.run_id,
        "lmserver_max_tool_turns": args.lmserver_max_tool_turns,
        "disable_auto_web_search": args.disable_auto_web_search,
        "generated_path": str(generated_path),
        "frame_count": len(frames),
    }
    (case_result_dir / "run_meta.json").write_text(json.dumps(run_meta, indent=2), encoding="utf-8")
    print(json.dumps(run_meta))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
