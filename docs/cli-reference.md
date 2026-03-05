# Octarq CLI Reference

Complete command-line reference for `octarq`, the CLI tool for the Octarq Agent OS.

## Overview

The `octarq` binary is the primary interface for managing the Octarq Agent OS. It supports two modes of operation:

- **Daemon mode** -- When a daemon is running (`octarq start`), CLI commands communicate with it over HTTP. This is the recommended mode for production use.
- **In-process mode** -- When no daemon is detected, commands that support it will boot an ephemeral in-process kernel. Agents spawned in this mode are not persisted and will be lost when the process exits.

Running `octarq` with no subcommand launches the interactive TUI (terminal user interface) built with ratatui, which provides a full dashboard experience in the terminal.

## Installation

### From source (cargo)

```bash
cargo install --path crates/openfang-cli
```

### Build from workspace

```bash
cargo build --release -p openfang-cli
# Binary: target/release/octarq (or octarq.exe on Windows)
```

### Docker

```bash
docker run -it ghcr.io/jungley8/octarq:latest
```

### Shell installer

```bash
curl -fsSL https://octarq.jungley.net | sh
```

## Global Options

These options apply to all commands.

| Option            | Description                                                                    |
| ----------------- | ------------------------------------------------------------------------------ |
| `--config <PATH>` | Path to a custom config file. Overrides the default `~/.openfang/config.toml`. |
| `--help`          | Print help information for any command or subcommand.                          |
| `--version`       | Print the version of the `octarq` binary.                                      |

**Environment variables:**

| Variable              | Description                                                                            |
| --------------------- | -------------------------------------------------------------------------------------- |
| `RUST_LOG`            | Controls log verbosity (e.g. `info`, `debug`, `openfang_kernel=trace`).                |
| `OPENFANG_AGENTS_DIR` | Override the agent templates directory.                                                |
| `EDITOR` / `VISUAL`   | Editor used by `octarq config edit`. Falls back to `notepad` (Windows) or `vi` (Unix). |

---

## Command Reference

### octarq (no subcommand)

Launch the interactive TUI dashboard.

```
octarq [--config <PATH>]
```

The TUI provides a full-screen terminal interface with panels for agents, chat, workflows, channels, skills, settings, and more. Tracing output is redirected to `~/.openfang/tui.log` to avoid corrupting the terminal display.

Press `Ctrl+C` to exit. A second `Ctrl+C` force-exits the process.

---

### octarq init

Initialize the Octarq workspace. Creates `~/.openfang/` with subdirectories (`data/`, `agents/`) and a default `config.toml`.

```
octarq init [--quick]
```

**Options:**

| Option    | Description                                                                                                                    |
| --------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `--quick` | Skip interactive prompts. Auto-detects the best available LLM provider and writes config immediately. Suitable for CI/scripts. |

**Behavior:**

- Without `--quick`: Launches an interactive 5-step onboarding wizard (ratatui TUI) that walks through provider selection, API key configuration, and optionally starts the daemon.
- With `--quick`: Auto-detects providers by checking environment variables in priority order: Groq, Gemini, DeepSeek, Anthropic, OpenAI, OpenRouter. Falls back to Groq if none are found.
- File permissions are restricted to owner-only (`0600` for files, `0700` for directories) on Unix.

**Example:**

```bash
# Interactive setup
octarq init

# Non-interactive (CI/scripts)
export GROQ_API_KEY="gsk_..."
octarq init --quick
```

---

### octarq start

Start the Octarq daemon (kernel + API server).

```
octarq start [--config <PATH>]
```

**Behavior:**

- Checks if a daemon is already running; exits with an error if so.
- Boots the Octarq kernel (loads config, initializes SQLite database, loads agents, connects MCP servers, starts background tasks).
- Starts the HTTP API server on the address specified in `config.toml` (default: `127.0.0.1:4200`).
- Writes `daemon.json` to `~/.openfang/` so other CLI commands can discover the running daemon.
- Blocks until interrupted with `Ctrl+C`.

