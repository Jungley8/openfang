//! Status, doctor, dashboard, completion, sessions, logs, health, system info/version.

use crate::cli::Cli;
use crate::daemon::{
    boot_kernel, daemon_client, daemon_json, find_daemon, require_daemon, restrict_dir_permissions,
    restrict_file_permissions,
};
use crate::ui;
use crate::{
    copy_to_clipboard, open_in_browser, prompt_input, start_daemon_background, test_api_key,
};
use clap::CommandFactory;
use openfang_types::agent::AgentManifest;
use std::path::PathBuf;

pub fn cmd_status(config: Option<PathBuf>, json: bool) {
    if let Some(base) = find_daemon() {
        let client = daemon_client();
        let body = daemon_json(client.get(format!("{base}/api/status")).send());

        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&body).unwrap_or_default()
            );
            return;
        }

        ui::section("OpenFang Daemon Status");
        ui::blank();
        ui::kv_ok("Status", body["status"].as_str().unwrap_or("?"));
        ui::kv(
            "Agents",
            &body["agent_count"].as_u64().unwrap_or(0).to_string(),
        );
        ui::kv("Provider", body["default_provider"].as_str().unwrap_or("?"));
        ui::kv("Model", body["default_model"].as_str().unwrap_or("?"));
        ui::kv("API", &base);
        ui::kv("Dashboard", &format!("{base}/"));
        ui::kv("Data dir", body["data_dir"].as_str().unwrap_or("?"));
        ui::kv(
            "Uptime",
            &format!("{}s", body["uptime_seconds"].as_u64().unwrap_or(0)),
        );

        if let Some(agents) = body["agents"].as_array() {
            if !agents.is_empty() {
                ui::blank();
                ui::section("Active Agents");
                for a in agents {
                    println!(
                        "    {} ({}) -- {} [{}:{}]",
                        a["name"].as_str().unwrap_or("?"),
                        a["id"].as_str().unwrap_or("?"),
                        a["state"].as_str().unwrap_or("?"),
                        a["model_provider"].as_str().unwrap_or("?"),
                        a["model_name"].as_str().unwrap_or("?"),
                    );
                }
            }
        }
    } else {
        let kernel = boot_kernel(config);
        let agent_count = kernel.registry.count();

        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "in-process",
                    "agent_count": agent_count,
                    "data_dir": kernel.config.data_dir.display().to_string(),
                    "default_provider": kernel.config.default_model.provider,
                    "default_model": kernel.config.default_model.model,
                    "daemon": false,
                }))
                .unwrap_or_default()
            );
            return;
        }

        ui::section("OpenFang Status (in-process)");
        ui::blank();
        ui::kv("Agents", &agent_count.to_string());
        ui::kv("Provider", &kernel.config.default_model.provider);
        ui::kv("Model", &kernel.config.default_model.model);
        ui::kv("Data dir", &kernel.config.data_dir.display().to_string());
        ui::kv_warn("Daemon", "NOT RUNNING");
        ui::blank();
        ui::hint("Run `openfang start` to launch the daemon");

        if agent_count > 0 {
            ui::blank();
            ui::section("Persisted Agents");
            for entry in kernel.registry.list() {
                println!("    {} ({}) -- {:?}", entry.name, entry.id, entry.state);
            }
        }
    }
}

