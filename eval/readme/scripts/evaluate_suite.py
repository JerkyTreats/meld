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
    parser.add_argument("--provider", required=True)
    parser.add_argument("--agent", default="docs-writer")
    parser.add_argument("--meld-bin", default="meld")
    parser.add_argument("--run-id", default=dt.datetime.now(dt.UTC).strftime("%Y%m%dT%H%M%SZ"))
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
        preflight_cmd = [args.meld_bin, "provider", "test", args.provider, "--timeout", "10"]
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
                "provider": args.provider,
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
                args.provider,
                "--agent",
                args.agent,
                "--meld-bin",
                args.meld_bin,
                "--run-id",
                args.run_id,
                "--harness-root",
                str(harness_root),
            ]
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
                hint = build_provider_hint(args.provider, detail)
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
        metrics["case_id"] = case_id
        case_results.append(metrics)

    total = len(case_results)
    passed = sum(1 for item in case_results if item.get("passed"))
    failed = total - passed
    avg_score = round(
        sum(float(item.get("score", 0.0)) for item in case_results) / total if total else 0.0,
        4,
    )

    summary = {
        "run_id": args.run_id,
        "provider": args.provider,
        "agent": args.agent,
        "total_cases": total,
        "passed_cases": passed,
        "failed_cases": failed,
        "avg_score": avg_score,
        "rubric_path": str(rubric_path),
        "cases": case_results,
    }
    (run_dir / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")

    report_lines = [
        f"# README Eval Report ({args.run_id})",
        "",
        f"- Provider: `{args.provider}`",
        f"- Agent: `{args.agent}`",
        f"- Cases: `{total}`",
        f"- Passed: `{passed}`",
        f"- Failed: `{failed}`",
        f"- Average score: `{avg_score}`",
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
                "- `{}`: {} score={} similarity={} heading_cov={}".format(
                    case_id,
                    "PASS" if item.get("passed") else "FAIL",
                    item.get("score"),
                    item.get("similarity"),
                    item.get("heading_coverage"),
                )
            )

    (run_dir / "report.md").write_text("\n".join(report_lines) + "\n", encoding="utf-8")
    print(json.dumps(summary))
    return 0 if failed == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