**Output:**

```
  Octarq Agent OS v0.1.0

  Starting daemon...

  [ok] Kernel booted (groq/llama-3.3-70b-versatile)
  [ok] 50 models available
  [ok] 3 agent(s) loaded

  API:        http://127.0.0.1:4200
  Dashboard:  http://127.0.0.1:4200/
  Provider:   groq
  Model:      llama-3.3-70b-versatile

  hint: Open the dashboard in your browser, or run `octarq chat`
  hint: Press Ctrl+C to stop the daemon
```

**Example:**

```bash
# Start with default config
octarq start

# Start with custom config
octarq start --config /path/to/config.toml
```

---

### octarq status

Show the current kernel/daemon status.

```
octarq status [--json]
```

**Options:**

| Option   | Description                                 |
| -------- | ------------------------------------------- |
| `--json` | Output machine-readable JSON for scripting. |

**Behavior:**

- If a daemon is running: queries `GET /api/status` and displays agent count, provider, model, uptime, API URL, data directory, and lists active agents.
- If no daemon is running: boots an in-process kernel and shows persisted state. Displays a warning that the daemon is not running.

**Example:**

```bash
octarq status

octarq status --json | jq '.agent_count'
```

---

### octarq doctor

Run diagnostic checks on the Octarq installation.

```
octarq doctor [--json] [--repair]
```

**Options:**

| Option     | Description                                                                                                                       |
| ---------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `--json`   | Output results as JSON for scripting.                                                                                             |
| `--repair` | Attempt to auto-fix issues (create missing directories, config, remove stale files). Prompts for confirmation before each repair. |

**Checks performed:**

1. **Octarq directory** -- `~/.openfang/` exists
2. **.env file** -- exists and has correct permissions (0600 on Unix)
3. **Config TOML syntax** -- `config.toml` parses without errors
4. **Daemon status** -- whether a daemon is running
5. **Port 4200 availability** -- if daemon is not running, checks if the port is free
6. **Stale daemon.json** -- leftover `daemon.json` from a crashed daemon
7. **Database file** -- SQLite magic bytes validation
8. **Disk space** -- warns if less than 100MB available (Unix only)
9. **Agent manifests** -- validates all `.toml` files in `~/.openfang/agents/`
10. **LLM provider keys** -- checks env vars for 10 providers (Groq, OpenRouter, Anthropic, OpenAI, DeepSeek, Gemini, Google, Together, Mistral, Fireworks), performs live validation (401/403 detection)
11. **Channel tokens** -- format validation for Telegram, Discord, Slack tokens
12. **Config consistency** -- checks that `api_key_env` references in config match actual environment variables
13. **Rust toolchain** -- `rustc --version`

**Example:**

```bash
octarq doctor

octarq doctor --repair

octarq doctor --json
```

---

### octarq dashboard

Open the web dashboard in the default browser.

```
octarq dashboard
```

**Behavior:**

- Requires a running daemon.
- Opens the daemon URL (e.g. `http://127.0.0.1:4200/`) in the system browser.
- Copies the URL to the system clipboard (uses PowerShell on Windows, `pbcopy` on macOS, `xclip`/`xsel` on Linux).

**Example:**

```bash
octarq dashboard
```

---

### octarq completion

Generate shell completion scripts.

```
octarq completion <SHELL>
```

**Arguments:**

| Argument  | Description                                                          |
| --------- | -------------------------------------------------------------------- |
| `<SHELL>` | Target shell. One of: `bash`, `zsh`, `fish`, `elvish`, `powershell`. |

**Example:**

```bash
# Bash
octarq completion bash > ~/.bash_completion.d/octarq

# Zsh
octarq completion zsh > ~/.zfunc/_octarq

# Fish
octarq completion fish > ~/.config/fish/completions/octarq.fish

# PowerShell
octarq completion powershell > octarq.ps1
```

---

## Agent Commands

### octarq agent new

Spawn an agent from a built-in template.

```
octarq agent new [<TEMPLATE>]
```