pub fn cmd_doctor(json: bool, repair: bool) {
    let mut checks: Vec<serde_json::Value> = Vec::new();
    let mut all_ok = true;
    let mut repaired = false;

    if !json {
        ui::step("OpenFang Doctor");
        println!();
    }

    let home = dirs::home_dir();
    if let Some(h) = &home {
        let openfang_dir = h.join(".openfang");

        // --- Check 1: OpenFang directory ---
        if openfang_dir.exists() {
            if !json {
                ui::check_ok(&format!("OpenFang directory: {}", openfang_dir.display()));
            }
            checks.push(serde_json::json!({"check": "openfang_dir", "status": "ok", "path": openfang_dir.display().to_string()}));
        } else if repair {
            if !json {
                ui::check_fail("OpenFang directory not found.");
            }
            let answer = prompt_input("    Create it now? [Y/n] ");
            if answer.is_empty() || answer.starts_with('y') || answer.starts_with('Y') {
                if std::fs::create_dir_all(&openfang_dir).is_ok() {
                    restrict_dir_permissions(&openfang_dir);
                    for sub in ["data", "agents"] {
                        let _ = std::fs::create_dir_all(openfang_dir.join(sub));
                    }
                    if !json {
                        ui::check_ok("Created OpenFang directory");
                    }
                    repaired = true;
                } else {
                    if !json {
                        ui::check_fail("Failed to create directory");
                    }
                    all_ok = false;
                }
            } else {
                all_ok = false;
            }
            checks.push(serde_json::json!({"check": "openfang_dir", "status": if repaired { "repaired" } else { "fail" }}));
        } else {
            if !json {
                ui::check_fail("OpenFang directory not found. Run `openfang init` first.");
            }
            checks.push(serde_json::json!({"check": "openfang_dir", "status": "fail"}));
            all_ok = false;
        }

        // --- Check 2: .env file exists + permissions ---
        let env_path = openfang_dir.join(".env");
        if env_path.exists() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = std::fs::metadata(&env_path) {
                    let mode = meta.permissions().mode() & 0o777;
                    if mode == 0o600 {
                        if !json {
                            ui::check_ok(".env file (permissions OK)");
                        }
                    } else if repair {
                        let _ = std::fs::set_permissions(
                            &env_path,
                            std::fs::Permissions::from_mode(0o600),
                        );
                        if !json {
                            ui::check_ok(".env file (permissions fixed to 0600)");
                        }
                        repaired = true;
                    } else if !json {
                        ui::check_warn(&format!(
                            ".env file has loose permissions ({:o}), should be 0600",
                            mode
                        ));
                    }
                } else if !json {
                    ui::check_ok(".env file");
                }
            }
            #[cfg(not(unix))]
            {
                if !json {
                    ui::check_ok(".env file");
                }
            }
            checks.push(serde_json::json!({"check": "env_file", "status": "ok"}));
        } else {
            if !json {
                ui::check_warn(
                    ".env file not found (create with: openfang config set-key <provider>)",
                );
            }
            checks.push(serde_json::json!({"check": "env_file", "status": "warn"}));
        }

        // --- Check 3: Config TOML syntax validation ---
        let config_path = openfang_dir.join("config.toml");
        if config_path.exists() {
            let config_content = std::fs::read_to_string(&config_path).unwrap_or_default();
            match toml::from_str::<toml::Value>(&config_content) {
                Ok(_) => {
                    if !json {
                        ui::check_ok(&format!("Config file: {}", config_path.display()));
                    }
                    checks.push(serde_json::json!({"check": "config_file", "status": "ok"}));
                }
                Err(e) => {
                    if !json {
                        ui::check_fail(&format!("Config file has syntax errors: {e}"));
                        ui::hint("Fix with: openfang config edit");
                    }
                    checks.push(serde_json::json!({"check": "config_syntax", "status": "fail", "error": e.to_string()}));
                    all_ok = false;
                }
            }
        } else if repair {
            if !json {
                ui::check_fail("Config file not found.");
            }
            let answer = prompt_input("    Create default config? [Y/n] ");
            if answer.is_empty() || answer.starts_with('y') || answer.starts_with('Y') {
                let default_config = r#"# OpenFang Agent OS configuration
# See https://github.com/RightNow-AI/openfang for documentation

# For Docker, change to "0.0.0.0:4200" or set OPENFANG_LISTEN env var.
api_listen = "127.0.0.1:4200"

[default_model]
provider = "groq"
model = "llama-3.3-70b-versatile"
api_key_env = "GROQ_API_KEY"

[memory]
decay_rate = 0.05
"#;
                let _ = std::fs::create_dir_all(&openfang_dir);
                if std::fs::write(&config_path, default_config).is_ok() {
                    restrict_file_permissions(&config_path);
                    if !json {
                        ui::check_ok("Created default config.toml");
                    }
                    repaired = true;
                } else {
                    if !json {
                        ui::check_fail("Failed to create config.toml");
                    }
                    all_ok = false;
                }
            } else {
                all_ok = false;
            }
            checks.push(serde_json::json!({"check": "config_file", "status": if repaired { "repaired" } else { "fail" }}));
        } else {
            if !json {
                ui::check_fail("Config file not found.");
            }
            checks.push(serde_json::json!({"check": "config_file", "status": "fail"}));
            all_ok = false;
        }

        // --- Check 4: Port 4200 availability ---
        if !json {
            println!();
        }
        let daemon_running = find_daemon();
        if let Some(ref base) = daemon_running {
            if !json {
                ui::check_ok(&format!("Daemon running at {base}"));
            }
            checks.push(serde_json::json!({"check": "daemon", "status": "ok", "url": base}));
        } else {
            if !json {
                ui::check_warn("Daemon not running (start with `openfang start`)");
            }
            checks.push(serde_json::json!({"check": "daemon", "status": "warn"}));

            match std::net::TcpListener::bind("127.0.0.1:4200") {
                Ok(_) => {
                    if !json {
                        ui::check_ok("Port 4200 is available");
                    }
                    checks.push(serde_json::json!({"check": "port_4200", "status": "ok"}));
                }
                Err(_) => {
                    if !json {
                        ui::check_warn("Port 4200 is in use by another process");
                    }
                    checks.push(serde_json::json!({"check": "port_4200", "status": "warn"}));
                }
            }
        }

        // --- Check 5: Stale daemon.json ---
        let daemon_json_path = openfang_dir.join("daemon.json");
        if daemon_json_path.exists() && daemon_running.is_none() {
            if repair {
                let _ = std::fs::remove_file(&daemon_json_path);
                if !json {
                    ui::check_ok("Removed stale daemon.json");
                }
                repaired = true;
            } else if !json {
                ui::check_warn(
                    "Stale daemon.json found (daemon not running). Run with --repair to clean up.",
                );
            }
            checks.push(serde_json::json!({"check": "stale_daemon_json", "status": if repair { "repaired" } else { "warn" }}));
        }

        // --- Check 6: Database file ---
        let db_path = openfang_dir.join("data").join("openfang.db");
        if db_path.exists() {
            if let Ok(bytes) = std::fs::read(&db_path) {
                if bytes.len() >= 16 && bytes.starts_with(b"SQLite format 3") {
                    if !json {
                        ui::check_ok("Database file (valid SQLite)");
                    }
                    checks.push(serde_json::json!({"check": "database", "status": "ok"}));
                } else {
                    if !json {
                        ui::check_fail("Database file exists but is not valid SQLite");
                    }
                    checks.push(serde_json::json!({"check": "database", "status": "fail"}));
                    all_ok = false;
                }
            }
        } else {
            if !json {
                ui::check_warn("No database file (will be created on first run)");
            }
            checks.push(serde_json::json!({"check": "database", "status": "warn"}));
        }

        // --- Check 7: Disk space ---
        #[cfg(unix)]
        {
            if let Ok(output) = std::process::Command::new("df")
                .args(["-m", &openfang_dir.display().to_string()])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = stdout.lines().nth(1) {
                    let cols: Vec<&str> = line.split_whitespace().collect();
                    if cols.len() >= 4 {
                        if let Ok(available_mb) = cols[3].parse::<u64>() {
                            if available_mb < 100 {
                                if !json {
                                    ui::check_warn(&format!(
                                        "Low disk space: {available_mb}MB available"
                                    ));
                                }
                                checks.push(serde_json::json!({"check": "disk_space", "status": "warn", "available_mb": available_mb}));
                            } else {
                                if !json {
                                    ui::check_ok(&format!(
                                        "Disk space: {available_mb}MB available"
                                    ));
                                }
                                checks.push(serde_json::json!({"check": "disk_space", "status": "ok", "available_mb": available_mb}));
                            }
                        }
                    }
                }
            }
        }

        // --- Check 8: Agent manifests parse correctly ---
        let agents_dir = openfang_dir.join("agents");
        if agents_dir.exists() {
            let mut agent_errors = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&agents_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Err(e) = toml::from_str::<AgentManifest>(&content) {
                                agent_errors.push((
                                    path.file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string(),
                                    e.to_string(),
                                ));
                            }
                        }
                    }
                }
            }
            if agent_errors.is_empty() {
                if !json {
                    ui::check_ok("Agent manifests are valid");
                }
                checks.push(serde_json::json!({"check": "agent_manifests", "status": "ok"}));
            } else {
                for (file, err) in &agent_errors {
                    if !json {
                        ui::check_fail(&format!("Invalid manifest {file}: {err}"));
                    }
                }
                checks.push(serde_json::json!({"check": "agent_manifests", "status": "fail", "errors": agent_errors.len()}));
                all_ok = false;
            }
        }
    } else {
        if !json {
            ui::check_fail("Could not determine home directory");
        }
        checks.push(serde_json::json!({"check": "home_dir", "status": "fail"}));
        all_ok = false;
    }

    // --- LLM providers ---
    if !json {
        println!("\n  LLM Providers:");
    }
    let provider_keys = [
        ("GROQ_API_KEY", "Groq", "groq"),
        ("OPENROUTER_API_KEY", "OpenRouter", "openrouter"),
        ("ANTHROPIC_API_KEY", "Anthropic", "anthropic"),
        ("OPENAI_API_KEY", "OpenAI", "openai"),
        ("DEEPSEEK_API_KEY", "DeepSeek", "deepseek"),
        ("GEMINI_API_KEY", "Gemini", "gemini"),
        ("GOOGLE_API_KEY", "Google", "google"),
        ("TOGETHER_API_KEY", "Together", "together"),
        ("MISTRAL_API_KEY", "Mistral", "mistral"),
        ("FIREWORKS_API_KEY", "Fireworks", "fireworks"),
    ];

    let mut any_key_set = false;
    for (env_var, name, provider_id) in &provider_keys {
        let set = std::env::var(env_var).is_ok();
        if set {
            let valid = test_api_key(provider_id, env_var);
            if valid {
                if !json {
                    ui::provider_status(name, env_var, true);
                }
            } else if !json {
                ui::check_warn(&format!("{name} ({env_var}) - key rejected (401/403)"));
            }
            any_key_set = true;
            checks.push(serde_json::json!({"check": "provider", "name": name, "env_var": env_var, "status": if valid { "ok" } else { "warn" }, "live_test": !valid}));
        } else {
            if !json {
                ui::provider_status(name, env_var, false);
            }
            checks.push(serde_json::json!({"check": "provider", "name": name, "env_var": env_var, "status": "warn"}));
        }
    }

    if !any_key_set {
        if !json {
            println!();
            ui::check_fail("No LLM provider API keys found!");
            ui::blank();
            ui::section("Getting an API key (free tiers)");
            ui::suggest_cmd("Groq:", "https://console.groq.com       (free, fast)");
            ui::suggest_cmd("Gemini:", "https://aistudio.google.com    (free tier)");
            ui::suggest_cmd("DeepSeek:", "https://platform.deepseek.com  (low cost)");
            ui::blank();
            ui::hint("Or run: openfang config set-key groq");
        }
        all_ok = false;
    }

    // --- Check 10: Channel token format validation ---
    if !json {
        println!("\n  Channel Integrations:");
    }
    let channel_keys = [
        ("TELEGRAM_BOT_TOKEN", "Telegram"),
        ("DISCORD_BOT_TOKEN", "Discord"),
        ("SLACK_APP_TOKEN", "Slack App"),
        ("SLACK_BOT_TOKEN", "Slack Bot"),
    ];
    for (env_var, name) in &channel_keys {
        let set = std::env::var(env_var).is_ok();
        if set {
            let val = std::env::var(env_var).unwrap_or_default();
            let format_ok = match *env_var {
                "TELEGRAM_BOT_TOKEN" => val.contains(':'),
                "DISCORD_BOT_TOKEN" => val.len() > 50,
                "SLACK_APP_TOKEN" => val.starts_with("xapp-"),
                "SLACK_BOT_TOKEN" => val.starts_with("xoxb-"),
                _ => true,
            };
            if format_ok {
                if !json {
                    ui::provider_status(name, env_var, true);
                }
            } else if !json {
                ui::check_warn(&format!("{name} ({env_var}) - unexpected token format"));
            }
            checks.push(serde_json::json!({"check": "channel", "name": name, "env_var": env_var, "status": if format_ok { "ok" } else { "warn" }}));
        } else {
            if !json {
                ui::provider_status(name, env_var, false);
            }
            checks.push(serde_json::json!({"check": "channel", "name": name, "env_var": env_var, "status": "warn"}));
        }
    }

    // --- Check 11: .env keys vs config api_key_env consistency ---
    if let Some(ref h) = home {
        let openfang_dir = h.join(".openfang");
        let config_path = openfang_dir.join("config.toml");
        if config_path.exists() {
            let config_str = std::fs::read_to_string(&config_path).unwrap_or_default();
            for line in config_str.lines() {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("api_key_env") {
                    if let Some(val_part) = rest.strip_prefix('=') {
                        let val = val_part.trim().trim_matches('"');
                        if !val.is_empty() && std::env::var(val).is_err() {
                            if !json {
                                ui::check_warn(&format!(
                                    "Config references {val} but it is not set in env or .env"
                                ));
                            }
                            checks.push(serde_json::json!({"check": "env_consistency", "status": "warn", "missing_var": val}));
                        }
                    }
                }
            }
        }
    }

    // --- Check 12: Config deserialization into KernelConfig ---
    if let Some(ref h) = home {
        let openfang_dir = h.join(".openfang");
        let config_path = openfang_dir.join("config.toml");
        if config_path.exists() {
            if !json {
                println!("\n  Config Validation:");
            }
            let config_content = std::fs::read_to_string(&config_path).unwrap_or_default();
            match toml::from_str::<openfang_types::config::KernelConfig>(&config_content) {
                Ok(cfg) => {
                    if !json {
                        ui::check_ok("Config deserializes into KernelConfig");
                    }
                    checks.push(serde_json::json!({"check": "config_deser", "status": "ok"}));

                    let mode = format!("{:?}", cfg.exec_policy.mode);
                    let safe_bins_count = cfg.exec_policy.safe_bins.len();
                    if !json {
                        ui::check_ok(&format!(
                            "Exec policy: mode={mode}, safe_bins={safe_bins_count}"
                        ));
                    }
                    checks.push(serde_json::json!({"check": "exec_policy", "status": "ok", "mode": mode, "safe_bins": safe_bins_count}));

                    if !cfg.include.is_empty() {
                        let mut include_ok = true;
                        for inc in &cfg.include {
                            let inc_path = openfang_dir.join(inc);
                            if inc_path.exists() {
                                if !json {
                                    ui::check_ok(&format!("Include file: {inc}"));
                                }
                            } else if repair {
                                if !json {
                                    ui::check_warn(&format!("Include file missing: {inc}"));
                                }
                                include_ok = false;
                            } else {
                                if !json {
                                    ui::check_fail(&format!("Include file not found: {inc}"));
                                }
                                include_ok = false;
                                all_ok = false;
                            }
                        }
                        checks.push(serde_json::json!({"check": "config_includes", "status": if include_ok { "ok" } else { "fail" }, "count": cfg.include.len()}));
                    }

                    if !cfg.mcp_servers.is_empty() {
                        let mcp_count = cfg.mcp_servers.len();
                        if !json {
                            ui::check_ok(&format!("MCP servers configured: {mcp_count}"));
                        }
                        for server in &cfg.mcp_servers {
                            match &server.transport {
                                openfang_types::config::McpTransportEntry::Stdio {
                                    command,
                                    ..
                                } => {
                                    if command.is_empty() {
                                        if !json {
                                            ui::check_warn(&format!(
                                                "MCP server '{}' has empty command",
                                                server.name
                                            ));
                                        }
                                        checks.push(serde_json::json!({"check": "mcp_server_config", "status": "warn", "name": server.name}));
                                    }
                                }
                                openfang_types::config::McpTransportEntry::Sse { url } => {
                                    if url.is_empty() {
                                        if !json {
                                            ui::check_warn(&format!(
                                                "MCP server '{}' has empty URL",
                                                server.name
                                            ));
                                        }
                                        checks.push(serde_json::json!({"check": "mcp_server_config", "status": "warn", "name": server.name}));
                                    }
                                }
                            }
                        }
                        checks.push(serde_json::json!({"check": "mcp_servers", "status": "ok", "count": mcp_count}));
                    }
                }
                Err(e) => {
                    if !json {
                        ui::check_fail(&format!("Config fails KernelConfig deserialization: {e}"));
                    }
                    checks.push(serde_json::json!({"check": "config_deser", "status": "fail", "error": e.to_string()}));
                    all_ok = false;
                }
            }
        }
    }

    // --- Check 13: Skill registry health ---
    {
        if !json {
            println!("\n  Skills:");
        }
        let skills_dir = home
            .as_ref()
            .map(|h| h.join(".openfang").join("skills"))
            .unwrap_or_else(|| std::path::PathBuf::from("skills"));
        let mut skill_reg = openfang_skills::registry::SkillRegistry::new(skills_dir.clone());
        skill_reg.load_bundled();
        let bundled_count = skill_reg.count();
        if !json {
            ui::check_ok(&format!("Bundled skills loaded: {bundled_count}"));
        }
        checks.push(
            serde_json::json!({"check": "bundled_skills", "status": "ok", "count": bundled_count}),
        );

        if skills_dir.exists() {
            match skill_reg.load_workspace_skills(&skills_dir) {
                Ok(_) => {
                    let total = skill_reg.count();
                    let ws_count = total.saturating_sub(bundled_count);
                    if ws_count > 0 {
                        if !json {
                            ui::check_ok(&format!("Workspace skills loaded: {ws_count}"));
                        }
                        checks.push(serde_json::json!({"check": "workspace_skills", "status": "ok", "count": ws_count}));
                    }
                }
                Err(e) => {
                    if !json {
                        ui::check_warn(&format!("Failed to load workspace skills: {e}"));
                    }
                    checks.push(serde_json::json!({"check": "workspace_skills", "status": "warn", "error": e.to_string()}));
                }
            }
        }

        let skills = skill_reg.list();
        let mut injection_warnings = 0;
        for skill in &skills {
            if let Some(ref prompt) = skill.manifest.prompt_context {
                let warnings = openfang_skills::verify::SkillVerifier::scan_prompt_content(prompt);
                if !warnings.is_empty() {
                    injection_warnings += 1;
                    if !json {
                        ui::check_warn(&format!(
                            "Prompt injection warning in skill: {}",
                            skill.manifest.skill.name
                        ));
                    }
                }
            }
        }
        if injection_warnings > 0 {
            checks.push(serde_json::json!({"check": "skill_injection_scan", "status": "warn", "warnings": injection_warnings}));
        } else {
            if !json {
                ui::check_ok("All skills pass prompt injection scan");
            }
            checks.push(serde_json::json!({"check": "skill_injection_scan", "status": "ok"}));
        }
    }

    // --- Check 14: Extension registry health ---
    if let Some(ref h) = home {
        if !json {
            println!("\n  Extensions:");
        }
        let openfang_dir = h.join(".openfang");
        let mut ext_registry =
            openfang_extensions::registry::IntegrationRegistry::new(&openfang_dir);
        ext_registry.load_bundled();
        let _ = ext_registry.load_installed();
        let template_count = ext_registry.template_count();
        let installed_count = ext_registry.installed_count();
        if !json {
            ui::check_ok(&format!(
                "Available integration templates: {template_count}"
            ));
            ui::check_ok(&format!("Installed integrations: {installed_count}"));
        }
        checks.push(serde_json::json!({"check": "extensions_available", "status": "ok", "count": template_count}));
        checks.push(serde_json::json!({"check": "extensions_installed", "status": "ok", "count": installed_count}));
    }

    // --- Check 15: Daemon health detail (if running) ---
    if let Some(ref base) = find_daemon() {
        if !json {
            println!("\n  Daemon Health:");
        }
        let client = daemon_client();
        match client.get(format!("{base}/api/health/detail")).send() {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(body) = resp.json::<serde_json::Value>() {
                    if let Some(agents) = body.get("agent_count").and_then(|v| v.as_u64()) {
                        if !json {
                            ui::check_ok(&format!("Running agents: {agents}"));
                        }
                        checks.push(serde_json::json!({"check": "daemon_agents", "status": "ok", "count": agents}));
                    }
                    if let Some(uptime) = body.get("uptime_secs").and_then(|v| v.as_u64()) {
                        let hours = uptime / 3600;
                        let mins = (uptime % 3600) / 60;
                        if !json {
                            ui::check_ok(&format!("Daemon uptime: {hours}h {mins}m"));
                        }
                        checks.push(serde_json::json!({"check": "daemon_uptime", "status": "ok", "secs": uptime}));
                    }
                    if let Some(db_status) = body.get("database").and_then(|v| v.as_str()) {
                        if db_status == "ok" {
                            if !json {
                                ui::check_ok("Database connectivity: OK");
                            }
                        } else {
                            if !json {
                                ui::check_fail(&format!("Database status: {db_status}"));
                            }
                            all_ok = false;
                        }
                        checks.push(serde_json::json!({"check": "daemon_db", "status": db_status}));
                    }
                }
            }
            Ok(resp) => {
                if !json {
                    ui::check_warn(&format!("Health detail returned {}", resp.status()));
                }
                checks.push(serde_json::json!({"check": "daemon_health", "status": "warn"}));
            }
            Err(e) => {
                if !json {
                    ui::check_warn(&format!("Failed to query daemon health: {e}"));
                }
                checks.push(serde_json::json!({"check": "daemon_health", "status": "warn", "error": e.to_string()}));
            }
        }

        match client.get(format!("{base}/api/skills")).send() {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(body) = resp.json::<serde_json::Value>() {
                    if let Some(arr) = body.as_array() {
                        if !json {
                            ui::check_ok(&format!("Skills loaded in daemon: {}", arr.len()));
                        }
                        checks.push(serde_json::json!({"check": "daemon_skills", "status": "ok", "count": arr.len()}));
                    }
                }
            }
            _ => {}
        }

        match client.get(format!("{base}/api/mcp/servers")).send() {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(body) = resp.json::<serde_json::Value>() {
                    if let Some(arr) = body.as_array() {
                        let connected = arr
                            .iter()
                            .filter(|s| {
                                s.get("connected")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false)
                            })
                            .count();
                        if !json {
                            ui::check_ok(&format!(
                                "MCP servers: {} configured, {} connected",
                                arr.len(),
                                connected
                            ));
                        }
                        checks.push(serde_json::json!({"check": "daemon_mcp", "status": "ok", "configured": arr.len(), "connected": connected}));
                    }
                }
            }
            _ => {}
        }

        match client.get(format!("{base}/api/integrations/health")).send() {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(body) = resp.json::<serde_json::Value>() {
                    if let Some(obj) = body.as_object() {
                        let healthy = obj
                            .values()
                            .filter(|v| v.get("healthy").and_then(|h| h.as_bool()).unwrap_or(false))
                            .count();
                        let total = obj.len();
                        if healthy == total {
                            if !json {
                                ui::check_ok(&format!(
                                    "Integration health: {healthy}/{total} healthy"
                                ));
                            }
                        } else if !json {
                            ui::check_warn(&format!(
                                "Integration health: {healthy}/{total} healthy"
                            ));
                        }
                        checks.push(serde_json::json!({"check": "integration_health", "status": if healthy == total { "ok" } else { "warn" }, "healthy": healthy, "total": total}));
                    }
                }
            }
            _ => {}
        }
    }

    if !json {
        println!();
    }
    match std::process::Command::new("rustc")
        .arg("--version")
        .output()
    {
        Ok(output) => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !json {
                ui::check_ok(&format!("Rust: {version}"));
            }
            checks.push(serde_json::json!({"check": "rust", "status": "ok", "version": version}));
        }
        Err(_) => {
            if !json {
                ui::check_fail("Rust toolchain not found");
            }
            checks.push(serde_json::json!({"check": "rust", "status": "fail"}));
            all_ok = false;
        }
    }

    match std::process::Command::new("python3")
        .arg("--version")
        .output()
    {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !json {
                ui::check_ok(&format!("Python: {version}"));
            }
            checks.push(serde_json::json!({"check": "python", "status": "ok", "version": version}));
        }
        _ => {
            match std::process::Command::new("python")
                .arg("--version")
                .output()
            {
                Ok(output) if output.status.success() => {
                    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !json {
                        ui::check_ok(&format!("Python: {version}"));
                    }
                    checks.push(
                        serde_json::json!({"check": "python", "status": "ok", "version": version}),
                    );
                }
                _ => {
                    if !json {
                        ui::check_warn("Python not found (needed for Python skill runtime)");
                    }
                    checks.push(serde_json::json!({"check": "python", "status": "warn"}));
                }
            }
        }
    }

    match std::process::Command::new("node").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !json {
                ui::check_ok(&format!("Node.js: {version}"));
            }
            checks.push(serde_json::json!({"check": "node", "status": "ok", "version": version}));
        }
        _ => {
            if !json {
                ui::check_warn("Node.js not found (needed for Node skill runtime)");
            }
            checks.push(serde_json::json!({"check": "node", "status": "warn"}));
        }
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "all_ok": all_ok,
                "checks": checks,
            }))
            .unwrap_or_default()
        );
    } else {
        println!();
        if all_ok {
            ui::success("All checks passed! OpenFang is ready.");
            ui::hint("Start the daemon: openfang start");
        } else if repaired {
            ui::success("Repairs applied. Re-run `openfang doctor` to verify.");
        } else {
            ui::error("Some checks failed.");
            if !repair {
                ui::hint("Run `openfang doctor --repair` to attempt auto-fix");
            }
        }
    }
}

