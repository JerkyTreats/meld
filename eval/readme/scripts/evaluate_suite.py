#!/usr/bin/env python3
import argparse
import datetime as dt
import difflib
import json
import pathlib
import re
import subprocess
import sys
from typing import Dict, List, Tuple


HEADING_RE = re.compile(r"^\s{0,3}#{1,6}\s+(.+?)\s*$")


def parse_simple_yaml(path: pathlib.Path) -> Dict:
    # Minimal parser for this rubric file shape: nested maps with scalar values.
    root: Dict = {}
    stack: List[Tuple[int, Dict]] = [(-1, root)]
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.rstrip()
        if not line or line.lstrip().startswith("#"):
            continue
        indent = len(line) - len(line.lstrip(" "))
        key, sep, value = line.lstrip().partition(":")
        if sep == "":
            continue
        key = key.strip()
        value = value.strip()
        while stack and indent <= stack[-1][0]:
            stack.pop()
        parent = stack[-1][1]
        if value == "":
            parent[key] = {}
            stack.append((indent, parent[key]))
        else:
            lowered = value.lower()
            if lowered in {"true", "false"}:
                parsed = lowered == "true"
            else:
                try:
                    parsed = int(value) if value.isdigit() else float(value)
                except ValueError:
                    parsed = value.strip("'\"")
            parent[key] = parsed
    return root


def load_local_run_config(harness_root: pathlib.Path) -> Dict:
    path = harness_root / "config" / "local" / "run.local.json"
    if not path.exists():
        return {}
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError("run.local.json must contain a top-level JSON object")
    return payload


def normalize_markdown(text: str) -> str:
    lines = [line.rstrip() for line in text.replace("\r\n", "\n").split("\n")]
    compact = "\n".join(lines).strip()
    return compact


def extract_headings(text: str) -> List[str]:
    headings = []
    for line in text.splitlines():
        match = HEADING_RE.match(line)
        if match:
            headings.append(match.group(1).strip().lower())
    return headings


def evaluate_case(expected: str, actual: str, rubric: Dict) -> Dict:
    expected_n = normalize_markdown(expected)
    actual_n = normalize_markdown(actual)
    similarity = difflib.SequenceMatcher(None, expected_n, actual_n).ratio()

    expected_h = set(extract_headings(expected_n))
    actual_h = set(extract_headings(actual_n))
    heading_coverage = 1.0 if not expected_h else len(expected_h & actual_h) / len(expected_h)

    expected_len = max(1, len(expected_n))
    actual_len = len(actual_n)
    len_ratio = actual_len / expected_len
    length_proximity = max(0.0, 1.0 - abs(1.0 - len_ratio))

    weights = rubric.get("weights", {})
    score = (
        float(weights.get("similarity", 0.55)) * similarity
        + float(weights.get("heading_coverage", 0.30)) * heading_coverage
        + float(weights.get("length_proximity", 0.15)) * length_proximity
    )
    min_score = float(rubric.get("pass", {}).get("min_score", 0.72))
    min_heading_coverage = float(rubric.get("pass", {}).get("min_heading_coverage", 0.70))
    passed = score >= min_score and heading_coverage >= min_heading_coverage

    return {
        "passed": passed,
        "score": round(score, 4),
        "similarity": round(similarity, 4),
        "heading_coverage": round(heading_coverage, 4),
        "length_ratio": round(len_ratio, 4),
        "length_proximity": round(length_proximity, 4),
        "min_score": min_score,
        "min_heading_coverage": min_heading_coverage,
    }


def compute_speed_score(generate_elapsed_ms: int, rubric: Dict) -> float:
    optimization = rubric.get("optimization", {})
    speed_target_ms = float(optimization.get("speed_target_generate_ms", 30000))
    if generate_elapsed_ms <= 0:
        return 0.0
    score = speed_target_ms / float(generate_elapsed_ms)
    # Cap so very fast runs do not dominate utility.
    return max(0.0, min(1.0, score))