**Arguments:**

| Argument     | Description                                                                                                                          |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------ |
| `<TEMPLATE>` | Template name (e.g. `coder`, `assistant`, `researcher`). If omitted, displays an interactive picker listing all available templates. |

**Behavior:**

- Templates are discovered from: the repo `agents/` directory (dev builds), `~/.openfang/agents/` (installed), and `OPENFANG_AGENTS_DIR` (env override).
- Each template is a directory containing an `agent.toml` manifest.
- In daemon mode: sends `POST /api/agents` with the manifest. Agent is persistent.
- In standalone mode: boots an in-process kernel. Agent is ephemeral.

**Example:**

```bash
# Interactive picker
octarq agent new

# Spawn by name
octarq agent new coder

# Spawn the assistant template
octarq agent new assistant
```

---

### octarq agent spawn

Spawn an agent from a custom manifest file.

```
octarq agent spawn <MANIFEST>
```

**Arguments:**

| Argument     | Description                          |
| ------------ | ------------------------------------ |
| `<MANIFEST>` | Path to an agent manifest TOML file. |

**Behavior:**

- Reads and parses the TOML manifest file.
- In daemon mode: sends the raw TOML to `POST /api/agents`.
- In standalone mode: boots an in-process kernel and spawns the agent locally.

**Example:**

```bash
octarq agent spawn ./my-agent/agent.toml
```

---

### octarq agent list

List all running agents.

```
octarq agent list [--json]
```

**Options:**

| Option   | Description                         |
| -------- | ----------------------------------- |
| `--json` | Output as JSON array for scripting. |

**Output columns:** ID, NAME, STATE, PROVIDER, MODEL (daemon mode) or ID, NAME, STATE, CREATED (in-process mode).

**Example:**

```bash
octarq agent list

octarq agent list --json | jq '.[].name'
```

---

### octarq agent chat

Start an interactive chat session with a specific agent.

```
octarq agent chat <AGENT_ID>
```

**Arguments:**

| Argument     | Description                                  |
| ------------ | -------------------------------------------- |
| `<AGENT_ID>` | Agent UUID. Obtain from `octarq agent list`. |

**Behavior:**

- Opens a REPL-style chat loop.
- Type messages at the `you>` prompt.
- Agent responses display at the `agent>` prompt, followed by token usage and iteration count.
- Type `exit`, `quit`, or press `Ctrl+C` to end the session.

**Example:**

```bash
octarq agent chat a1b2c3d4-e5f6-7890-abcd-ef1234567890
```

---

### octarq agent kill

Terminate a running agent.

```
octarq agent kill <AGENT_ID>
```

**Arguments:**

| Argument     | Description              |
| ------------ | ------------------------ |
| `<AGENT_ID>` | Agent UUID to terminate. |

**Example:**

```bash
octarq agent kill a1b2c3d4-e5f6-7890-abcd-ef1234567890
```

---

## Workflow Commands

All workflow commands require a running daemon.

### octarq workflow list

List all registered workflows.

```
octarq workflow list
```

**Output columns:** ID, NAME, STEPS, CREATED.

---

### octarq workflow create

Create a workflow from a JSON definition file.

```
octarq workflow create <FILE>
```

**Arguments:**

| Argument | Description                                        |
| -------- | -------------------------------------------------- |
| `<FILE>` | Path to a JSON file describing the workflow steps. |

**Example:**

```bash
octarq workflow create ./my-workflow.json
```

---

### octarq workflow run

Execute a workflow by ID.

```
octarq workflow run <WORKFLOW_ID> <INPUT>
```

**Arguments:**

| Argument        | Description                                        |
| --------------- | -------------------------------------------------- |
| `<WORKFLOW_ID>` | Workflow UUID. Obtain from `octarq workflow list`. |
| `<INPUT>`       | Input text to pass to the workflow.                |

**Example:**

```bash
octarq workflow run abc123 "Analyze this code for security issues"
```

---

## Trigger Commands

All trigger commands require a running daemon.

