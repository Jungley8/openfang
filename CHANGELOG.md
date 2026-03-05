# Changelog

All notable changes to OpenFang will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
See [v0.1.0 changelog](https://github.com/RightNow-AI/openfang/blob/main/CHANGELOG.md#010---2026-02-24) for more details.

## [0.4.3] - 2026-03-05

### ⚙️ Miscellaneous Tasks

- *(release)* Update CHANGELOG for v0.4.2
- Merge 0.3.14~0.3.20  updates from the upstream main branch

---

## [0.4.2] - 2026-03-04

### 🐛 Bug Fixes

- *(gemini)* Use HARM_CATEGORY_* enum names for safety_settings

### ⚙️ Miscellaneous Tasks

- *(release)* Update CHANGELOG in release workflow and push to default branch

---

## [0.4.1] - 2026-03-04

### 🚀 Features

- Enhance /agent {name} command handling and Telegram integration

### 🐛 Bug Fixes

- *(clippy)* Needless_as_bytes and needless_borrows in telegram.rs

### 🚜 Refactor

- Enhance async tracing in kernel and workflow execution
- Improve async tracing in agent loop and tool runner

### ⚙️ Miscellaneous Tasks

- *(release)* Add pre-commit hooks, git-cliff config, and CI changelog automation
- Update pre-commit configuration and GitHub release workflow
- Merge v0.3.5~v0.3.10 updates from the upstream main branch
- Update release workflow
- *(openfang-cli)* Remove dead code from main, fix OPENFANG_HOME in init
- Merge v0.3.11~v0.3.13 updates from the upstream main branch

---

## [0.3.3] - 2026-03-03

### 🎨 Styling

- Fmt

---

## [0.3.2] - 2026-03-01

### 🚀 Features

- *(cli)* Add agent set model command
- *(telos)* Add TELOS report and preview endpoints, integrate snapshot management (#4)
- *(felos)* TELOS skills, report/preview endpoints, snapshot management (#5)

### 🐛 Bug Fixes

- *(auth)* Propagate API key in CLI and dashboard API calls to prevent 401
- *(cli)* Collapse redundant else in env check

### 💼 Other

- CLI 模块化重构 (#6)

### ⚙️ Miscellaneous Tasks

- *(branding)* Rename README brand to Octarq and add octopus logo
- *(release)* Update workflows
- Apply formatting and update config
- *(test)* Cache registry and target for workspace tests

---