pub fn cmd_dashboard() {
    let base = if let Some(url) = find_daemon() {
        url
    } else {
        ui::hint("No daemon running — starting one now...");
        match start_daemon_background() {
            Ok(url) => {
                ui::success("Daemon started");
                url
            }
            Err(e) => {
                ui::error_with_fix(
                    &format!("Could not start daemon: {e}"),
                    "Start it manually: openfang start",
                );
                std::process::exit(1);
            }
        }
    };

    let url = format!("{base}/");
    ui::success(&format!("Opening dashboard at {url}"));
    if copy_to_clipboard(&url) {
        ui::hint("URL copied to clipboard");
    }
    if !open_in_browser(&url) {
        ui::hint(&format!("Could not open browser. Visit: {url}"));
    }
}

pub fn cmd_completion(shell: clap_complete::Shell) {
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "openfang", &mut std::io::stdout());
}

pub fn cmd_sessions(agent: Option<&str>, json: bool) {
    let base = require_daemon("sessions");
    let client = daemon_client();
    let url = match agent {
        Some(a) => format!("{base}/api/sessions?agent={a}"),
        None => format!("{base}/api/sessions"),
    };
    let body = daemon_json(client.get(&url).send());
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
        return;
    }
    if let Some(arr) = body.as_array() {
        if arr.is_empty() {
            println!("No sessions found.");
            return;
        }
        println!("{:<38} {:<16} {:<8} LAST ACTIVE", "ID", "AGENT", "MSGS");
        println!("{}", "-".repeat(80));
        for s in arr {
            println!(
                "{:<38} {:<16} {:<8} {}",
                s["id"].as_str().unwrap_or("?"),
                s["agent_name"].as_str().unwrap_or("?"),
                s["message_count"].as_u64().unwrap_or(0),
                s["last_active"].as_str().unwrap_or("?"),
            );
        }
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
    }
}