### octarq trigger list

List all event triggers.

```
octarq trigger list [--agent-id <ID>]
```

**Options:**

| Option            | Description                                 |
| ----------------- | ------------------------------------------- |
| `--agent-id <ID>` | Filter triggers by the owning agent's UUID. |

**Output columns:** TRIGGER ID, AGENT ID, ENABLED, FIRES, PATTERN.

---

### octarq trigger create

Create an event trigger for an agent.

```
octarq trigger create <AGENT_ID> <PATTERN_JSON> [--prompt <TEMPLATE>] [--max-fires <N>]
```

**Arguments:**

| Argument         | Description                              |
| ---------------- | ---------------------------------------- |
| `<AGENT_ID>`     | UUID of the agent that owns the trigger. |
| `<PATTERN_JSON>` | Trigger pattern as a JSON string.        |

**Options:**

| Option                | Default              | Description                                                           |
| --------------------- | -------------------- | --------------------------------------------------------------------- |
| `--prompt <TEMPLATE>` | `"Event: {{event}}"` | Prompt template. Use `{{event}}` as a placeholder for the event data. |
| `--max-fires <N>`     | `0` (unlimited)      | Maximum number of times the trigger will fire.                        |

**Pattern examples:**

```bash
# Fire on any lifecycle event
octarq trigger create <AGENT_ID> '{"lifecycle":{}}'

# Fire when a specific agent is spawned
octarq trigger create <AGENT_ID> '{"agent_spawned":{"name_pattern":"*"}}'

# Fire on agent termination
octarq trigger create <AGENT_ID> '{"agent_terminated":{}}'

# Fire on all events (limited to 10 fires)
octarq trigger create <AGENT_ID> '{"all":{}}' --max-fires 10
```

---

### octarq trigger delete

Delete a trigger by ID.

```
octarq trigger delete <TRIGGER_ID>
```

**Arguments:**

| Argument       | Description                    |
| -------------- | ------------------------------ |
| `<TRIGGER_ID>` | UUID of the trigger to delete. |

---

## Skill Commands

### octarq skill list

List all installed skills.

```
octarq skill list
```

**Output columns:** NAME, VERSION, TOOLS, DESCRIPTION.

Loads skills from `~/.openfang/skills/` plus bundled skills compiled into the binary.

---

### octarq skill install

Install a skill from a local directory, git URL, or FangHub marketplace.

```
octarq skill install <SOURCE>
```

**Arguments:**

| Argument   | Description                                             |
| ---------- | ------------------------------------------------------- |
| `<SOURCE>` | Skill name (FangHub), local directory path, or git URL. |

**Behavior:**

- **Local directory:** Looks for `skill.toml` in the directory. If not found, checks for OpenClaw-format skills (SKILL.md with YAML frontmatter) and auto-converts them.
- **Remote (FangHub):** Fetches and installs from the FangHub marketplace. Skills pass through SHA256 verification and prompt injection scanning.

**Example:**

```bash
# Install from local directory
octarq skill install ./my-skill/

# Install from FangHub
octarq skill install web-search

# Install an OpenClaw-format skill
octarq skill install ./openclaw-skill/
```

---

### octarq skill remove

Remove an installed skill.

```
octarq skill remove <NAME>
```

**Arguments:**

| Argument | Description                  |
| -------- | ---------------------------- |
| `<NAME>` | Name of the skill to remove. |

**Example:**

```bash
octarq skill remove web-search
```

---

### octarq skill search

Search the FangHub marketplace for skills.

```
octarq skill search <QUERY>
```

**Arguments:**

| Argument  | Description          |
| --------- | -------------------- |
| `<QUERY>` | Search query string. |

**Example:**

```bash
octarq skill search "docker kubernetes"
```

---

### octarq skill create

Interactively scaffold a new skill project.

```
octarq skill create
```

**Behavior:**

Prompts for:
- Skill name
- Description
- Runtime (`python`, `node`, or `wasm`; defaults to `python`)