def compute_utility(accuracy_score: float, speed_score: float, rubric: Dict) -> float:
    optimization = rubric.get("optimization", {})
    accuracy_floor = float(
        optimization.get("accuracy_floor", rubric.get("pass", {}).get("min_score", 0.72))
    )
    if accuracy_score < accuracy_floor:
        return 0.0
    weights = optimization.get("utility_weights", {})
    accuracy_weight = float(weights.get("accuracy", 0.85))
    speed_weight = float(weights.get("speed", 0.15))
    total = accuracy_weight + speed_weight
    if total <= 0:
        return accuracy_score
    utility = (accuracy_weight * accuracy_score + speed_weight * speed_score) / total
    return max(0.0, min(1.0, utility))


def load_run_meta(run_dir: pathlib.Path, case_id: str) -> Dict:
    path = run_dir / case_id / "run_meta.json"
    if not path.exists():
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return {}


def parse_run_case_failure(stdout_text: str) -> Dict:
    try:
        payload = json.loads(stdout_text.strip())
        if isinstance(payload, dict) and payload.get("status") == "failed":
            return payload
    except (json.JSONDecodeError, TypeError):
        pass
    return {}


def build_provider_hint(provider: str, detail: str) -> str:
    text = detail.lower()
    if provider == "local" and "404" in text and "not found" in text:
        return (
            "Local provider likely needs a /v1 base endpoint. "
            "Try: meld provider edit local --endpoint https://api.chat.internal.jerkytreats.dev/v1"
        )
    return ""


