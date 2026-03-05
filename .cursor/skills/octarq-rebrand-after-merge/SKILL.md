---
name: octarq-rebrand-after-merge
description: Re-apply Octarq surface branding when merging upstream Openfang code into the Octarq fork. Use when merging from RightNow-AI/openfang, rebasing, or when new files from upstream need branding applied. Keeps internal names (crates, types, env, paths) unchanged for merge compatibility.
---

# Octarq Rebrand After Upstream Merge

When merging or rebasing from **RightNow-AI/openfang** into **Jungley8/Octarq**, re-apply only **surface** branding. Do **not** change crate names, type names, env vars, or default paths so that future merges stay easy.

## 0. Resolve conflicts with script (recommended)

After `git merge upstream/main` (or rebase) leaves conflicts:

```bash
# From repo root
python .cursor/skills/octarq-rebrand-after-merge/scripts/resolve-brand-conflicts.py --dry-run   # preview
python .cursor/skills/octarq-rebrand-after-merge/scripts/resolve-brand-conflicts.py           # apply
```

The script:

- **Rust** (`crates/*/src/**/*.rs`, tests, benches): `git checkout --theirs` (no brand change).
- **Cargo.toml**: Accept theirs, then force `[[bin]].name` to `octarq` (CLI) or `octarq-desktop` (desktop). Supports split CLI main (e.g. `path = "src/main.rs"` or `src/bin/octarq.rs`); only the bin name is replaced, package name stays `openfang-cli` / `openfang-desktop`.
- **tauri.conf.json**: Accept theirs, patch `productName` / `identifier` to Octarq.
- **Docs & scripts** (`.md`, `.sh`, `.ps1`, …): Accept theirs, replace `openfang` (word only, not `openfang-` / `OPENFANG_`) with `octarq`.
- **.github/workflows/*.yml**: Accept theirs, replace `--bin openfang`, artifact names `openfang-*`, release name, install/octarq URLs.
- **Dockerfile**: Accept theirs, replace `--bin octarq`, `COPY …/octarq`, `ENTRYPOINT ["octarq"]`.

Writes `brand-merge-report.md` with auto-resolved vs manual items. Exit code 1 if any file needs manual review.

## 1. Change to Octarq (user-facing)

| What | From | To |
|------|------|-----|
| CLI binary name | `openfang` | `octarq` |
| Desktop binary name | `openfang-desktop` | `octarq-desktop` |
| Product name in UI/docs | OpenFang | Octarq |
| Install script REPO | (upstream repo) | `Jungley8/Octarq` |
| Install default path | (if upstream uses other) | `~/.openfang/bin` (keep) |
| Install env vars | (if upstream added new) | Keep `OPENFANG_INSTALL_DIR`, `OPENFANG_VERSION` |
| Docs command examples | `openfang` | `octarq` |
| Tauri productName/identifier | OpenFang / ai.openfang.desktop | Octarq / ai.octarq.desktop |
| Release/install URLs | openfang.sh, rightnow-ai/openfang | octarq.sh (or project URL), Jungley8/Octarq |
| SDK package names | @openfang/sdk, openfang (PyPI) | @octarq/sdk, octarq |
| SDK class names (JS/TS) | OpenFang, OpenFangError | Octarq, OctarqError |
| API static keys | openfang-onboarded | octarq-onboarded |
| Log download filename prefix | openfang-logs- | octarq-logs- |
| systemd unit / Docker | ExecStart/ENTRYPOINT `openfang` | `octarq` |
| Release workflow | --bin openfang, artifact openfang-* | --bin octarq, artifact octarq-* |

**Files to touch (when they exist / are modified by upstream):**

- `crates/openfang-cli/Cargo.toml`: `[[bin]].name = "octarq"` (keep `path` as upstream, e.g. `src/main.rs` or split entry like `src/bin/octarq.rs`).
- `crates/openfang-desktop/Cargo.toml`: `[[bin]].name = "octarq-desktop"`, description "Octarq Agent OS"
- `crates/openfang-desktop/tauri.conf.json`: productName, identifier, longDescription, updater endpoints
- `crates/openfang-desktop/capabilities/default.json`, `gen/schemas/capabilities.json`: "Octarq" in description
- `scripts/install.sh`, `scripts/install.ps1`: REPO=`Jungley8/Octarq`, default `~/.openfang/bin`, env OPENFANG_*, binary name `octarq` in messages and paths
- `.github/workflows/release.yml`: --bin octarq, artifact names octarq-*, release name "Octarq", body URLs
- `.github/workflows/ci.yml`: install script URL (e.g. octarq.sh)
- `Dockerfile`, `Dockerfile.dist`: build --bin octarq, COPY octarq, ENTRYPOINT ["octarq"]; optional /opt/octarq for agents
- `deploy/openfang.service`: Description "Octarq...", ExecStart `/usr/local/bin/octarq`, User/Group octarq
- `openfang.toml.example`: first-line comment "Octarq Agent OS"
- `cliff.toml`: changelog header "Octarq"
- Docs under `docs/` (except `docs/README.md` if you keep it neutral): product name "Octarq", command `octarq`, install/URL to Jungley8/Octarq
- `README.md`: optional; if you brand it, use Octarq and project URLs
- `sdk/javascript/package.json`: name @octarq/sdk, description, repository; `index.js` / `index.d.ts`: Octarq, OctarqError
- `sdk/python/setup.py`: name octarq, description Octarq
- `crates/openfang-api/static/js/app.js`, `pages/wizard.js`, `pages/logs.js`: octarq-onboarded, octarq-logs-
- Test/restart argv in API tests: `"octarq".to_string()` for daemon restart command
- `crates/openfang-cli/src/launcher.rs`: comment "when `octarq` is run"

## 2. Do NOT change (keep for merge compatibility)

| What | Keep as |
|------|--------|
| Crate names | `openfang-*` |
| Crate directory names | `crates/openfang-*` |
| Rust type/struct names | `OpenFangKernel`, `OpenFangError`, `OpenFangResult` |
| KernelError variant | `OpenFang(OpenFangError)` |
| use / path in Rust | `openfang_kernel::OpenFangKernel`, etc. |
| Env vars | `OPENFANG_HOME`, `OPENFANG_VAULT_KEY`, `OPENFANG_AGENT_ID`, `OPENFANG_MESSAGE` |
| Default config/data path | `~/.openfang`, `~/.openfang/config.toml`, `openfang.db` |
| Channel/API field names | `openfang_user` |
| OAuth client id prefix | (optional keep) openfang-google-client-id etc. |
| Keyring / vault service string | openfang-vault (or keep for compatibility) |
| Container prefix | openfang-sandbox (or keep) |
| Internal repo URLs for upstream sync | RightNow-AI/openfang where needed for release fetch |

If upstream adds **new** crates, **new** env vars, or **new** paths, keep their names (openfang-* / OPENFANG_* / .openfang) so the next merge does not conflict.

## 3. Post-merge checklist

After merging or rebasing from upstream:

1. **Binary names**: `crates/openfang-cli/Cargo.toml` and `crates/openfang-desktop/Cargo.toml` have `[[bin]].name` = `octarq` and `octarq-desktop`.
2. **Scripts**: `scripts/install.sh` and `scripts/install.ps1` use REPO=`Jungley8/Octarq`, default dir `~/.openfang/bin`, env `OPENFANG_*`, and mention binary `octarq`.
3. **Workflows**: `.github/workflows/release.yml` builds `--bin octarq`, uploads `octarq-*` artifacts, release name "Octarq", body links to Jungley8/Octarq.
4. **Docs**: In `docs/` (and optionally README), product name Octarq, command `octarq`, no revert to OpenFang in user-facing copy.
5. **Rust types**: No `OctarqKernel` / `OctarqError` in code; keep `OpenFangKernel` and `OpenFangError`.
6. **Verify**: `cargo build --release --bin octarq` and, if needed, `--bin octarq-desktop`.

## 4. Conflict resolution

When git reports conflicts:

- **In Cargo.toml (binary name, deps)**: Prefer **theirs** for dependency and crate names; then set `[[bin]].name` to `octarq` / `octarq-desktop` in the CLI and desktop crates.
- **In install scripts**: Prefer **theirs** for logic; then re-apply REPO=`Jungley8/Octarq`, default path `~/.openfang/bin`, env `OPENFANG_*`, and `octarq` in messages.
- **In Rust code**: Prefer **theirs** for logic; ensure type names stay `OpenFangKernel` / `OpenFangError` (revert any upstream or local `OctarqKernel` back to `OpenFangKernel`).
- **In docs**: Prefer **theirs** for structure; then replace product name and command to Octarq / `octarq` in user-facing text.
