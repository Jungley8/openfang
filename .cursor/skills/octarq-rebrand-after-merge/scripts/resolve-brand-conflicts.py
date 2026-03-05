#!/usr/bin/env python3
"""
Resolve Octarq brand conflicts after merging upstream OpenFang.

Usage:
    python .cursor/skills/octarq-rebrand-after-merge/scripts/resolve-brand-conflicts.py [--dry-run] [-v]
    # From repo root, after: git merge upstream/main  (or rebase)

Scans git conflict files (--diff-filter=U), applies brand rules, writes brand-merge-report.md.
Rust sources → checkout --theirs. Cargo.toml / tauri / docs / scripts → theirs + re-apply Octarq surface.
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import re
import subprocess
import sys
from pathlib import Path
from dataclasses import dataclass, field

# ── Rules (no external refs) ───────────────────────────────────────────────

REPO_OCTARQ = "Jungley8/Octarq"
DEFAULT_INSTALL_DIR = "~/.openfang/bin"
ENV_VARS_KEEP = ("OPENFANG_INSTALL_DIR", "OPENFANG_VERSION", "OPENFANG_HOME")

# Rust/internal: accept upstream as-is (no brand replacement)
THEIRS_GLOB_PATTERNS = [
    "crates/*/src/**/*.rs",
    "crates/*/tests/**/*.rs",
    "crates/*/benches/**/*.rs",
    "proto/**",
    "migrations/**",
]

# Word "openfang" only (not openfang-, openfang_, OPENFANG_)
DOC_SCRIPT_BRAND_PATTERN = re.compile(r"\bopenfang\b(?![-_])")

# Tauri keys we force to Octarq
TAURI_OCTARQ = {
    "productName": "Octarq",
    "identifier": "ai.octarq.desktop",
}


@dataclass
class ConflictFile:
    path: Path
    action: str = "pending"
    note: str = ""


@dataclass
class Report:
    auto_resolved: list[ConflictFile] = field(default_factory=list)
    needs_review: list[ConflictFile] = field(default_factory=list)
    errors: list[str] = field(default_factory=list)


def get_conflict_files(repo_root: Path) -> list[Path]:
    r = subprocess.run(
        ["git", "diff", "--name-only", "--diff-filter=U"],
        capture_output=True,
        text=True,
        cwd=repo_root,
    )
    if r.returncode != 0:
        print(f"[ERROR] git diff: {r.stderr}", file=sys.stderr)
        sys.exit(1)
    return [repo_root / f.strip() for f in r.stdout.splitlines() if f.strip()]


def rel(path: Path, repo_root: Path) -> str:
    return str(path.relative_to(repo_root))


def is_theirs(path: Path, repo_root: Path) -> bool:
    r = rel(path, repo_root)
    for pat in THEIRS_GLOB_PATTERNS:
        if fnmatch.fnmatch(r, pat):
            return True
    return False


def resolve_cargo_toml(path: Path, repo_root: Path, dry_run: bool) -> str | None:
    """Accept theirs, then force [[bin]].name to octarq / octarq-desktop."""
    if not path.name == "Cargo.toml":
        return None
    r = rel(path, repo_root)
    if not dry_run:
        subprocess.run(["git", "checkout", "--theirs", str(path)], check=True, cwd=repo_root)
    text = path.read_text(encoding="utf-8")

    if r == "crates/openfang-cli/Cargo.toml":
        # Only [[bin]].name: "openfang" -> "octarq". Package name stays openfang-cli.
        # Support split main: path might be src/main.rs or src/bin/octarq.rs etc.
        new_text = re.sub(
            r'(\s*name\s*=\s*)"openfang"(\s*(?:\n|$))',
            r'\1"octarq"\2',
            text,
            count=1,
        )
        if new_text != text and not dry_run:
            path.write_text(new_text, encoding="utf-8")
        return "auto_ours: set [[bin]].name = octarq" if new_text != text else "auto_theirs: bin name already octarq"

    if r == "crates/openfang-desktop/Cargo.toml":
        # Package name stays openfang-desktop; only [[bin]].name -> octarq-desktop (second occurrence).
        second = text.find('name = "openfang-desktop"')
        if second != -1:
            second = text.find('name = "openfang-desktop"', second + 1)  # second occurrence = bin
        if second == -1:
            second = text.find('name = "openfang-desktop"')  # only one: might be bin-only Cargo
        if second != -1:
            new_text = text[:second] + 'name = "octarq-desktop"' + text[second + len('name = "openfang-desktop"'):]
            if new_text != text and not dry_run:
                path.write_text(new_text, encoding="utf-8")
            return "auto_ours: set [[bin]].name = octarq-desktop"
        return "auto_theirs: no openfang-desktop bin name to replace"

    return None


def resolve_tauri(path: Path, repo_root: Path, dry_run: bool) -> str | None:
    if path.name != "tauri.conf.json":
        return None
    r = rel(path, repo_root)
    if "openfang-desktop" not in r:
        return None
    if not dry_run:
        subprocess.run(["git", "checkout", "--theirs", str(path)], check=True, cwd=repo_root)
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
        for k, v in TAURI_OCTARQ.items():
            if k in data:
                data[k] = v
        if not dry_run:
            path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        return "auto_ours: patched productName, identifier"
    except json.JSONDecodeError:
        return None


def resolve_doc_or_script(path: Path, repo_root: Path, dry_run: bool) -> tuple[str, str]:
    """Accept theirs, replace \\bopenfang\\b (not openfang- / openfang_ / OPENFANG_) with octarq."""
    if not dry_run:
        subprocess.run(["git", "checkout", "--theirs", str(path)], check=True, cwd=repo_root)
    text = path.read_text(encoding="utf-8")
    new_text = DOC_SCRIPT_BRAND_PATTERN.sub("octarq", text)
    changed = new_text != text
    if changed and not dry_run:
        path.write_text(new_text, encoding="utf-8")
    action = "auto_doc" if changed else "auto_theirs"
    note = "replaced command/product name" if changed else "no surface brand tokens"
    return action, note


def resolve_workflow_yml(path: Path, repo_root: Path, dry_run: bool) -> tuple[str, str] | None:
    """Accept theirs, then --bin openfang -> octarq, openfang-* artifact -> octarq-*, release name OpenFang -> Octarq."""
    if path.suffix.lower() not in (".yml", ".yaml") or ".github" not in str(path):
        return None
    if not dry_run:
        subprocess.run(["git", "checkout", "--theirs", str(path)], check=True, cwd=repo_root)
    text = path.read_text(encoding="utf-8")
    # Do not replace crates/openfang-desktop (path)
    t = text
    t = re.sub(r"--bin\s+openfang\b", "--bin octarq", t)
    t = re.sub(r"openfang-\$\{\{", "octarq-${{", t)
    t = re.sub(r'name:\s*openfang-', "name: octarq-", t)
    t = re.sub(r'"OpenFang\s+\$\{\{', '"Octarq ${{', t)
    t = re.sub(r"openfang\.sh", "octarq.sh", t)
    t = re.sub(r"rightnow-ai/openfang", "Jungley8/Octarq", t, flags=re.I)
    t = re.sub(r"RightNow-AI/openfang", "Jungley8/Octarq", t)
    if "openfang.exe" in t:
        t = t.replace("openfang.exe", "octarq.exe")
    if "release/openfang" in t:
        t = t.replace("release/openfang", "release/octarq")
    changed = t != text
    if changed and not dry_run:
        path.write_text(t, encoding="utf-8")
    return ("auto_doc", "workflow: octarq bin/artifacts/name/urls" if changed else "auto_theirs")


def resolve_dockerfile(path: Path, repo_root: Path, dry_run: bool) -> tuple[str, str] | None:
    if "Dockerfile" not in path.name:
        return None
    if not dry_run:
        subprocess.run(["git", "checkout", "--theirs", str(path)], check=True, cwd=repo_root)
    text = path.read_text(encoding="utf-8")
    t = text.replace("--bin openfang", "--bin octarq")
    t = t.replace("/release/openfang ", "/release/octarq ")
    t = t.replace('ENTRYPOINT ["openfang"]', 'ENTRYPOINT ["octarq"]')
    t = re.sub(r"COPY\s+.*/openfang\s+/", lambda m: m.group(0).replace("/openfang ", "/octarq ") if "/openfang " in m.group(0) else m.group(0), t)
    if "/opt/openfang/" in t:
        t = t.replace("/opt/openfang/", "/opt/octarq/")
    changed = t != text
    if changed and not dry_run:
        path.write_text(t, encoding="utf-8")
    return ("auto_doc", "Dockerfile: octarq bin/entrypoint" if changed else "auto_theirs")


def resolve_one(cf: ConflictFile, repo_root: Path, dry_run: bool, verbose: bool) -> ConflictFile:
    path, rel_path = cf.path, rel(cf.path, repo_root)

    if is_theirs(path, repo_root):
        if not dry_run:
            subprocess.run(["git", "checkout", "--theirs", str(path)], check=True, cwd=repo_root)
        cf.action = "auto_theirs"
        cf.note = "Rust/internal: accepted upstream"
        return cf

    if path.name == "Cargo.toml":
        result = resolve_cargo_toml(path, repo_root, dry_run)
        if result:
            cf.action = "auto_ours"
            cf.note = result
            if not dry_run:
                subprocess.run(["git", "add", str(path)], check=True, cwd=repo_root)
            return cf

    if path.name == "tauri.conf.json":
        result = resolve_tauri(path, repo_root, dry_run)
        if result:
            cf.action = "auto_ours"
            cf.note = result
            if not dry_run:
                subprocess.run(["git", "add", str(path)], check=True, cwd=repo_root)
            return cf

    if path.suffix.lower() in (".md", ".txt", ".sh", ".ps1", ".rst"):
        action, note = resolve_doc_or_script(path, repo_root, dry_run)
        cf.action = action
        cf.note = note
        if not dry_run:
            subprocess.run(["git", "add", str(path)], check=True, cwd=repo_root)
        return cf

    if path.suffix.lower() in (".yml", ".yaml") and ".github" in str(path):
        res = resolve_workflow_yml(path, repo_root, dry_run)
        if res:
            cf.action, cf.note = res
            if not dry_run:
                subprocess.run(["git", "add", str(path)], check=True, cwd=repo_root)
            return cf

    if "Dockerfile" in path.name:
        res = resolve_dockerfile(path, repo_root, dry_run)
        if res:
            cf.action, cf.note = res
            if not dry_run:
                subprocess.run(["git", "add", str(path)], check=True, cwd=repo_root)
            return cf

    cf.action = "manual"
    cf.note = f"no rule for {rel_path}"
    return cf


def write_report(report: Report, repo_root: Path, dry_run: bool) -> None:
    lines = ["# Brand Merge Report\n"]
    if dry_run:
        lines.append("**DRY RUN — no files modified**\n")
    lines.append(f"\n## Auto-resolved ({len(report.auto_resolved)})\n")
    for cf in report.auto_resolved:
        lines.append(f"- `[{cf.action}]` {cf.path.relative_to(repo_root)}: {cf.note}")
    lines.append(f"\n## Needs review ({len(report.needs_review)})\n")
    for cf in report.needs_review:
        lines.append(f"- `{cf.path.relative_to(repo_root)}`: {cf.note}")
    if report.errors:
        lines.append("\n## Errors\n")
        for e in report.errors:
            lines.append(f"- {e}")
    lines.append("\n---\n**Next**: fix manual items, then `cargo build --release --bin octarq`, then `git add . && git commit`.")
    out = "\n".join(lines) + "\n"
    print(out)
    if not dry_run:
        (repo_root / "brand-merge-report.md").write_text(out, encoding="utf-8")
        print("Report: brand-merge-report.md")


def main() -> None:
    ap = argparse.ArgumentParser(description="Resolve Octarq brand conflicts after upstream merge")
    ap.add_argument("--dry-run", action="store_true", help="Do not modify files")
    ap.add_argument("-v", "--verbose", action="store_true")
    ap.add_argument("--repo-root", default=".", help="Repo root (default: .)")
    args = ap.parse_args()
    repo_root = Path(args.repo_root).resolve()

    files = get_conflict_files(repo_root)
    if not files:
        print("No conflict files (--diff-filter=U). Nothing to do.")
        sys.exit(0)

    print(f"Found {len(files)} conflicted file(s).\n")
    report = Report()
    for p in files:
        cf = ConflictFile(path=p)
        try:
            cf = resolve_one(cf, repo_root, args.dry_run, args.verbose)
        except Exception as e:
            cf.action = "manual"
            cf.note = str(e)
            report.errors.append(str(e))
        if args.verbose:
            print(f"  [{cf.action}] {rel(p, repo_root)}: {cf.note}")
        if cf.action == "manual":
            report.needs_review.append(cf)
        else:
            report.auto_resolved.append(cf)

    write_report(report, repo_root, args.dry_run)
    sys.exit(1 if report.needs_review else 0)


if __name__ == "__main__":
    main()