Creates a directory under `~/.openfang/skills/<name>/` with:
- `skill.toml` -- manifest file
- `src/main.py` (or `src/index.js`) -- entry point with boilerplate

**Example:**

```bash
octarq skill create
# Skill name: my-tool
# Description: A custom analysis tool
# Runtime (python/node/wasm) [python]: python
```

---

## Channel Commands

### octarq channel list

List configured channels and their status.

```
octarq channel list
```

**Output columns:** CHANNEL, ENV VAR, STATUS.

Checks `config.toml` for channel configuration sections and environment variables for required tokens. Status is one of: `Ready`, `Missing env`, `Not configured`.

**Channels checked:** webchat, telegram, discord, slack, whatsapp, signal, matrix, email.

---

### octarq channel setup

Interactive setup wizard for a channel integration.

```
octarq channel setup [<CHANNEL>]
```

**Arguments:**

| Argument    | Description                                               |
| ----------- | --------------------------------------------------------- |
| `<CHANNEL>` | Channel name. If omitted, displays an interactive picker. |

**Supported channels:** `telegram`, `discord`, `slack`, `whatsapp`, `email`, `signal`, `matrix`.

Each wizard:
1. Displays step-by-step instructions for obtaining credentials.
2. Prompts for tokens/credentials.
3. Saves tokens to `~/.openfang/.env` with owner-only permissions.
4. Appends the channel configuration block to `config.toml` (prompts for confirmation).
5. Warns to restart the daemon if one is running.

**Example:**

```bash
# Interactive picker
octarq channel setup

# Direct setup
octarq channel setup telegram
octarq channel setup discord
octarq channel setup slack
```

---

### octarq channel test

Send a test message through a configured channel.

```
octarq channel test <CHANNEL>
```

**Arguments:**

| Argument    | Description           |
| ----------- | --------------------- |
| `<CHANNEL>` | Channel name to test. |

Requires a running daemon. Sends `POST /api/channels/<channel>/test`.

**Example:**

```bash
octarq channel test telegram
```

---

### octarq channel enable

Enable a channel integration.

```
octarq channel enable <CHANNEL>
```

**Arguments:**

| Argument    | Description             |
| ----------- | ----------------------- |
| `<CHANNEL>` | Channel name to enable. |

In daemon mode: sends `POST /api/channels/<channel>/enable`. Without a daemon: prints a note that the change will take effect on next start.

---

### octarq channel disable

Disable a channel without removing its configuration.

```
octarq channel disable <CHANNEL>
```

**Arguments:**

| Argument    | Description              |
| ----------- | ------------------------ |
| `<CHANNEL>` | Channel name to disable. |

In daemon mode: sends `POST /api/channels/<channel>/disable`. Without a daemon: prints a note to edit `config.toml`.

---

## Config Commands

### octarq config show

Display the current configuration file.

```
octarq config show
```

Prints the contents of `~/.openfang/config.toml` with the file path as a header comment.

---

### octarq config edit

Open the configuration file in your editor.

```
octarq config edit
```

Uses `$EDITOR`, then `$VISUAL`, then falls back to `notepad` (Windows) or `vi` (Unix).

---

### octarq config get

Get a single configuration value by dotted key path.

```
octarq config get <KEY>
```

**Arguments:**

| Argument | Description                              |
| -------- | ---------------------------------------- |
| `<KEY>`  | Dotted key path into the TOML structure. |

**Example:**

```bash
octarq config get default_model.provider
# groq

octarq config get api_listen
# 127.0.0.1:4200

octarq config get memory.decay_rate
# 0.05
```

---

### octarq config set

Set a configuration value by dotted key path.

```
octarq config set <KEY> <VALUE>
```

**Arguments:**

| Argument  | Description                                                                               |
| --------- | ----------------------------------------------------------------------------------------- |
| `<KEY>`   | Dotted key path.                                                                          |
| `<VALUE>` | New value. Type is inferred from the existing value (integer, float, boolean, or string). |

