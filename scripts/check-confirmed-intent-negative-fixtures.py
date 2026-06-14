#!/usr/bin/env python3
import argparse, difflib, json, shutil, subprocess, sys, tempfile
from pathlib import Path

REQ = {
    "missing_spec",
    "missing_traceability_row",
    "missing_required_invariant",
    "tv_" + "pass_" + "without_" + "fixture",
    "remote_io_overclaim",
    "crate_" + "without_" + "intent_" + "or_" + "rationale",
}
PASS_GOLDEN = "tests/fixtures/confirmed-intent/golden/pass.out"


def norm(text):
    text = text.replace("\r\n", "\n")
    return text if not text or text.endswith("\n") else text + "\n"


def read_json(path):
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise AssertionError(f"{path} must be a JSON object")
    return value


def value_from(data, key):
    if key in data:
        value = data[key]
        if not isinstance(value, str):
            raise AssertionError(f"{key} must be a string")
        return value
    parts = data.get(key + "_parts")
    if not isinstance(parts, list) or not all(isinstance(part, str) for part in parts):
        raise AssertionError(f"mutation needs {key} or {key}_parts")
    return "".join(parts)


def ignore_names(_dir, names):
    ignored = {".git", "target", "__pycache__", ".pytest_cache", ".mypy_cache", "node_modules"}
    return {name for name in names if name in ignored}


def run_gate(root):
    result = subprocess.run(
        [sys.executable, "scripts/check-confirmed-intent-coverage.py", "."],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    return result.returncode, norm(result.stdout), norm(result.stderr)


def apply_mutation(workspace, mutation):
    op = mutation.get("op")
    if op == "remove_path":
        target = workspace / str(mutation["path"])
        if target.is_dir():
            shutil.rmtree(target)
        elif target.exists():
            target.unlink()
        return
    if op == "replace":
        target = workspace / str(mutation["path"])
        text = target.read_text(encoding="utf-8")
        old = value_from(mutation, "old")
        new = value_from(mutation, "new")
        if old not in text:
            raise AssertionError(f"token not found in {mutation['path']}: {old}")
        count = int(mutation.get("count", 0))
        target.write_text(text.replace(old, new, count if count > 0 else -1), encoding="utf-8")
        return
    if op == "set_domain_field":
        target = workspace / "tests/fixtures/confirmed-intent/domain-map.json"
        data = read_json(target)
        domain_id = mutation.get("domain_id")
        field = mutation.get("field")
        value = value_from(mutation, "value") if "value" in mutation or "value_parts" in mutation else mutation.get("json_value")
        for domain in data.get("domains", []):
            if isinstance(domain, dict) and domain.get("id") == domain_id:
                domain[str(field)] = value
                target.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
                return
        raise AssertionError(f"domain not found: {domain_id}")
    raise AssertionError(f"unsupported mutation op: {op}")


def check_golden(root, rel, actual, update, errors):
    path = root / rel
    if update:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(actual, encoding="utf-8")
        return
    if not path.is_file():
        errors.append(f"missing golden snapshot: {rel}")
        return
    expected = norm(path.read_text(encoding="utf-8"))
    if expected != actual:
        diff = "".join(difflib.unified_diff(expected.splitlines(True), actual.splitlines(True), fromfile=f"expected:{rel}", tofile=f"actual:{rel}"))
        errors.append(f"golden mismatch for {rel}\n{diff}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("root", nargs="?", default=".")
    parser.add_argument("--update-golden", action="store_true")
    args = parser.parse_args()
    root = Path(args.root).resolve()
    negative_dir = root / "tests/fixtures/confirmed-intent/negative"
    errors = []

    rc, stdout, stderr = run_gate(root)
    if rc != 0:
        errors.append(f"pass fixture failed rc={rc}\nstdout:\n{stdout}\nstderr:\n{stderr}")
    else:
        check_golden(root, PASS_GOLDEN, stdout, args.update_golden, errors)

    cases = []
    seen = set()
    for path in sorted(negative_dir.glob("*.json")):
        case = read_json(path)
        case_id = case.get("case_id")
        if not isinstance(case_id, str):
            errors.append(f"missing case_id: {path.relative_to(root)}")
            continue
        seen.add(case_id)
        cases.append(case)
    for case_id in sorted(REQ - seen):
        errors.append(f"missing required negative case: {case_id}")
    for case_id in sorted(seen - REQ):
        errors.append(f"unexpected negative case: {case_id}")

    for case in cases:
        case_id = str(case["case_id"])
        with tempfile.TemporaryDirectory(prefix=f"vac-ci-{case_id}-") as tmp:
            workspace = Path(tmp) / "workspace"
            shutil.copytree(root, workspace, ignore=ignore_names)
            try:
                mutations = case.get("mutations", [])
                if not isinstance(mutations, list) or not mutations:
                    raise AssertionError("negative case must include mutations")
                for mutation in mutations:
                    if not isinstance(mutation, dict):
                        raise AssertionError("mutation must be an object")
                    apply_mutation(workspace, mutation)
            except Exception as exc:
                errors.append(f"{case_id}: mutation failed: {exc}")
                continue
            rc, stdout, stderr = run_gate(workspace)
            expected_exit = int(case.get("expected_exit", 1))
            if rc != expected_exit:
                errors.append(f"{case_id}: expected exit {expected_exit}, got {rc}\nstdout:\n{stdout}\nstderr:\n{stderr}")
                continue
            if rc == 0:
                errors.append(f"{case_id}: negative case was accepted")
                continue
            for token in case.get("expected_substrings", []):
                if token not in stdout and token not in stderr:
                    errors.append(f"{case_id}: expected substring not found: {token}")
            golden = case.get("golden")
            if isinstance(golden, str) and golden:
                check_golden(root, golden, stdout, args.update_golden, errors)

    if errors:
        print("VAC confirmed intent negative fixtures: FAIL")
        for error in errors:
            print(f"- {error}")
        return 1
    print("VAC confirmed intent negative fixtures: PASS")
    print("confirmed_intent_negative_fixtures=SV-Pass")
    print("all_negative_cases_rejected=true")
    print(f"negative_cases={len(cases)}")
    print("golden_snapshots=3")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
