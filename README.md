<div align="center">

<br/>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/benchmarks/architecture-overview.svg">
  <img alt="OpenFang" width="80" src="docs/benchmarks/architecture-overview.svg">
</picture>

```
  ___                   _____
 / _ \ _ __   ___ _ __ |  ___|_ _ _ __   __ _
| | | | '_ \ / _ \ '_ \| |_ / _` | '_ \ / _` |
| |_| | |_) |  __/ | | |  _| (_| | | | | (_| |
 \___/| .__/ \___|_| |_|_|  \__,_|_| |_|\__, |
      |_|                                |___/
```

### The Open-Source Agent Operating System

**Deploy autonomous AI agents that actually work.** 40 channels. 53 tools. 7 autonomous Hands. 27 providers. 140 API endpoints. 16 security systems. 137K lines of Rust. One binary.

[Quick Start](#-quick-start) | [Why OpenFang](#-why-openfang) | [Hands](#-hands--autonomous-agent-packages) | [Benchmarks](#-benchmarks) | [Architecture](#-architecture) | [Docs](docs/)

<br/>

> **Early Release Notice** &mdash; OpenFang `v0.1` is feature-complete but new. You may encounter rough edges, breaking config changes between versions, or unexpected behavior under heavy load. We ship fast and fix fast. [Report issues here.](https://github.com/RightNow-AI/openfang/issues) Production deployments should pin to a specific commit until `v1.0`.

<br/>

</div>

---

## Migrating from OpenClaw?

```bash
openfang migrate --from openclaw
```

**One command.** Your agents, memory, skills, and channel configs are imported automatically. OpenFang reads SKILL.md files natively and installs directly from ClawHub. Everything you built keeps working &mdash; but now it runs faster, with more security, more tools, and an actual operating system underneath.

See [MIGRATION.md](MIGRATION.md) for the full guide.

---

## Why OpenFang?

Most agent frameworks are Python libraries glued together with hope. OpenFang is different &mdash; it's an **operating system** for AI agents, written from scratch in Rust.

| What you get | What that means |
|---|---|
| **Kernel architecture** | Agents are first-class OS processes with registries, schedulers, capability gates, and event buses &mdash; not Python scripts in a loop |
| **16 security layers** | Capability enforcement, WASM sandbox, taint tracking, Merkle audit trail, Ed25519 manifests, SSRF protection, secret zeroization &mdash; defense in depth, not config flags |
| **Hands** | Pre-built autonomous agent packages that *work for you* &mdash; activate a Hand and it runs 24/7 (lead gen, research, social media, predictions) |
| **53 built-in tools** | Filesystem, web, shell, browser automation, inter-agent communication, scheduling, image generation, Docker, TTS/STT &mdash; all sandboxed |
| **140 API endpoints** | Full REST/WS/SSE API with OpenAI-compatible `/v1/chat/completions`, so any existing client just works |
| **40 channels** | Telegram, Discord, Slack, WhatsApp, Teams, LINE, Mastodon, Matrix, Email, and 31 more &mdash; every adapter has rate limiting, DM/group policies, and per-channel model overrides |
| **One binary, <200ms startup** | No Python, no pip, no virtualenvs, no Docker required. Download one binary and you're running |

### What OpenClaw doesn't have

OpenClaw pioneered open-source agents. OpenFang builds on that vision and goes further:

- **9x more LLM providers** (27 vs 3) with native Anthropic, Gemini, and OpenAI drivers
- **3.5x more tools** (53 vs 15) including browser automation, Docker sandbox, image generation, TTS
- **4.6x more API endpoints** (140 vs ~30) with full OpenAI-compatible API
- **16 vs 3 security systems** &mdash; WASM sandbox with dual metering, Merkle audit trail, taint tracking, Ed25519 manifests (OpenClaw has config-based ACL)
- **Hands** &mdash; autonomous agent packages that don't exist in OpenClaw
- **P2P networking** via OFP with HMAC-SHA256 mutual auth
- **MCP client + server** for IDE integration
- **A2A protocol** for cross-platform agent interop
- **Visual workflow builder** with drag-and-drop
- **Per-response cost tracking** with budget management
- **16x faster cold start** (<200ms vs ~3.2s)
- **6x smaller install** (~32MB vs ~200MB)
- **1,759 tests** with zero clippy warnings

---

## Benchmarks

### Security Defense Depth

How many independent, testable security mechanisms does each framework provide?

<p align="center">
  <img src="docs/benchmarks/security-layers.svg" alt="Security layers comparison" width="700"/>
</p>

OpenFang implements **16 discrete security systems** that operate independently: capability gates, WASM dual-metered sandbox, information flow taint tracking, Merkle hash-chain audit trail, Ed25519 signed manifests, SSRF protection, secret zeroization, OFP mutual authentication, security headers, GCRA rate limiter, path traversal prevention, subprocess sandbox, prompt injection scanner, loop guard, session repair, and health endpoint redaction.

### Feature Coverage

Raw feature counts from source code and public documentation:

<p align="center">
  <img src="docs/benchmarks/feature-coverage.svg" alt="Feature coverage comparison" width="750"/>
</p>

### Runtime Performance

Native Rust binary vs. Python runtime overhead:

<p align="center">
  <img src="docs/benchmarks/performance.svg" alt="Performance comparison" width="750"/>
</p>

<details>
<summary><strong>Full comparison table</strong></summary>

| Feature | OpenFang | OpenClaw | LangChain | AutoGPT | CrewAI |
|---------|----------|----------|-----------|---------|--------|
| Language | **Rust** | Python | Python | Python | Python |
| Architecture | **OS kernel** | Plugin system | Library | Single agent | Framework |
| Channels | **40** | 38 | 0 | 0 | 0 |
| Bundled skills | **60** | 57 | 0 | 0 | 0 |
| Built-in tools | **53** | ~15 | ~8 | ~10 | ~5 |
| LLM providers | **27** (3 native drivers) | 3 | ~20 | 1-2 | Few |
| Models in catalog | **123** | ~20 | Varies | ~5 | Varies |
| API endpoints | **140** | ~30 | N/A | ~10 | N/A |
| Security systems | **16** | 3 | 1 | 2 | 1 |
| Autonomous Hands | **7** | 0 | 0 | 0 | 0 |
| OpenAI-compat API | Streaming | No | No | No | No |
| MCP support | Client + Server | No | No | No | No |
| A2A protocol | Full | No | No | No | No |
| WASM sandbox | Dual metered | No | No | No | No |
| Desktop app | Tauri 2.0 | Electron | None | None | None |
| P2P networking | OFP (HMAC-SHA256) | None | None | None | None |
| Visual workflows | Drag-and-drop | None | None | None | None |
| Cost tracking | Per-response + budget | None | Callbacks | None | None |
| Audit trail | Merkle chain | None | None | None | None |
| Cold start | **<200ms** | ~3.2s | ~2s | ~5s | ~2s |
| Install size | **~32 MB** | ~200 MB | ~350 MB | ~200 MB | ~150 MB |
| Tests | **1,759** | Unknown | Unknown | Unknown | Unknown |
| Voice UI | Mic + TTS | No | No | No | No |
| Client SDKs | JS + Python | None | Python | None | None |

</details>

---

## Hands &mdash; Autonomous Agent Packages

**Hands are the killer feature.** A Hand is a pre-built, domain-complete autonomous agent that you activate and it *works for you*. Unlike regular agents (where you chat back and forth), Hands run independently &mdash; you configure them once, and they execute on a schedule or continuously.

Think of it like installing an app on your phone, except the app is an AI agent with tools, memory, and a mission.

### 7 Bundled Hands

| Hand | What it does |
|------|-------------|
| **Browser** | Autonomous web browser &mdash; navigates sites, fills forms, clicks buttons, completes multi-step web tasks. Asks for approval before purchases. |
| **Researcher** | Deep research agent &mdash; exhaustive investigation across the web, cross-referencing, fact-checking, delivers structured reports with citations. |
| **Collector** | Intelligence collector &mdash; monitors any target (competitor, topic, market) continuously with change detection and knowledge graphs. |
| **Lead** | Lead generation &mdash; discovers, enriches, and delivers qualified leads on a schedule. Outputs structured contact lists. |
| **Predictor** | Future prediction engine &mdash; collects signals, builds reasoning chains, makes calibrated predictions, and tracks its own accuracy over time. |
| **Twitter** | Twitter/X manager &mdash; content creation, scheduled posting, engagement monitoring, and performance analytics. |
| **Clip** | Video clip generator &mdash; turns long-form video into viral short clips with captions and thumbnails. |

### How Hands Work

```bash
# List available Hands
openfang hand list

# Activate a Hand
openfang hand activate researcher

# Check status
openfang hand status researcher

# Deactivate
openfang hand deactivate researcher
```

Or use the agent tools programmatically:
```
hand_activate({ "hand": "lead", "config": { "industry": "SaaS", "region": "US" } })
hand_status({ "hand": "lead" })
```

Each Hand ships with:
- A **HAND.toml** manifest (tools, capabilities, requirements, configurable settings)
- A **SKILL.md** knowledge file (domain expertise injected into the LLM)
- **Dashboard metrics** (built-in monitoring and status)
- **Guardrails** (approval gates for sensitive actions like purchases)

### Build Your Own Hand

```toml
# my-hand/HAND.toml
[hand]
id = "my-custom-hand"
name = "My Custom Hand"
version = "0.1.0"
description = "Does something amazing autonomously"
category = "productivity"

[agent]
template = "assistant"
system_prompt = "You are an autonomous agent that..."

[tools]
allow = ["web_fetch", "web_search", "file_write", "shell_exec"]

[settings]
schedule = "every 6 hours"
```

---

## Architecture

OpenFang is structured like an operating system. The **kernel** manages agent lifecycles, enforces security policies, and coordinates subsystems. The **runtime** executes agent logic with LLM drivers and sandboxed tools. **Channels** connect agents to the outside world. **Memory** persists everything in SQLite.

<p align="center">
  <img src="docs/benchmarks/architecture-overview.svg" alt="OpenFang Architecture" width="850"/>
</p>

### 14 Crates

| Crate | Role | Lines |
|-------|------|-------|
| `openfang-kernel` | Agent registry, scheduler, capabilities, workflows, RBAC, metering, heartbeat, budget | 4,630 |
| `openfang-runtime` | Agent loop, 3 LLM drivers, 53 tools, WASM sandbox, MCP, A2A, web search, stability | 18,000+ |
| `openfang-api` | 140 REST/WS/SSE endpoints, OpenAI-compat, rate limiter, security headers, dashboard | 8,000+ |
| `openfang-channels` | 40 channel adapters, bridge, formatter, rate limiter, DM/group policies | 12,000+ |
| `openfang-memory` | SQLite substrate (KV, embeddings, usage, canonical sessions, JSONL mirror) | 2,800+ |
| `openfang-types` | Core types, taint tracking, Ed25519 manifests, model catalog types, config | 5,500+ |
| `openfang-skills` | 60 bundled skills, SKILL.md parser, FangHub marketplace, ClawHub client, security scanner | 4,000+ |
| `openfang-hands` | Hand registry, 7 bundled Hands, HAND.toml parser, lifecycle management | 1,200+ |
| `openfang-extensions` | 25 MCP templates, AES-256-GCM credential vault, OAuth2 PKCE, health monitor | 3,500+ |
| `openfang-wire` | P2P networking via OFP with HMAC-SHA256 mutual authentication | 1,500+ |
| `openfang-cli` | Command-line interface, TUI dashboard, daemon auto-detect, MCP server mode | 5,200+ |
| `openfang-desktop` | Tauri 2.0 native app (WebView, system tray, notifications, single-instance) | 200+ |
| `openfang-migrate` | Migration engine (OpenClaw YAML, LangChain, AutoGPT) | 4,000+ |
| `xtask` | Build automation | 200+ |

**Total: 137,728 lines of Rust. 1,759 tests. Zero clippy warnings.**

---

## Quick Start

```bash
# Install from source
git clone https://github.com/RightNow-AI/openfang.git
cd openfang
cargo install --path crates/openfang-cli

# Configure (pick any provider)
export GROQ_API_KEY="your-key"       # Free tier available
# or: ANTHROPIC_API_KEY, OPENAI_API_KEY, GEMINI_API_KEY, DEEPSEEK_API_KEY, etc.

# Initialize workspace
openfang init

# Start the daemon (API server + web dashboard + channels)
openfang start
# Dashboard: http://localhost:4200

# Spawn and chat with an agent
openfang agent spawn assistant
openfang agent chat <id>

# Or use the HTTP API
curl -X POST http://localhost:4200/api/agents \
  -H "Content-Type: application/json" \
  -d '{"template": "coder"}'
```

### Docker

```bash
docker run -p 4200:4200 -e GROQ_API_KEY=your-key ghcr.io/RightNow-AI/openfang
```

### Connect to Any LLM

```bash
# Cloud providers (API key required)
export ANTHROPIC_API_KEY="..."    # Claude models
export OPENAI_API_KEY="..."       # GPT models
export GEMINI_API_KEY="..."       # Gemini models
export GROQ_API_KEY="..."         # Fast inference (free tier)
export DEEPSEEK_API_KEY="..."     # DeepSeek models
export XAI_API_KEY="..."          # Grok models

# Local models (no API key needed)
export OLLAMA_BASE_URL="http://localhost:11434"
export LMSTUDIO_BASE_URL="http://localhost:1234"
```

OpenFang supports **27 providers** and **123+ models** out of the box. See [docs/providers.md](docs/providers.md) for the full list.

---

## 40 Messaging Channels

Connect your agents to every platform your users are on:

<details>
<summary><strong>View all 40 channels</strong></summary>

| Channel | Protocol | Category |
|---------|----------|----------|
| Telegram | Bot API (long-polling) | Core |
| Discord | Gateway WebSocket v10 | Core |
| Slack | Events API + Web API | Core |
| WhatsApp | Cloud API (webhook) | Core |
| Signal | signal-cli REST API | Core |
| Matrix | Client-Server API (/sync) | Core |
| Email | IMAP + SMTP | Core |
| Microsoft Teams | Bot Framework v3 (webhook) | Enterprise |
| Mattermost | WebSocket + REST v4 | Enterprise |
| Google Chat | Service account webhook | Enterprise |
| Webex | Bot SDK (WebSocket) | Enterprise |
| Feishu/Lark | Open Platform webhook | Enterprise |
| Rocket.Chat | REST API (polling) | Enterprise |
| Zulip | Event queue (long-polling) | Enterprise |
| LINE | Messaging API (webhook) | Social |
| Viber | Bot API (webhook) | Social |
| Facebook Messenger | Platform API (webhook) | Social |
| Mastodon | Streaming API (WebSocket) | Social |
| Bluesky | AT Protocol (WebSocket) | Social |
| Reddit | OAuth2 API (polling) | Social |
| LinkedIn | Messaging API (polling) | Social |
| Twitch | IRC gateway | Social |
| IRC | Raw TCP (PRIVMSG) | Community |
| XMPP | XMPP protocol | Community |
| Guilded | WebSocket API | Community |
| Revolt | WebSocket API | Community |
| Keybase | Bot API (polling) | Community |
| Discourse | REST API (polling) | Community |
| Gitter | Streaming API | Community |
| Nextcloud Talk | REST API (polling) | Self-hosted |
| Threema | Gateway API (webhook) | Privacy |
| Nostr | NIP-01 relay (WebSocket) | Privacy |
| Mumble | TCP text protocol | Privacy |
| Pumble | Bot API (webhook) | Workplace |
| Flock | Bot API (webhook) | Workplace |
| Twist | API v3 (polling) | Workplace |
| DingTalk | Robot API (webhook) | Workplace |
| ntfy | SSE pub/sub | Notification |
| Gotify | WebSocket API | Notification |
| Webhook | Generic HTTP in/out (HMAC-SHA256) | Integration |

</details>

Every adapter features: graceful shutdown, exponential backoff on reconnect, `Zeroizing<String>` for secrets, message splitting for platform limits, and per-channel model/prompt/policy overrides.

```bash
openfang channel setup telegram    # Interactive wizard
openfang channel setup discord
openfang channel setup slack
```

---

## 53 Built-in Tools

| Category | Tools |
|----------|-------|
| **Filesystem** | `file_read`, `file_write`, `file_list`, `apply_patch` |
| **Web** | `web_fetch`, `web_search` (4-engine: Tavily, Brave, Perplexity, DuckDuckGo) |
| **Shell** | `shell_exec` |
| **Inter-agent** | `agent_send`, `agent_spawn`, `agent_list`, `agent_kill`, `agent_find` |
| **Memory** | `memory_store`, `memory_recall` |
| **Collaboration** | `task_post`, `task_claim`, `task_complete`, `task_list`, `event_publish` |
| **Scheduling** | `schedule_create`, `schedule_list`, `schedule_delete` |
| **Knowledge** | `knowledge_add_entity`, `knowledge_add_relation`, `knowledge_query` |
| **Vision & Media** | `image_analyze`, `media_describe`, `media_transcribe`, `location_get` |
| **Browser** | `browser_navigate`, `browser_click`, `browser_type`, `browser_screenshot`, `browser_read_page`, `browser_close` |
| **Image Gen** | `image_generate` (DALL-E / GPT-Image) |
| **Cron** | `cron_create`, `cron_list`, `cron_cancel` |
| **Hands** | `hand_list`, `hand_activate`, `hand_status`, `hand_deactivate` |
| **A2A** | `a2a_discover`, `a2a_send` |
| **Voice** | `text_to_speech`, `speech_to_text` |
| **Docker** | `docker_exec` |
| **Processes** | `process_start`, `process_poll`, `process_write`, `process_kill`, `process_list` |
| **Canvas** | `canvas_present` |

Plus dynamically loaded tools from **MCP servers** and **skills**.

---

## 30 Agent Templates

```bash
openfang agent spawn coder              # Software development
openfang agent spawn researcher         # Deep web research
openfang agent spawn writer             # Content writing
openfang agent spawn analyst            # Data analysis
openfang agent spawn ops                # DevOps & sysadmin
openfang agent spawn orchestrator       # Multi-agent delegation
openfang agent spawn architect          # System design
openfang agent spawn security-auditor   # Security analysis
openfang agent spawn data-scientist     # ML & data science
openfang agent spawn assistant          # General assistant
# + 20 more (debugger, planner, translator, recruiter, tutor, etc.)
```

Each template is diversified across **4 provider tiers** (Frontier, Smart, Balanced, Fast) for optimal cost/performance:

```toml
# Per-agent model override
[model]
provider = "groq"
model = "llama-3.3-70b-versatile"
api_key_env = "GROQ_API_KEY"
```

---

## 60 Bundled Skills

Expert knowledge modules that enhance any agent's capabilities:

| Category | Skills |
|----------|--------|
| **DevOps & Infra** | ci-cd, ansible, prometheus, nginx, kubernetes, terraform, helm, docker, sysadmin, shell-scripting, linux-networking |
| **Cloud** | aws, gcp, azure |
| **Languages** | rust-expert, python-expert, typescript-expert, golang-expert |
| **Frontend** | react-expert, nextjs-expert, css-expert |
| **Databases** | postgres-expert, redis-expert, sqlite-expert, mongodb, elasticsearch, sql-analyst |
| **APIs & Web** | graphql-expert, openapi-expert, api-tester, oauth-expert |
| **AI/ML** | ml-engineer, llm-finetuning, vector-db, prompt-engineer |
| **Security** | security-audit, crypto-expert, compliance |
| **Dev Tools** | github, git-expert, jira, linear-tools, sentry, code-reviewer, regex-expert |
| **Writing** | technical-writer, writing-coach, email-writer, presentation |
| **Data** | data-analyst, data-pipeline |
| **Collaboration** | slack-tools, notion, confluence, figma-expert |
| **Advanced** | wasm-expert, pdf-reader, web-search |

Skills use the **SKILL.md format** (YAML frontmatter + Markdown expert knowledge), are embedded at compile time, and pass automated prompt injection scanning.

---

## Security &mdash; 16 Defense Systems

OpenFang uses **defense-in-depth** with 16 independent security systems woven through every layer:

| System | What it does |
|--------|-------------|
| **Capability gates** | Agents declare required tools, network, memory access &mdash; kernel enforces at runtime |
| **WASM dual metering** | Fuel + epoch interruption with watchdog thread for sandboxed code execution |
| **Merkle audit trail** | Hash-chained audit log &mdash; tamper-evident record of every agent action |
| **Taint tracking** | Information flow labels track data provenance across agents and tools |
| **Ed25519 manifests** | Cryptographically signed agent manifests prevent tampering |
| **SSRF protection** | Private IP + cloud metadata endpoint blocking on all outbound requests |
| **Secret zeroization** | `Zeroizing<String>` auto-wipes API keys from memory on drop |
| **OFP mutual auth** | HMAC-SHA256 nonce-based mutual authentication for P2P networking |
| **Security headers** | CSP, X-Frame-Options, X-Content-Type-Options, HSTS on all HTTP responses |
| **GCRA rate limiter** | Cost-aware token bucket with per-IP tracking and stale entry cleanup |
| **Path traversal prevention** | `safe_resolve_path` blocks `../` directory traversal in all file tools |
| **Subprocess sandbox** | `env_clear()` + selective variable passthrough for skill execution |
| **Prompt injection scanner** | Detects override attempts, data exfiltration, shell references in skills |
| **Loop guard** | SHA256-based tool call loop detection with warn/block/circuit-breaker |
| **Session repair** | Auto-fixes corrupted conversation history (orphaned tool results, duplicates) |
| **Health redaction** | Public health endpoint shows minimal info; full details behind auth |

```toml
# Per-agent capability enforcement
[capabilities]
tools = ["file_read", "web_fetch"]     # Only these tools allowed
network = ["api.example.com:443"]      # Only these hosts
memory_read = ["self.*"]               # Only own memory
max_llm_tokens_per_hour = 50000        # Resource quota
```

---

## REST API (140 endpoints)

Full reference: [docs/api-reference.md](docs/api-reference.md)

<details>
<summary><strong>View all endpoints</strong></summary>

### Core
```
GET  /api/health                         # Health check (redacted for public)
GET  /api/health/detail                  # Full health (auth required)
GET  /api/status                         # System status
GET  /api/version                        # Version info
POST /api/shutdown                       # Graceful shutdown (10-phase)
```

### Agents
```
GET    /api/agents                       # List all agents
POST   /api/agents                       # Spawn from TOML manifest
GET    /api/agents/{id}                  # Agent details
DELETE /api/agents/{id}                  # Kill agent
PUT    /api/agents/{id}/update           # Update config
PUT    /api/agents/{id}/model            # Switch model
POST   /api/agents/{id}/message          # Send message (blocking)
POST   /api/agents/{id}/message/stream   # Send message (SSE)
GET    /api/agents/{id}/ws               # WebSocket chat
POST   /api/agents/{id}/upload           # File upload
PATCH  /api/agents/{id}/identity         # Update identity
```

### OpenAI-Compatible
```
POST /v1/chat/completions               # Chat (streaming + non-streaming)
GET  /v1/models                          # List agents as models
```

### Budget & Cost
```
GET  /api/budget                         # Global budget status
PUT  /api/budget                         # Update budget limits
GET  /api/budget/agents                  # Per-agent cost ranking
GET  /api/budget/agents/{id}             # Single agent budget detail
```

### A2A (Agent-to-Agent)
```
GET  /.well-known/agent.json             # A2A agent card
GET  /a2a/agents                         # A2A agent list
POST /a2a/tasks/send                     # Send A2A task
GET  /api/a2a/agents                     # External A2A agents
POST /api/a2a/discover                   # Discover external agent
POST /api/a2a/send                       # Send to external agent
```

### Network
```
GET  /api/network/status                 # OFP network status
GET  /api/peers                          # Connected OFP peers
```

### Models & Providers
```
GET  /api/models                         # Full model catalog (123+ models)
GET  /api/models/{id}                    # Model details
GET  /api/models/aliases                 # 40 aliases
GET  /api/providers                      # 27 providers with auth status
POST /api/providers/{name}/key           # Set provider API key
POST /api/providers/{name}/test          # Test connectivity
```

### Workflows, Sessions, Skills, Memory, and more
See [docs/api-reference.md](docs/api-reference.md) for the complete list.

</details>

---

## Client SDKs

### JavaScript / TypeScript

```javascript
const { OpenFang } = require("@openfang/sdk");
const client = new OpenFang("http://localhost:4200");

const agent = await client.agents.create({ template: "assistant" });
const reply = await client.agents.message(agent.id, "Hello!");

// Stream responses
for await (const event of client.agents.stream(agent.id, "Tell me a joke")) {
  if (event.type === "text_delta") process.stdout.write(event.delta);
}
```

### Python

```python
from openfang_client import OpenFang

client = OpenFang("http://localhost:4200")
agent = client.agents.create(template="assistant")
reply = client.agents.message(agent["id"], "Hello!")

for event in client.agents.stream(agent["id"], "Tell me a joke"):
    if event.get("type") == "text_delta":
        print(event["delta"], end="", flush=True)
```

Both SDKs cover: agents, sessions, workflows, skills, channels, memory, triggers, schedules, models, providers, budget, and Hands.

---

## Workflow Engine

Define multi-agent pipelines in TOML or use the **Visual Builder** (drag-and-drop node graph):

```toml
[[steps]]
name = "research"
agent_name = "researcher"
prompt = "Research: {{input}}"
mode = "sequential"

[[steps]]
name = "write"
agent_name = "writer"
prompt = "Write an article based on: {{research}}"
mode = "sequential"

[[steps]]
name = "review"
agent_name = "code-reviewer"
prompt = "Review this article: {{write}}"
mode = "sequential"
```

Supports: sequential, fan-out, collect, conditional, and loop modes.

---

## Desktop App

Native desktop app built with Tauri 2.0:

- WebView pointing at the embedded API server
- System tray with status, quick actions, and notifications
- Single-instance enforcement
- Hide-to-tray on close
- Mobile-ready architecture

```bash
cargo tauri dev      # Development
cargo tauri build    # Production
```

---

## CLI Reference

```bash
openfang init                      # Initialize workspace
openfang start                     # Start daemon (API + dashboard + channels)
openfang status                    # Check daemon status
openfang doctor                    # Diagnose setup

openfang agent spawn <template>    # Spawn agent
openfang agent list                # List agents
openfang agent chat <id>           # Interactive chat
openfang agent kill <id>           # Terminate agent

openfang hand list                 # List available Hands
openfang hand activate <name>      # Activate a Hand
openfang hand status <name>        # Check Hand status
openfang hand deactivate <name>    # Deactivate a Hand

openfang workflow list             # List workflows
openfang workflow create <file>    # Create from TOML
openfang workflow run <id>         # Execute workflow

openfang channel setup <name>      # Channel setup wizard
openfang skill list                # List skills
openfang skill search <query>      # Search FangHub + ClawHub

openfang migrate --from openclaw   # Import from OpenClaw
openfang config show               # Show configuration
openfang mcp                       # Start MCP server
```

---

## Early Release &mdash; Stability Notice

OpenFang `v0.1` is **feature-complete** with 137K lines of Rust, 1,759 tests, and zero clippy warnings. However, this is a **new release** and you should expect:

- **Breaking config changes** between minor versions until `v1.0`
- **Edge cases** in some channel adapters (especially newer ones like XMPP, Nostr, Mumble)
- **Resource usage** may be higher than expected under heavy multi-agent workloads
- **Some Hands** are more battle-tested than others (Browser and Researcher are the most mature)

We're shipping fast and fixing fast. If something breaks, [open an issue](https://github.com/RightNow-AI/openfang/issues) and we'll fix it. Pin to a specific commit for production until `v1.0`.

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, code style, and PR guidelines.

```bash
# Development workflow
cargo build --workspace             # Build all 14 crates
cargo test --workspace              # Run 1,759 tests
cargo clippy --workspace -- -D warnings  # Zero warnings policy
```

---

## License

Dual-licensed under [Apache 2.0](LICENSE-APACHE) and [MIT](LICENSE-MIT).

---

<div align="center">

**Built with Rust. Secured by design. Ready for your agents.**

[Get Started](docs/getting-started.md) | [API Reference](docs/api-reference.md) | [Security](docs/security.md) | [Report Issues](https://github.com/RightNow-AI/openfang/issues)

</div>