**Warning:** This command re-serializes the TOML file, which strips all comments.

**Example:**

```bash
octarq config set default_model.provider anthropic
octarq config set default_model.model claude-sonnet-4-20250514
octarq config set api_listen "0.0.0.0:4200"
```

---

### octarq config set-key

Save an LLM provider API key to `~/.openfang/.env`.

```
octarq config set-key <PROVIDER>
```

**Arguments:**

| Argument     | Description                                                                                                                                                                   |
| ------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `<PROVIDER>` | Provider name (e.g. `groq`, `anthropic`, `openai`, `gemini`, `deepseek`, `openrouter`, `together`, `mistral`, `fireworks`, `perplexity`, `cohere`, `xai`, `brave`, `tavily`). |

**Behavior:**

- Prompts interactively for the API key.
- Saves to `~/.openfang/.env` as `<PROVIDER_NAME>_API_KEY=<value>`.
- Runs a live validation test against the provider's API.
- File permissions are restricted to owner-only on Unix.

**Example:**

```bash
octarq config set-key groq
# Paste your groq API key: gsk_...
# [ok] Saved GROQ_API_KEY to ~/.openfang/.env
# Testing key... OK
```

---

### octarq config delete-key

Remove an API key from `~/.openfang/.env`.

```
octarq config delete-key <PROVIDER>
```

**Arguments:**

| Argument     | Description    |
| ------------ | -------------- |
| `<PROVIDER>` | Provider name. |

**Example:**

```bash
octarq config delete-key openai
```

---

### octarq config test-key

Test provider connectivity with the stored API key.

```
octarq config test-key <PROVIDER>
```

**Arguments:**

| Argument     | Description    |
| ------------ | -------------- |
| `<PROVIDER>` | Provider name. |

**Behavior:**

- Reads the API key from the environment (loaded from `~/.openfang/.env`).
- Hits the provider's models/health endpoint.
- Reports `OK` (key accepted) or `FAILED (401/403)` (key rejected).
- Exits with code 1 on failure.

**Example:**

```bash
octarq config test-key groq
# Testing groq (GROQ_API_KEY)... OK
```

---

## Quick Chat

### octarq chat

Quick alias for starting a chat session.

```
octarq chat [<AGENT>]
```

**Arguments:**

| Argument  | Description                  |
| --------- | ---------------------------- |
| `<AGENT>` | Optional agent name or UUID. |

**Behavior:**

- **Daemon mode:** Finds the agent by name or ID among running agents. If no agent name is given, uses the first available agent. If no agents exist, suggests `octarq agent new`.
- **Standalone mode (no daemon):** Boots an in-process kernel and auto-spawns an agent from templates. Searches for an agent matching the given name, then falls back to `assistant`, then to the first available template.

This is the simplest way to start chatting -- it works with or without a daemon.

**Example:**

```bash
# Chat with the default agent
octarq chat

# Chat with a specific agent by name
octarq chat coder

# Chat with a specific agent by UUID
octarq chat a1b2c3d4-e5f6-7890-abcd-ef1234567890
```

---

## Migration

### octarq migrate

Migrate configuration and agents from another agent framework.

```
octarq migrate --from <FRAMEWORK> [--source-dir <PATH>] [--dry-run]
```

**Options:**

| Option                | Description                                                                                                |
| --------------------- | ---------------------------------------------------------------------------------------------------------- |
| `--from <FRAMEWORK>`  | Source framework. One of: `openclaw`, `langchain`, `autogpt`.                                              |
| `--source-dir <PATH>` | Path to the source workspace. Auto-detected if not set (e.g. `~/.openclaw`, `~/.langchain`, `~/Auto-GPT`). |
| `--dry-run`           | Show what would be imported without making changes.                                                        |

**Behavior:**

- Converts agent configurations, YAML manifests, and settings from the source framework into Octarq format.
- Saves imported data to `~/.openfang/`.
- Writes a `migration_report.md` summarizing what was imported.

**Example:**