def main() -> int:
    parser = argparse.ArgumentParser(description="Run and evaluate README fixture suite.")
    parser.add_argument("--provider", default=None)
    parser.add_argument(
        "--provider-overwrite-file",
        default=None,
        help="Optional provider TOML template to apply per case run and then restore.",
    )
    parser.add_argument(
        "--workflow-variant-dir",
        default=None,
        help="Optional directory copied into fixture config/workflows per case run and then restore.",
    )
    parser.add_argument("--agent", default="docs-writer")
    parser.add_argument("--workflow-id", default=None, help="Optional runtime workflow_id override")
    parser.add_argument("--provider-model", default=None, help="Optional runtime provider model override")
    parser.add_argument("--meld-bin", default="meld")
    parser.add_argument("--run-id", default=dt.datetime.now(dt.UTC).strftime("%Y%m%dT%H%M%SZ"))
    parser.add_argument(
        "--lmserver-max-tool-turns",
        type=int,
        default=None,
        help="If set, pass through to run_case to temporarily inject provider additional_json.lmserver_max_tool_turns.",
    )
    parser.add_argument(
        "--additional-json-file",
        default=None,
        help="Optional JSON object file merged into provider default_options.additional_json during eval runs.",
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
    parser.add_argument("--harness-root", default="eval/readme")
    parser.add_argument(
        "--preflight-provider-test",
        action="store_true",
        help="Run `meld provider test <provider>` before fixture execution.",
    )
    parser.add_argument(
        "--skip-generate",
        action="store_true",
        help="Skip meld execution and evaluate using existing generated files; falls back to expected README for dry checks.",
    )
    parser.add_argument("--case-id", action="append", dest="case_ids", help="Optional specific case id(s). Repeat for multiple.")
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

    fixtures_dir = harness_root / "fixtures"
    rubric_path = harness_root / "rubrics" / "readme_quality.yaml"
    rubric = parse_simple_yaml(rubric_path)

    if args.case_ids:
        case_ids = args.case_ids
    else:
        case_ids = sorted(
            p.name for p in fixtures_dir.iterdir() if p.is_dir() and (p / "input_fs").exists()
        )

    if not case_ids:
        print("No fixtures found to evaluate.", file=sys.stderr)
        return 2

    run_dir = harness_root / "results" / args.run_id
    run_dir.mkdir(parents=True, exist_ok=True)

    if args.preflight_provider_test and not args.skip_generate:
        preflight_cmd = [args.meld_bin, "provider", "test", provider, "--timeout", "10"]
        preflight = subprocess.run(
            preflight_cmd,
            cwd=str(repo_root),
            capture_output=True,
            text=True,
            check=False,
        )
        if preflight.returncode != 0:
            summary = {
                "run_id": args.run_id,
                "provider": provider,
                "agent": args.agent,
                "total_cases": 0,
                "passed_cases": 0,
                "failed_cases": 0,
                "avg_score": 0.0,
                "rubric_path": str(rubric_path),
                "error": "provider_preflight_failed",
                "failed_command": " ".join(preflight_cmd),
                "stderr": preflight.stderr,
                "stdout": preflight.stdout,
            }
            (run_dir / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
            (run_dir / "report.md").write_text(
                "# README Eval Report ({})\n\n- FAIL: provider preflight failed\n- Command: `{}`\n".format(
                    args.run_id, " ".join(preflight_cmd)
                ),
                encoding="utf-8",
            )
            print(json.dumps(summary))
            return 2

    case_results = []
    for case_id in case_ids:
        expected_path = fixtures_dir / case_id / "expected" / "README.md"
        generated_path = run_dir / case_id / "generated_README.md"
        if not args.skip_generate:
            run_case_cmd = [
                sys.executable,
                str(harness_root / "scripts" / "run_case.py"),
                "--case-id",
                case_id,
                "--provider",
                provider,
                "--agent",
                args.agent,
                "--meld-bin",
                args.meld_bin,
                "--run-id",
                args.run_id,
                "--harness-root",
                str(harness_root),
            ]
            if args.provider_overwrite_file is not None:
                run_case_cmd.extend(["--provider-overwrite-file", args.provider_overwrite_file])
            if args.workflow_variant_dir is not None:
                run_case_cmd.extend(["--workflow-variant-dir", args.workflow_variant_dir])
            if args.workflow_id is not None:
                run_case_cmd.extend(["--workflow-id", args.workflow_id])
            if args.provider_model is not None:
                run_case_cmd.extend(["--provider-model", args.provider_model])
            if args.lmserver_max_tool_turns is not None:
                run_case_cmd.extend(
                    [
                        "--lmserver-max-tool-turns",
                        str(args.lmserver_max_tool_turns),
                    ]
                )
            if args.additional_json_file is not None:
                run_case_cmd.extend(["--additional-json-file", args.additional_json_file])
            if args.disable_auto_web_search:
                run_case_cmd.append("--disable-auto-web-search")
            else:
                run_case_cmd.append("--allow-auto-web-search")
            proc = subprocess.run(
                run_case_cmd,
                cwd=str(repo_root),
                capture_output=True,
                text=True,
                check=False,
            )
            if proc.returncode != 0:
                failure_payload = parse_run_case_failure(proc.stdout)
                detail = failure_payload.get("stderr", "")
                hint = build_provider_hint(provider, detail)
                case_results.append(
                    {
                        "case_id": case_id,
                        "passed": False,
                        "error": "run_case_failed",
                        "failed_step": failure_payload.get("failed_step"),
                        "failed_command": failure_payload.get("command"),
                        "error_detail": detail,
                        "hint": hint,
                        "stdout": proc.stdout,
                        "stderr": proc.stderr,
                    }
                )
                continue

        expected = expected_path.read_text(encoding="utf-8") if expected_path.exists() else ""
        if generated_path.exists():
            actual = generated_path.read_text(encoding="utf-8")
        elif args.skip_generate:
            # Dry checks are useful while curating fixtures before first model run.
            actual = expected
        else:
            actual = ""
        metrics = evaluate_case(expected, actual, rubric)
        run_meta = load_run_meta(run_dir, case_id)
        generate_elapsed_ms = run_meta.get("generate_elapsed_ms")
        if isinstance(generate_elapsed_ms, int):
            speed_score = round(compute_speed_score(generate_elapsed_ms, rubric), 4)
            utility = round(compute_utility(float(metrics.get("score", 0.0)), speed_score, rubric), 4)
        else:
            speed_score = None
            utility = None
        metrics["case_id"] = case_id
        metrics["generate_elapsed_ms"] = generate_elapsed_ms
        metrics["total_elapsed_ms"] = run_meta.get("total_elapsed_ms")
        metrics["speed_score"] = speed_score
        metrics["utility"] = utility
        case_results.append(metrics)

    total = len(case_results)
    passed = sum(1 for item in case_results if item.get("passed"))
    failed = total - passed
    avg_score = round(
        sum(float(item.get("score", 0.0)) for item in case_results) / total if total else 0.0,
        4,
    )
    timed_cases = [item for item in case_results if isinstance(item.get("generate_elapsed_ms"), int)]
    avg_generate_ms = round(
        sum(int(item["generate_elapsed_ms"]) for item in timed_cases) / len(timed_cases), 2
    ) if timed_cases else None
    utility_cases = [item for item in case_results if isinstance(item.get("utility"), (float, int))]
    avg_utility = round(
        sum(float(item["utility"]) for item in utility_cases) / len(utility_cases), 4
    ) if utility_cases else None

    summary = {
        "run_id": args.run_id,
        "provider": provider,
        "provider_overwrite_file": args.provider_overwrite_file,
        "workflow_variant_dir": args.workflow_variant_dir,
        "workflow_id": args.workflow_id,
        "agent": args.agent,
        "provider_model": args.provider_model,
        "lmserver_max_tool_turns": args.lmserver_max_tool_turns,
        "disable_auto_web_search": args.disable_auto_web_search,
        "additional_json_file": args.additional_json_file,
        "total_cases": total,
        "passed_cases": passed,
        "failed_cases": failed,
        "avg_score": avg_score,
        "avg_generate_elapsed_ms": avg_generate_ms,
        "avg_utility": avg_utility,
        "rubric_path": str(rubric_path),
        "cases": case_results,
    }
    (run_dir / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")

    report_lines = [
        f"# README Eval Report ({args.run_id})",
        "",
        f"- Provider: `{provider}`",
        f"- Provider overwrite file: `{args.provider_overwrite_file}`",
        f"- Workflow variant dir: `{args.workflow_variant_dir}`",
        f"- Workflow id override: `{args.workflow_id}`",
        f"- Agent: `{args.agent}`",
        f"- Provider model override: `{args.provider_model}`",
        f"- lmserver_max_tool_turns: `{args.lmserver_max_tool_turns}`",
        f"- disable_auto_web_search: `{args.disable_auto_web_search}`",
        f"- additional_json_file: `{args.additional_json_file}`",
        f"- Cases: `{total}`",
        f"- Passed: `{passed}`",
        f"- Failed: `{failed}`",
        f"- Average score: `{avg_score}`",
        f"- Average generate ms: `{avg_generate_ms}`",
        f"- Average utility: `{avg_utility}`",
        "",
        "## Case Results",
        "",
    ]
    for item in case_results:
        case_id = item.get("case_id", "unknown")
        if item.get("error"):
            detail = item.get("error_detail") or item.get("stderr") or ""
            hint = item.get("hint") or ""
            if detail:
                if hint:
                    report_lines.append(
                        f"- `{case_id}`: FAIL (`{item['error']}`) - {detail} | Hint: {hint}"
                    )
                else:
                    report_lines.append(f"- `{case_id}`: FAIL (`{item['error']}`) - {detail}")
            else:
                report_lines.append(f"- `{case_id}`: FAIL (`{item['error']}`)")
        else:
            report_lines.append(
                "- `{}`: {} score={} utility={} speed_score={} gen_ms={} similarity={} heading_cov={}".format(
                    case_id,
                    "PASS" if item.get("passed") else "FAIL",
                    item.get("score"),
                    item.get("utility"),
                    item.get("speed_score"),
                    item.get("generate_elapsed_ms"),
                    item.get("similarity"),
                    item.get("heading_coverage"),
                )
            )

    (run_dir / "report.md").write_text("\n".join(report_lines) + "\n", encoding="utf-8")
    print(json.dumps(summary))
    return 0 if failed == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
