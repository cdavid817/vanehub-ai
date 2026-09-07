#!/usr/bin/env python3
"""Check this handoff's structure without executing OpenSpec, CLIs, or network calls.

Python 3.10+, standard library only. This is NOT the official OpenSpec validator.
Reads files by default; --output optionally writes a JSON report to the given path.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys
from typing import Any
from urllib.parse import unquote, urlsplit


def read(path: Path, errors: list[str]) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as exc:
        errors.append(f"Cannot read {path.name}: {exc}")
        return ""


def parse_spec(path: Path, errors: list[str]) -> list[dict[str, Any]]:
    text = read(path, errors)
    capability = path.parent.name
    mode = ""
    requirements: list[dict[str, Any]] = []
    names: set[str] = set()
    for part in re.split(r"(?=^## (?:ADDED|MODIFIED|REMOVED|RENAMED) Requirements\s*$)", text, flags=re.M):
        match = re.match(r"## (ADDED|MODIFIED|REMOVED|RENAMED) Requirements", part)
        if not match:
            if "### Requirement:" in part:
                errors.append(f"{capability}: requirement has no delta section")
            continue
        mode = match.group(1)
        if mode not in {"ADDED", "MODIFIED"}:
            errors.append(f"{capability}: unexpected delta section {mode} in this package")
        chunks = re.split(r"(?=^### Requirement: )", part, flags=re.M)[1:]
        for chunk in chunks:
            title = chunk.splitlines()[0].removeprefix("### Requirement: ").strip()
            if title in names:
                errors.append(f"{capability}: duplicate requirement {title}")
            names.add(title)
            head = chunk.split("#### Scenario:", 1)[0]
            if not re.search(r"\b(?:SHALL|MUST)\b", head):
                errors.append(f"{capability}/{title}: no SHALL or MUST")
            scenario_chunks = re.split(r"(?=^#### Scenario: )", chunk, flags=re.M)[1:]
            scenario_names: list[str] = []
            for scenario in scenario_chunks:
                name = scenario.splitlines()[0].removeprefix("#### Scenario: ").strip()
                if name in scenario_names:
                    errors.append(f"{capability}/{title}: duplicate scenario {name}")
                scenario_names.append(name)
                if not re.search(r"\*\*WHEN\*\*", scenario) or not re.search(r"\*\*THEN\*\*", scenario):
                    errors.append(f"{capability}/{title}/{name}: missing WHEN or THEN")
            if not scenario_names:
                errors.append(f"{capability}/{title}: missing four-hash scenario")
            requirements.append({"capability": capability, "section": mode, "requirement": title, "scenarios": scenario_names})
    if not requirements:
        errors.append(f"{capability}: no requirements parsed")
    return requirements


def check_links(paths: list[Path], change: Path, errors: list[str]) -> int:
    checked = 0
    for path in paths:
        text = read(path, errors)
        # Only check real Markdown links, not examples inside fenced code blocks.
        text = re.sub(r"^```[^\n]*\n.*?^```\s*$", "", text, flags=re.M | re.S)
        for match in re.finditer(r"(?<!!)\[[^\]\n]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)", text):
            target = match.group(1).strip("<>")
            parts = urlsplit(target)
            if parts.scheme or parts.netloc or not parts.path:
                continue
            checked += 1
            dest = (path.parent / unquote(parts.path)).resolve()
            if not dest.exists():
                errors.append(f"Broken relative link in {path.relative_to(change) if path.is_relative_to(change) else path.name}: {target}")
    return checked


def run(change: Path) -> dict[str, Any]:
    errors: list[str] = []
    required = [
        ".openspec.yaml", "proposal.md", "design.md", "tasks.md", "provider-matrix.md",
        "acceptance.md", "implementation-notes.md", "references/official-sources.md",
        "references/repository-audit.md", "references/baseline-modified-scenarios.json",
        "verification/requirements-index.json", "verification/runbook.md",
        "verification/test-matrix.md", "verification/results.md", "verification/authoring-validation.md",
    ]
    for name in required:
        if not (change / name).is_file():
            errors.append(f"Missing required file: {name}")
    yaml = read(change / ".openspec.yaml", errors)
    if not re.search(r"^schema:\s*spec-driven\s*$", yaml, re.M):
        errors.append("Missing spec-driven schema")
    if not re.search(r"^created:\s*\d{4}-\d{2}-\d{2}\s*$", yaml, re.M):
        errors.append("Missing ISO created date")
    proposal = read(change / "proposal.md", errors)
    for heading in ("## Why", "## What Changes", "## Capabilities", "## Impact"):
        if heading not in proposal:
            errors.append(f"Missing proposal heading: {heading}")
    new_part = proposal.split("### New Capabilities", 1)[-1].split("### Modified Capabilities", 1)[0]
    modified_part = proposal.split("### Modified Capabilities", 1)[-1].split("## Impact", 1)[0]
    declared = set(re.findall(r"^- `([a-z0-9-]+)`:", new_part + "\n" + modified_part, re.M))
    specs = sorted((change / "specs").glob("*/spec.md"))
    found = {path.parent.name for path in specs}
    if declared != found:
        errors.append(f"Capabilities mismatch: declared_only={sorted(declared-found)}, specs_only={sorted(found-declared)}")
    parsed = [r for path in specs for r in parse_spec(path, errors)]
    try:
        expected = json.loads(read(change / "verification/requirements-index.json", errors))
        sort_key = lambda item: (item["capability"], item["section"], item["requirement"])
        if sorted(parsed, key=sort_key) != sorted(expected, key=sort_key):
            errors.append("requirements-index.json differs from actual specifications")
    except (ValueError, KeyError, TypeError) as exc:
        errors.append(f"Invalid requirements index: {exc}")
    preserved = 0
    try:
        baseline = json.loads(read(change / "references/baseline-modified-scenarios.json", errors))
        actual = {r["requirement"]: r for r in parsed if r["capability"] == "provider-plugin-sdk" and r["section"] == "MODIFIED"}
        for requirement, scenarios in baseline["requirements"].items():
            if requirement not in actual:
                errors.append(f"Missing baseline modified requirement: {requirement}")
                continue
            for scenario in scenarios:
                if scenario not in actual[requirement]["scenarios"]:
                    errors.append(f"Missing baseline scenario: {requirement}/{scenario}")
                else:
                    preserved += 1
    except (ValueError, KeyError, TypeError) as exc:
        errors.append(f"Invalid baseline scenario reference: {exc}")
    tasks_text = read(change / "tasks.md", errors)
    task_lines = [line for line in tasks_text.splitlines() if re.match(r"^- \[", line)]
    ids: list[str] = []
    checked_tasks = 0
    for line in task_lines:
        match = re.match(r"^- \[([ xX])\] (\d+\.\d+) \S", line)
        if not match:
            errors.append(f"Invalid checkbox task: {line[:100]}")
            continue
        ident = match.group(2)
        if ident in ids:
            errors.append(f"Duplicate task id: {ident}")
        ids.append(ident)
        checked_tasks += match.group(1).lower() == "x"
    if not ids:
        errors.append("No implementation tasks")
    per_section: dict[str, list[int]] = {}
    for ident in ids:
        section, num = ident.split(".")
        per_section.setdefault(section, []).append(int(num))
    for section, nums in per_section.items():
        if nums != list(range(1, len(nums) + 1)):
            errors.append(f"Non-contiguous task numbering in section {section}")
    md_paths = list(change.rglob("*.md"))
    # Also cover delivery-only entry points when this is an extracted bundle.
    delivery_root = change.parents[2]
    for name in ("VANEHUB_CLI_HANDOFF.md", "VANEHUB_CLI_CLAUDECODE_PROMPT.md"):
        if (delivery_root / name).is_file():
            md_paths.append(delivery_root / name)
    links = check_links(md_paths, change, errors)
    for path in md_paths:
        if read(path, errors).count("```") % 2:
            errors.append(f"Unbalanced fenced code in {path.name}")
    return {
        "validationKind": "structural-not-openspec-cli",
        "change": change.name,
        "result": "PASSED" if not errors else "FAILED",
        "counts": {
            "specFiles": len(specs), "requirements": len(parsed),
            "scenarios": sum(len(r["scenarios"]) for r in parsed),
            "implementationTasks": len(ids), "checkedTasks": checked_tasks,
            "baselineScenarioTitlesPreserved": preserved, "relativeLinksChecked": links,
        },
        "errors": errors,
        "limitations": [
            "Not the official OpenSpec validator; schema and semantic validation must run in the target repository.",
            "No implementation, executable, network, build, desktop or live-provider tests are performed.",
            "Baseline scenario preservation uses the supplied audit, not a fresh target-repository read.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--change-dir", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--output", type=Path, help="Optional JSON output path; otherwise only print")
    args = parser.parse_args()
    if not args.change_dir.is_dir():
        parser.error(f"Change directory does not exist: {args.change_dir}")
    result = run(args.change_dir.resolve())
    output = json.dumps(result, ensure_ascii=False, indent=2) + "\n"
    if args.output:
        try:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(output, encoding="utf-8")
        except OSError as exc:
            print(f"Cannot write report: {exc}", file=sys.stderr)
            return 2
    print(output, end="")
    return 0 if result["result"] == "PASSED" else 1


if __name__ == "__main__":
    sys.exit(main())