```bash
# Dry run migration from OpenClaw
octarq migrate --from openclaw --dry-run

# Migrate from OpenClaw (auto-detect source)
octarq migrate --from openclaw

# Migrate from LangChain with explicit source
octarq migrate --from langchain --source-dir /home/user/.langchain

# Migrate from AutoGPT
octarq migrate --from autogpt
```

---

## MCP Server

### octarq mcp

Start an MCP (Model Context Protocol) server over stdio.

```
octarq mcp
```

**Behavior:**

- Exposes running Octarq agents as MCP tools via JSON-RPC 2.0 over stdin/stdout with Content-Length framing.
- Each agent becomes a callable tool named `openfang_agent_<name>` (hyphens replaced with underscores).
- Connects to a running daemon via HTTP if available; otherwise boots an in-process kernel.
- Protocol version: `2024-11-05`.
- Maximum message size: 10MB (security limit).

**Supported MCP methods:**

| Method       | Description                                           |
| ------------ | ----------------------------------------------------- |
| `initialize` | Returns server capabilities and info.                 |
| `tools/list` | Lists all available agent tools.                      |
| `tools/call` | Sends a message to an agent and returns the response. |

**Tool input schema:**

Each agent tool accepts a single `message` (string) argument.

**Integration with Claude Desktop / other MCP clients:**

Add to your MCP client configuration:

```json
{
  "mcpServers": {
    "octarq": {
      "command": "octarq",
      "args": ["mcp"]
    }
  }
}
```

---

## Daemon Auto-Detect

The CLI uses a two-step mechanism to detect a running daemon:

1. **Read `daemon.json`:** On startup, the daemon writes `~/.openfang/daemon.json` containing the listen address (e.g. `127.0.0.1:4200`). The CLI reads this file to learn where the daemon is.

2. **Health check:** The CLI sends `GET http://<listen_addr>/api/health` with a 2-second timeout. If the health check succeeds, the daemon is considered running and the CLI uses HTTP to communicate with it.

If either step fails (no `daemon.json`, stale file, health check timeout), the CLI falls back to in-process mode for commands that support it. Commands that require a daemon (workflows, triggers, channel test/enable/disable, dashboard) will exit with an error and a helpful message.

**Daemon lifecycle:**

```
octarq start          # Starts daemon, writes daemon.json
                        # Other CLI instances detect daemon.json
octarq status         # Connects to daemon via HTTP
Ctrl+C                  # Daemon shuts down, daemon.json removed

octarq doctor --repair  # Cleans up stale daemon.json from crashes
```

---

## Environment File

Octarq loads `~/.openfang/.env` into the process environment on every CLI invocation. System environment variables take priority over `.env` values.

The `.env` file stores API keys and secrets:

```bash
GROQ_API_KEY=gsk_...
ANTHROPIC_API_KEY=sk-ant-...
GEMINI_API_KEY=AIza...
TELEGRAM_BOT_TOKEN=123456:ABC-DEF...
```

Manage keys with the `config set-key` / `config delete-key` commands rather than editing the file directly, as these commands enforce correct permissions.

---

## Exit Codes

| Code  | Meaning                                                                                             |
| ----- | --------------------------------------------------------------------------------------------------- |
| `0`   | Success.                                                                                            |
| `1`   | General error (invalid arguments, failed operations, missing daemon, parse errors, spawn failures). |
| `130` | Interrupted by second `Ctrl+C` (force exit).                                                        |

---

## Examples

### First-time setup

```bash
# 1. Set your API key
export GROQ_API_KEY="gsk_your_key_here"

# 2. Initialize Octarq
octarq init --quick

# 3. Start the daemon
octarq start
```

### Daily usage

```bash
# Quick chat (auto-spawns agent if needed)
octarq chat

# Chat with a specific agent
octarq chat coder

# Check what's running
octarq status

# Open the web dashboard
octarq dashboard
```

### Agent management