pub fn cmd_logs(lines: usize, follow: bool) {
    let log_path = dirs::home_dir()
        .map(|h| h.join(".openfang").join("tui.log"))
        .unwrap_or_else(|| PathBuf::from("tui.log"));

    if !log_path.exists() {
        ui::error_with_fix(
            "Log file not found",
            &format!("Expected at: {}", log_path.display()),
        );
        std::process::exit(1);
    }

    if follow {
        #[cfg(unix)]
        {
            let _ = std::process::Command::new("tail")
                .args(["-f", "-n", &lines.to_string()])
                .arg(&log_path)
                .status();
        }
        #[cfg(windows)]
        {
            let content = std::fs::read_to_string(&log_path).unwrap_or_default();
            let all_lines: Vec<&str> = content.lines().collect();
            let start = all_lines.len().saturating_sub(lines);
            for line in &all_lines[start..] {
                println!("{line}");
            }
            println!("--- Following {} (Ctrl+C to stop) ---", log_path.display());
            let mut last_len = content.len();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(500));
                if let Ok(new_content) = std::fs::read_to_string(&log_path) {
                    if new_content.len() > last_len {
                        print!("{}", &new_content[last_len..]);
                        last_len = new_content.len();
                    }
                }
            }
        }
    } else {
        let content = std::fs::read_to_string(&log_path).unwrap_or_default();
        let all_lines: Vec<&str> = content.lines().collect();
        let start = all_lines.len().saturating_sub(lines);
        for line in &all_lines[start..] {
            println!("{line}");
        }
    }
}

