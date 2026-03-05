# Octarq Rebrand — Quick Reference

Use with [SKILL.md](SKILL.md). Do not change README.md or docs/README.md unless the project explicitly brands them.

**After merge conflicts**: run `python .cursor/skills/octarq-rebrand-after-merge/scripts/resolve-brand-conflicts.py` from repo root (see SKILL.md §0).

## Replacements (surface only)

- In **docs** (excl. README): `OpenFang` → `Octarq`, ` openfang ` → ` octarq `, `` `openfang` `` → `` `octarq` ``, `` `openfang `` → `` `octarq ``.
- In **install scripts**: REPO → `Jungley8/Octarq`; keep `OPENFANG_INSTALL_DIR`, `OPENFANG_VERSION`, default `~/.openfang/bin` / `.openfang\bin`.
- In **workflows**: `--bin openfang` → `--bin octarq`, `openfang-${{ matrix.target }}` → `octarq-${{ matrix.target }}`, release name "OpenFang" → "Octarq", body URLs to Jungley8/Octarq / octarq.sh.
- In **Dockerfile**: `--bin openfang` → `--bin octarq`, `COPY .../openfang` → `COPY .../octarq`, `ENTRYPOINT ["openfang"]` → `ENTRYPOINT ["octarq"]`.
- In **tauri.conf.json**: productName "OpenFang" → "Octarq", identifier "ai.openfang.desktop" → "ai.octarq.desktop", longDescription and updater URL.
- In **JS SDK**: class `OpenFang` → `Octarq`, `OpenFangError` → `OctarqError`; package name `@openfang/sdk` → `@octarq/sdk`.
- In **API static**: `openfang-onboarded` → `octarq-onboarded`, `openfang-logs-` → `octarq-logs-`.

## Do not replace

- Crate names or paths: `openfang-*`, `openfang_*`.
- Type names: `OpenFangKernel`, `OpenFangError`, `OpenFangResult`.
- Env: `OPENFANG_HOME`, `OPENFANG_VAULT_KEY`, `OPENFANG_AGENT_ID`, `OPENFANG_MESSAGE`.
- Paths: `.openfang`, `openfang.db`, `openfang_user`, `openfang-sandbox`, `openfang-vault`, `openfang-skills` (org/config).

## Revert if present

- Any Rust use of `OctarqKernel` or `OctarqError` → `OpenFangKernel`, `OpenFangError` (kernel/types do not export Octarq*).