```bash
# Spawn from a template
octarq agent new assistant

# Spawn from a custom manifest
octarq agent spawn ./agents/custom-agent/agent.toml

# List running agents
octarq agent list

# Chat with an agent by UUID
octarq agent chat <UUID>

# Kill an agent
octarq agent kill <UUID>
```

### Workflow automation

```bash
# Create a workflow
octarq workflow create ./review-pipeline.json

# List workflows
octarq workflow list

# Run a workflow
octarq workflow run <WORKFLOW_ID> "Review the latest PR"
```

### Event triggers

```bash
# Create a trigger that fires on agent spawn
octarq trigger create <AGENT_ID> '{"agent_spawned":{"name_pattern":"*"}}' \
  --prompt "New agent spawned: {{event}}" \
  --max-fires 100

# List all triggers
octarq trigger list

# List triggers for a specific agent
octarq trigger list --agent-id <AGENT_ID>

# Delete a trigger
octarq trigger delete <TRIGGER_ID>
```

### Skill management

```bash
# Search FangHub
octarq skill search "code review"

# Install a skill
octarq skill install code-reviewer

# List installed skills
octarq skill list

# Create a new skill
octarq skill create

# Remove a skill
octarq skill remove code-reviewer
```

### Channel setup

```bash
# Interactive channel picker
octarq channel setup

# Direct channel setup
octarq channel setup telegram

# Check channel status
octarq channel list

# Test a channel
octarq channel test telegram

# Enable/disable channels
octarq channel enable discord
octarq channel disable slack
```

### Configuration

```bash
# View config
octarq config show

# Get a specific value
octarq config get default_model.provider

# Change provider
octarq config set default_model.provider anthropic
octarq config set default_model.model claude-sonnet-4-20250514
octarq config set default_model.api_key_env ANTHROPIC_API_KEY

# Manage API keys
octarq config set-key anthropic
octarq config test-key anthropic
octarq config delete-key openai

# Open in editor
octarq config edit
```

### Migration from other frameworks

```bash
# Preview migration
octarq migrate --from openclaw --dry-run

# Run migration
octarq migrate --from openclaw

# Migrate from LangChain
octarq migrate --from langchain --source-dir ~/.langchain
```

### MCP integration

```bash
# Start MCP server for Claude Desktop or other MCP clients
octarq mcp
```

### Diagnostics

```bash
# Run all diagnostic checks
octarq doctor

# Auto-repair issues
octarq doctor --repair

# Machine-readable diagnostics
octarq doctor --json
```

### Shell completions

```bash
# Generate and install completions for your shell
octarq completion bash >> ~/.bashrc
octarq completion zsh > "${fpath[1]}/_octarq"
octarq completion fish > ~/.config/fish/completions/octarq.fish
```

---

## Supported LLM Providers

The following providers are recognized by `octarq config set-key` and `octarq doctor`:

| Provider   | Environment Variable                 | Default Model              |
| ---------- | ------------------------------------ | -------------------------- |
| Groq       | `GROQ_API_KEY`                       | `llama-3.3-70b-versatile`  |
| Gemini     | `GEMINI_API_KEY` or `GOOGLE_API_KEY` | `gemini-2.5-flash`         |
| DeepSeek   | `DEEPSEEK_API_KEY`                   | `deepseek-chat`            |
| Anthropic  | `ANTHROPIC_API_KEY`                  | `claude-sonnet-4-20250514` |
| OpenAI     | `OPENAI_API_KEY`                     | `gpt-4o`                   |
| OpenRouter | `OPENROUTER_API_KEY`                 | `openrouter/auto`          |
| Together   | `TOGETHER_API_KEY`                   | --                         |
| Mistral    | `MISTRAL_API_KEY`                    | --                         |
| Fireworks  | `FIREWORKS_API_KEY`                  | --                         |
| Perplexity | `PERPLEXITY_API_KEY`                 | --                         |
| Cohere     | `COHERE_API_KEY`                     | --                         |
| xAI        | `XAI_API_KEY`                        | --                         |

Additional search/fetch provider keys: `BRAVE_API_KEY`, `TAVILY_API_KEY`.