pub fn cmd_health(json: bool) {
    match find_daemon() {
        Some(base) => {
            let client = daemon_client();
            let body = daemon_json(client.get(format!("{base}/api/health")).send());
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&body).unwrap_or_default()
                );
                return;
            }
            ui::success("Daemon is healthy");
            if let Some(status) = body["status"].as_str() {
                ui::kv("Status", status);
            }
            if let Some(uptime) = body.get("uptime_secs").and_then(|v| v.as_u64()) {
                let hours = uptime / 3600;
                let mins = (uptime % 3600) / 60;
                ui::kv("Uptime", &format!("{hours}h {mins}m"));
            }
        }
        None => {
            if json {
                println!("{}", serde_json::json!({"error": "daemon not running"}));
                std::process::exit(1);
            }
            ui::error("Daemon is not running.");
            ui::hint("Start it with: openfang start");
            std::process::exit(1);
        }
    }
}

pub fn cmd_system_info(json: bool) {
    if let Some(base) = find_daemon() {
        let client = daemon_client();
        let body = daemon_json(client.get(format!("{base}/api/status")).send());
        if json {
            let mut data = body.clone();
            if let Some(obj) = data.as_object_mut() {
                obj.insert(
                    "version".to_string(),
                    serde_json::json!(env!("CARGO_PKG_VERSION")),
                );
                obj.insert("api_url".to_string(), serde_json::json!(base));
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&data).unwrap_or_default()
            );
            return;
        }
        ui::section("OpenFang System Info");
        ui::blank();
        ui::kv("Version", env!("CARGO_PKG_VERSION"));
        ui::kv("Status", body["status"].as_str().unwrap_or("?"));
        ui::kv(
            "Agents",
            &body["agent_count"].as_u64().unwrap_or(0).to_string(),
        );
        ui::kv("Provider", body["default_provider"].as_str().unwrap_or("?"));
        ui::kv("Model", body["default_model"].as_str().unwrap_or("?"));
        ui::kv("API", &base);
        ui::kv("Data dir", body["data_dir"].as_str().unwrap_or("?"));
        ui::kv(
            "Uptime",
            &format!("{}s", body["uptime_seconds"].as_u64().unwrap_or(0)),
        );
    } else {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "version": env!("CARGO_PKG_VERSION"),
                    "daemon": "not_running",
                })
            );
            return;
        }
        ui::section("OpenFang System Info");
        ui::blank();
        ui::kv("Version", env!("CARGO_PKG_VERSION"));
        ui::kv_warn("Daemon", "NOT RUNNING");
        ui::hint("Start with: openfang start");
    }
}

pub fn cmd_system_version(json: bool) {
    if json {
        println!(
            "{}",
            serde_json::json!({"version": env!("CARGO_PKG_VERSION")})
        );
        return;
    }
    println!("openfang {}", env!("CARGO_PKG_VERSION"));
}
