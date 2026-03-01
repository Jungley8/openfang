//! Skill, channel, integration add/remove, scaffold, approvals, cron, security, memory, devices, webhooks.

use crate::cli::ScaffoldKind;
use crate::daemon::{
    daemon_client, daemon_json, find_daemon, require_daemon, restrict_file_permissions,
};
use crate::ui;
use crate::{copy_dir_recursive, openfang_home, prompt_input};
use colored::Colorize;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Skill commands
// ---------------------------------------------------------------------------

pub fn cmd_skill_install(source: &str) {
    let home = openfang_home();
    let skills_dir = home.join("skills");
    std::fs::create_dir_all(&skills_dir).unwrap_or_else(|e| {
        eprintln!("Error creating skills directory: {e}");
        std::process::exit(1);
    });

    let source_path = PathBuf::from(source);
    if source_path.exists() && source_path.is_dir() {
        let manifest_path = source_path.join("skill.toml");
        if !manifest_path.exists() {
            if openfang_skills::openclaw_compat::detect_openclaw_skill(&source_path) {
                println!("Detected OpenClaw skill format. Converting...");
                match openfang_skills::openclaw_compat::convert_openclaw_skill(&source_path) {
                    Ok(manifest) => {
                        let dest = skills_dir.join(&manifest.skill.name);
                        copy_dir_recursive(&source_path, &dest);
                        if let Err(e) = openfang_skills::openclaw_compat::write_openfang_manifest(
                            &dest, &manifest,
                        ) {
                            eprintln!("Failed to write manifest: {e}");
                            std::process::exit(1);
                        }
                        println!("Installed OpenClaw skill: {}", manifest.skill.name);
                    }
                    Err(e) => {
                        eprintln!("Failed to convert OpenClaw skill: {e}");
                        std::process::exit(1);
                    }
                }
                return;
            }
            eprintln!("No skill.toml found in {source}");
            std::process::exit(1);
        }

        let toml_str = std::fs::read_to_string(&manifest_path).unwrap_or_else(|e| {
            eprintln!("Error reading skill.toml: {e}");
            std::process::exit(1);
        });
        let manifest: openfang_skills::SkillManifest =
            toml::from_str(&toml_str).unwrap_or_else(|e| {
                eprintln!("Error parsing skill.toml: {e}");
                std::process::exit(1);
            });

        let dest = skills_dir.join(&manifest.skill.name);
        copy_dir_recursive(&source_path, &dest);
        println!(
            "Installed skill: {} v{}",
            manifest.skill.name, manifest.skill.version
        );
    } else {
        println!("Installing {source} from FangHub...");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = openfang_skills::marketplace::MarketplaceClient::new(
            openfang_skills::marketplace::MarketplaceConfig::default(),
        );
        match rt.block_on(client.install(source, &skills_dir)) {
            Ok(version) => println!("Installed {source} {version}"),
            Err(e) => {
                eprintln!("Failed to install skill: {e}");
                std::process::exit(1);
            }
        }
    }
}

pub fn cmd_skill_list() {
    let home = openfang_home();
    let skills_dir = home.join("skills");

    let mut registry = openfang_skills::registry::SkillRegistry::new(skills_dir);
    match registry.load_all() {
        Ok(0) => println!("No skills installed."),
        Ok(count) => {
            println!("{count} skill(s) installed:\n");
            println!(
                "{:<20} {:<10} {:<8} DESCRIPTION",
                "NAME", "VERSION", "TOOLS"
            );
            println!("{}", "-".repeat(70));
            for skill in registry.list() {
                println!(
                    "{:<20} {:<10} {:<8} {}",
                    skill.manifest.skill.name,
                    skill.manifest.skill.version,
                    skill.manifest.tools.provided.len(),
                    skill.manifest.skill.description,
                );
            }
        }
        Err(e) => {
            eprintln!("Error loading skills: {e}");
            std::process::exit(1);
        }
    }
}

pub fn cmd_skill_remove(name: &str) {
    let home = openfang_home();
    let skills_dir = home.join("skills");

    let mut registry = openfang_skills::registry::SkillRegistry::new(skills_dir);
    let _ = registry.load_all();
    match registry.remove(name) {
        Ok(()) => println!("Removed skill: {name}"),
        Err(e) => {
            eprintln!("Failed to remove skill: {e}");
            std::process::exit(1);
        }
    }
}

pub fn cmd_skill_search(query: &str) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let client = openfang_skills::marketplace::MarketplaceClient::new(
        openfang_skills::marketplace::MarketplaceConfig::default(),
    );
    match rt.block_on(client.search(query)) {
        Ok(results) if results.is_empty() => println!("No skills found for \"{query}\"."),
        Ok(results) => {
            println!("Skills matching \"{query}\":\n");
            for r in results {
                println!("  {} ({})", r.name, r.stars);
                if !r.description.is_empty() {
                    println!("    {}", r.description);
                }
                println!("    {}", r.url);
                println!();
            }
        }
        Err(e) => {
            eprintln!("Search failed: {e}");
            std::process::exit(1);
        }
    }
}

pub fn cmd_skill_create() {
    let name = prompt_input("Skill name: ");
    let description = prompt_input("Description: ");
    let runtime = prompt_input("Runtime (python/node/wasm) [python]: ");
    let runtime = if runtime.is_empty() {
        "python".to_string()
    } else {
        runtime
    };

    let home = openfang_home();
    let skill_dir = home.join("skills").join(&name);
    std::fs::create_dir_all(skill_dir.join("src")).unwrap_or_else(|e| {
        eprintln!("Error creating skill directory: {e}");
        std::process::exit(1);
    });

    let manifest = format!(
        r#"[skill]
name = "{name}"
version = "0.1.0"
description = "{description}"
author = ""
license = "MIT"
tags = []

[runtime]
type = "{runtime}"
entry = "src/main.py"

[[tools.provided]]
name = "{tool_name}"
description = "{description}"
input_schema = {{ type = "object", properties = {{ input = {{ type = "string" }} }}, required = ["input"] }}

[requirements]
tools = []
capabilities = []
"#,
        tool_name = name.replace('-', "_"),
    );

    std::fs::write(skill_dir.join("skill.toml"), &manifest).unwrap();

    let entry_content = match runtime.as_str() {
        "python" => format!(
            r#"#!/usr/bin/env python3
"""OpenFang skill: {name}"""
import json
import sys

def main():
    payload = json.loads(sys.stdin.read())
    tool_name = payload["tool"]
    input_data = payload["input"]

    # TODO: Implement your skill logic here
    result = {{"result": f"Processed: {{input_data.get('input', '')}}"}}

    print(json.dumps(result))

if __name__ == "__main__":
    main()
"#
        ),
        _ => "// TODO: Implement your skill\n".to_string(),
    };

    let entry_path = if runtime == "python" {
        "src/main.py"
    } else {
        "src/index.js"
    };
    std::fs::write(skill_dir.join(entry_path), entry_content).unwrap();

    println!("\nSkill created: {}", skill_dir.display());
    println!("\nFiles:");
    println!("  skill.toml");
    println!("  {entry_path}");
    println!("\nNext steps:");
    println!("  1. Edit the entry point to implement your skill logic");
    println!("  2. Test locally: openfang skill test");
    println!(
        "  3. Install: openfang skill install {}",
        skill_dir.display()
    );
}

// ---------------------------------------------------------------------------
// Channel commands
// ---------------------------------------------------------------------------

pub fn cmd_channel_list() {
    let home = openfang_home();
    let config_path = home.join("config.toml");

    if !config_path.exists() {
        println!("No configuration found. Run `openfang init` first.");
        return;
    }

    let config_str = std::fs::read_to_string(&config_path).unwrap_or_default();

    println!("Channel Integrations:\n");
    println!("{:<12} {:<10} STATUS", "CHANNEL", "ENV VAR");
    println!("{}", "-".repeat(50));

    let channels: Vec<(&str, &str)> = vec![
        ("webchat", ""),
        ("telegram", "TELEGRAM_BOT_TOKEN"),
        ("discord", "DISCORD_BOT_TOKEN"),
        ("slack", "SLACK_BOT_TOKEN"),
        ("whatsapp", "WA_ACCESS_TOKEN"),
        ("signal", ""),
        ("matrix", "MATRIX_TOKEN"),
        ("email", "EMAIL_PASSWORD"),
    ];

    for (name, env_var) in channels {
        let configured = config_str.contains(&format!("[channels.{name}]"));
        let env_set = if env_var.is_empty() {
            true
        } else {
            std::env::var(env_var).is_ok()
        };
        let status = match (configured, env_set) {
            (true, true) => "Ready",
            (true, false) => "Missing env",
            (false, _) => "Not configured",
        };
        println!(
            "{:<12} {:<10} {}",
            name,
            if env_var.is_empty() { "—" } else { env_var },
            status,
        );
    }

    println!("\nUse `openfang channel setup <channel>` to configure a channel.");
}

fn maybe_write_channel_config(channel: &str, config_block: &str) {
    let home = openfang_home();
    let config_path = home.join("config.toml");

    if !config_path.exists() {
        ui::hint("No config.toml found. Run `openfang init` first.");
        return;
    }

    let existing = std::fs::read_to_string(&config_path).unwrap_or_default();
    let section_header = format!("[channels.{channel}]");
    if existing.contains(&section_header) {
        ui::check_ok(&format!("{section_header} already in config.toml"));
        return;
    }

    let answer = prompt_input("  Write to config.toml? [Y/n] ");
    if answer.is_empty() || answer.starts_with('y') || answer.starts_with('Y') {
        let mut content = existing;
        content.push_str(config_block);
        if std::fs::write(&config_path, &content).is_ok() {
            restrict_file_permissions(&config_path);
            ui::check_ok(&format!("Added {section_header} to config.toml"));
        } else {
            ui::check_fail("Failed to write config.toml");
        }
    }
}

fn notify_daemon_restart() {
    if find_daemon().is_some() {
        ui::check_warn("Restart the daemon to activate this channel");
    } else {
        ui::hint("Start the daemon: openfang start");
    }
}

pub fn cmd_channel_setup(channel: Option<&str>) {
    let channel = match channel {
        Some(c) => c.to_string(),
        None => {
            ui::section("Channel Setup");
            ui::blank();
            let channel_list = [
                ("telegram", "Telegram bot (BotFather)"),
                ("discord", "Discord bot"),
                ("slack", "Slack app (Socket Mode)"),
                ("whatsapp", "WhatsApp Cloud API"),
                ("email", "Email (IMAP/SMTP)"),
                ("signal", "Signal (signal-cli)"),
                ("matrix", "Matrix homeserver"),
            ];

            for (i, (name, desc)) in channel_list.iter().enumerate() {
                println!("    {:>2}. {:<12} {}", i + 1, name, desc.dimmed());
            }
            ui::blank();

            let choice = prompt_input("  Choose channel [1]: ");
            let idx = if choice.is_empty() {
                0
            } else {
                choice
                    .parse::<usize>()
                    .unwrap_or(1)
                    .saturating_sub(1)
                    .min(channel_list.len() - 1)
            };
            channel_list[idx].0.to_string()
        }
    };

    match channel.as_str() {
        "telegram" => {
            ui::section("Setting up Telegram");
            ui::blank();
            println!("  1. Open Telegram and message @BotFather");
            println!("  2. Send /newbot and follow the prompts");
            println!("  3. Copy the bot token");
            ui::blank();

            let token = prompt_input("  Paste your bot token: ");
            if token.is_empty() {
                ui::error("No token provided. Setup cancelled.");
                return;
            }

            let config_block = "\n[channels.telegram]\nbot_token_env = \"TELEGRAM_BOT_TOKEN\"\ndefault_agent = \"assistant\"\n";
            maybe_write_channel_config("telegram", config_block);

            match crate::dotenv::save_env_key("TELEGRAM_BOT_TOKEN", &token) {
                Ok(()) => ui::success("Token saved to ~/.openfang/.env"),
                Err(_) => println!("    export TELEGRAM_BOT_TOKEN={token}"),
            }

            ui::blank();
            ui::success("Telegram configured");
            notify_daemon_restart();
        }
        "discord" => {
            ui::section("Setting up Discord");
            ui::blank();
            println!("  1. Go to https://discord.com/developers/applications");
            println!("  2. Create a New Application");
            println!("  3. Go to Bot section and click 'Add Bot'");
            println!("  4. Copy the bot token");
            println!("  5. Under Privileged Gateway Intents, enable:");
            println!("     - Message Content Intent");
            println!("  6. Use OAuth2 URL Generator to invite bot to your server");
            ui::blank();

            let token = prompt_input("  Paste your bot token: ");
            if token.is_empty() {
                ui::error("No token provided. Setup cancelled.");
                return;
            }

            let config_block = "\n[channels.discord]\nbot_token_env = \"DISCORD_BOT_TOKEN\"\ndefault_agent = \"coder\"\n";
            maybe_write_channel_config("discord", config_block);

            match crate::dotenv::save_env_key("DISCORD_BOT_TOKEN", &token) {
                Ok(()) => ui::success("Token saved to ~/.openfang/.env"),
                Err(_) => println!("    export DISCORD_BOT_TOKEN={token}"),
            }

            ui::blank();
            ui::success("Discord configured");
            notify_daemon_restart();
        }
        "slack" => {
            ui::section("Setting up Slack");
            ui::blank();
            println!("  1. Go to https://api.slack.com/apps");
            println!("  2. Create New App -> From Scratch");
            println!("  3. Enable Socket Mode (Settings -> Socket Mode)");
            println!("  4. Copy the App-Level Token (xapp-...)");
            println!("  5. Go to OAuth & Permissions, add scopes:");
            println!("     - chat:write, app_mentions:read, im:history");
            println!("  6. Install to workspace and copy Bot Token (xoxb-...)");
            ui::blank();

            let app_token = prompt_input("  Paste your App Token (xapp-...): ");
            let bot_token = prompt_input("  Paste your Bot Token (xoxb-...): ");

            let config_block = "\n[channels.slack]\napp_token_env = \"SLACK_APP_TOKEN\"\nbot_token_env = \"SLACK_BOT_TOKEN\"\ndefault_agent = \"assistant\"\n";
            maybe_write_channel_config("slack", config_block);

            if !app_token.is_empty() {
                match crate::dotenv::save_env_key("SLACK_APP_TOKEN", &app_token) {
                    Ok(()) => ui::success("App token saved to ~/.openfang/.env"),
                    Err(_) => println!("    export SLACK_APP_TOKEN={app_token}"),
                }
            }
            if !bot_token.is_empty() {
                match crate::dotenv::save_env_key("SLACK_BOT_TOKEN", &bot_token) {
                    Ok(()) => ui::success("Bot token saved to ~/.openfang/.env"),
                    Err(_) => println!("    export SLACK_BOT_TOKEN={bot_token}"),
                }
            }

            ui::blank();
            ui::success("Slack configured");
            notify_daemon_restart();
        }
        "whatsapp" => {
            ui::section("Setting up WhatsApp");
            ui::blank();
            println!("  WhatsApp Cloud API (recommended for production):");
            println!("  1. Go to https://developers.facebook.com");
            println!("  2. Create a Business App");
            println!("  3. Add WhatsApp product");
            println!("  4. Set up a test phone number");
            println!("  5. Copy Phone Number ID and Access Token");
            ui::blank();

            let phone_id = prompt_input("  Phone Number ID: ");
            let access_token = prompt_input("  Access Token: ");
            let verify_token = prompt_input("  Verify Token: ");

            let config_block = "\n[channels.whatsapp]\nmode = \"cloud_api\"\nphone_number_id_env = \"WA_PHONE_ID\"\naccess_token_env = \"WA_ACCESS_TOKEN\"\nverify_token_env = \"WA_VERIFY_TOKEN\"\nwebhook_port = 8443\ndefault_agent = \"assistant\"\n";
            maybe_write_channel_config("whatsapp", config_block);

            for (key, val) in [
                ("WA_PHONE_ID", &phone_id),
                ("WA_ACCESS_TOKEN", &access_token),
                ("WA_VERIFY_TOKEN", &verify_token),
            ] {
                if !val.is_empty() {
                    match crate::dotenv::save_env_key(key, val) {
                        Ok(()) => ui::success(&format!("{key} saved to ~/.openfang/.env")),
                        Err(_) => println!("    export {key}={val}"),
                    }
                }
            }

            ui::blank();
            ui::success("WhatsApp configured");
            notify_daemon_restart();
        }
        "email" => {
            ui::section("Setting up Email");
            ui::blank();
            println!("  For Gmail, use an App Password:");
            println!("  https://myaccount.google.com/apppasswords");
            ui::blank();

            let username = prompt_input("  Email address: ");
            if username.is_empty() {
                ui::error("No email provided. Setup cancelled.");
                return;
            }

            let password = prompt_input("  App password (or Enter to set later): ");

            let config_block = format!(
                "\n[channels.email]\nimap_host = \"imap.gmail.com\"\nimap_port = 993\nsmtp_host = \"smtp.gmail.com\"\nsmtp_port = 587\nusername = \"{username}\"\npassword_env = \"EMAIL_PASSWORD\"\npoll_interval = 30\ndefault_agent = \"assistant\"\n"
            );
            maybe_write_channel_config("email", &config_block);

            if !password.is_empty() {
                match crate::dotenv::save_env_key("EMAIL_PASSWORD", &password) {
                    Ok(()) => ui::success("Password saved to ~/.openfang/.env"),
                    Err(_) => println!("    export EMAIL_PASSWORD=your_app_password"),
                }
            } else {
                ui::hint("Set later: openfang config set-key email (or export EMAIL_PASSWORD=...)");
            }

            ui::blank();
            ui::success("Email configured");
            notify_daemon_restart();
        }
        "signal" => {
            ui::section("Setting up Signal");
            ui::blank();
            println!("  Signal requires signal-cli (https://github.com/AsamK/signal-cli).");
            ui::blank();
            println!("  1. Install signal-cli:");
            println!("     - macOS: brew install signal-cli");
            println!("     - Linux: download from GitHub releases");
            println!("     - Or use the Docker image");
            println!("  2. Register or link a phone number:");
            println!("     signal-cli -u +1YOURPHONE register");
            println!("     signal-cli -u +1YOURPHONE verify CODE");
            println!("  3. Start signal-cli in JSON-RPC mode:");
            println!("     signal-cli -u +1YOURPHONE jsonRpc --socket /tmp/signal-cli.sock");
            ui::blank();

            let phone = prompt_input("  Your phone number (+1XXXX, or Enter to skip): ");

            let config_block = "\n[channels.signal]\nphone_env = \"SIGNAL_PHONE\"\nsocket_path = \"/tmp/signal-cli.sock\"\ndefault_agent = \"assistant\"\n";
            maybe_write_channel_config("signal", config_block);

            if !phone.is_empty() {
                match crate::dotenv::save_env_key("SIGNAL_PHONE", &phone) {
                    Ok(()) => ui::success("Phone saved to ~/.openfang/.env"),
                    Err(_) => println!("    export SIGNAL_PHONE={phone}"),
                }
            }

            ui::blank();
            ui::success("Signal configured");
            notify_daemon_restart();
        }
        "matrix" => {
            ui::section("Setting up Matrix");
            ui::blank();
            println!("  1. Create a bot account on your Matrix homeserver");
            println!("     (e.g., register @openfang-bot:matrix.org)");
            println!("  2. Obtain an access token:");
            println!("     curl -X POST https://matrix.org/_matrix/client/r0/login \\");
            println!("       -d '{{\"type\":\"m.login.password\",\"user\":\"openfang-bot\",\"password\":\"...\"}}'");
            println!("     Copy the access_token from the response.");
            println!("  3. Invite the bot to rooms you want it to monitor.");
            ui::blank();

            let homeserver = prompt_input("  Homeserver URL [https://matrix.org]: ");
            let homeserver = if homeserver.is_empty() {
                "https://matrix.org".to_string()
            } else {
                homeserver
            };
            let token = prompt_input("  Access token: ");

            let config_block = "\n[channels.matrix]\nhomeserver_env = \"MATRIX_HOMESERVER\"\naccess_token_env = \"MATRIX_ACCESS_TOKEN\"\ndefault_agent = \"assistant\"\n";
            maybe_write_channel_config("matrix", config_block);

            let _ = crate::dotenv::save_env_key("MATRIX_HOMESERVER", &homeserver);
            if !token.is_empty() {
                match crate::dotenv::save_env_key("MATRIX_ACCESS_TOKEN", &token) {
                    Ok(()) => ui::success("Token saved to ~/.openfang/.env"),
                    Err(_) => println!("    export MATRIX_ACCESS_TOKEN={token}"),
                }
            }

            ui::blank();
            ui::success("Matrix configured");
            notify_daemon_restart();
        }
        other => {
            ui::error_with_fix(
                &format!("Unknown channel: {other}"),
                "Available: telegram, discord, slack, whatsapp, email, signal, matrix",
            );
            std::process::exit(1);
        }
    }
}

pub fn cmd_channel_test(channel: &str) {
    if let Some(base) = find_daemon() {
        let client = daemon_client();
        let body = daemon_json(
            client
                .post(format!("{base}/api/channels/{channel}/test"))
                .send(),
        );
        if body.get("status").is_some() {
            println!("Test message sent to {channel}!");
        } else {
            eprintln!(
                "Failed: {}",
                body["error"].as_str().unwrap_or("Unknown error")
            );
        }
    } else {
        eprintln!("Channel test requires a running daemon. Start with: openfang start");
        std::process::exit(1);
    }
}

pub fn cmd_channel_toggle(channel: &str, enable: bool) {
    let action = if enable { "enabled" } else { "disabled" };
    if let Some(base) = find_daemon() {
        let client = daemon_client();
        let endpoint = if enable { "enable" } else { "disable" };
        let body = daemon_json(
            client
                .post(format!("{base}/api/channels/{channel}/{endpoint}"))
                .send(),
        );
        if body.get("status").is_some() {
            println!("Channel {channel} {action}.");
        } else {
            eprintln!(
                "Failed: {}",
                body["error"].as_str().unwrap_or("Unknown error")
            );
        }
    } else {
        println!("Note: Channel {channel} will be {action} when the daemon starts.");
        println!("Edit ~/.openfang/config.toml to persist this change.");
    }
}

// ---------------------------------------------------------------------------
// Integration commands (openfang add/remove/integrations)
// ---------------------------------------------------------------------------

pub fn cmd_integration_add(name: &str, key: Option<&str>) {
    let home = openfang_home();
    let mut registry = openfang_extensions::registry::IntegrationRegistry::new(&home);
    registry.load_bundled();
    let _ = registry.load_installed();

    let template = match registry.get_template(name) {
        Some(t) => t.clone(),
        None => {
            ui::error(&format!("Unknown integration: '{name}'"));
            println!("\nAvailable integrations:");
            for t in registry.list_templates() {
                println!("  {} {} — {}", t.icon, t.id, t.description);
            }
            std::process::exit(1);
        }
    };

    let dotenv_path = home.join(".env");
    let vault_path = home.join("vault.enc");
    let vault = if vault_path.exists() {
        let mut v = openfang_extensions::vault::CredentialVault::new(vault_path);
        if v.unlock().is_ok() {
            Some(v)
        } else {
            None
        }
    } else {
        None
    };
    let mut resolver =
        openfang_extensions::credentials::CredentialResolver::new(vault, Some(&dotenv_path))
            .with_interactive(true);

    let mut provided_keys = std::collections::HashMap::new();
    if let Some(key_value) = key {
        if let Some(env_var) = template.required_env.iter().find(|e| e.is_secret) {
            provided_keys.insert(env_var.name.clone(), key_value.to_string());
        }
    }

    match openfang_extensions::installer::install_integration(
        &mut registry,
        &mut resolver,
        name,
        &provided_keys,
    ) {
        Ok(result) => {
            match &result.status {
                openfang_extensions::IntegrationStatus::Ready => {
                    ui::success(&result.message);
                }
                openfang_extensions::IntegrationStatus::Setup => {
                    println!("{}", result.message.yellow());
                    println!("\nTo add credentials:");
                    for env in &template.required_env {
                        if env.is_secret {
                            println!("  openfang vault set {}  # {}", env.name, env.help);
                            if let Some(ref url) = env.get_url {
                                println!("  Get it here: {url}");
                            }
                        }
                    }
                }
                _ => println!("{}", result.message),
            }

            if let Some(base_url) = find_daemon() {
                let client = daemon_client();
                let _ = client
                    .post(format!("{base_url}/api/integrations/reload"))
                    .send();
            }
        }
        Err(e) => {
            ui::error(&e.to_string());
            std::process::exit(1);
        }
    }
}

pub fn cmd_integration_remove(name: &str) {
    let home = openfang_home();
    let mut registry = openfang_extensions::registry::IntegrationRegistry::new(&home);
    registry.load_bundled();
    let _ = registry.load_installed();

    match openfang_extensions::installer::remove_integration(&mut registry, name) {
        Ok(msg) => {
            ui::success(&msg);
            if let Some(base_url) = find_daemon() {
                let client = daemon_client();
                let _ = client
                    .post(format!("{base_url}/api/integrations/reload"))
                    .send();
            }
        }
        Err(e) => {
            ui::error(&e.to_string());
            std::process::exit(1);
        }
    }
}

pub fn cmd_integrations_list(query: Option<&str>) {
    let home = openfang_home();
    let mut registry = openfang_extensions::registry::IntegrationRegistry::new(&home);
    registry.load_bundled();
    let _ = registry.load_installed();

    let dotenv_path = home.join(".env");
    let resolver =
        openfang_extensions::credentials::CredentialResolver::new(None, Some(&dotenv_path));

    let entries = if let Some(q) = query {
        openfang_extensions::installer::search_integrations(&registry, q)
    } else {
        openfang_extensions::installer::list_integrations(&registry, &resolver)
    };

    if entries.is_empty() {
        if let Some(q) = query {
            println!("No integrations matching '{q}'.");
        } else {
            println!("No integrations available.");
        }
        return;
    }

    let mut by_category: std::collections::BTreeMap<
        String,
        Vec<&openfang_extensions::installer::IntegrationListEntry>,
    > = std::collections::BTreeMap::new();
    for entry in &entries {
        by_category
            .entry(entry.category.clone())
            .or_default()
            .push(entry);
    }

    for (category, items) in &by_category {
        println!("\n{}", format!("  {category}").bold());
        for item in items {
            let status_badge = match &item.status {
                openfang_extensions::IntegrationStatus::Ready => "[Ready]".green().to_string(),
                openfang_extensions::IntegrationStatus::Setup => "[Setup]".yellow().to_string(),
                openfang_extensions::IntegrationStatus::Available => {
                    "[Available]".dimmed().to_string()
                }
                openfang_extensions::IntegrationStatus::Error(msg) => {
                    format!("[Error: {msg}]").red().to_string()
                }
                openfang_extensions::IntegrationStatus::Disabled => {
                    "[Disabled]".dimmed().to_string()
                }
            };
            println!(
                "    {} {:<20} {:<12} {}",
                item.icon, item.id, status_badge, item.description
            );
        }
    }
    println!();
    println!(
        "  {} integrations ({} installed)",
        entries.len(),
        entries
            .iter()
            .filter(|e| matches!(
                e.status,
                openfang_extensions::IntegrationStatus::Ready
                    | openfang_extensions::IntegrationStatus::Setup
            ))
            .count()
    );
    println!("  Use `openfang add <name>` to install an integration.");
}

// ---------------------------------------------------------------------------
// Scaffold commands (openfang new skill/integration)
// ---------------------------------------------------------------------------

pub fn cmd_scaffold(kind: ScaffoldKind) {
    let cwd = std::env::current_dir().unwrap_or_default();
    let result = match kind {
        ScaffoldKind::Skill => {
            openfang_extensions::installer::scaffold_skill(&cwd.join("my-skill"))
        }
        ScaffoldKind::Integration => {
            openfang_extensions::installer::scaffold_integration(&cwd.join("my-integration"))
        }
    };
    match result {
        Ok(msg) => ui::success(&msg),
        Err(e) => {
            ui::error(&e.to_string());
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Approvals
// ---------------------------------------------------------------------------

pub fn cmd_approvals_list(json: bool) {
    let base = require_daemon("approvals list");
    let client = daemon_client();
    let body = daemon_json(client.get(format!("{base}/api/approvals")).send());
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
        return;
    }
    if let Some(arr) = body.as_array() {
        if arr.is_empty() {
            println!("No pending approvals.");
            return;
        }
        println!("{:<38} {:<16} {:<12} REQUEST", "ID", "AGENT", "TYPE");
        println!("{}", "-".repeat(80));
        for a in arr {
            println!(
                "{:<38} {:<16} {:<12} {}",
                a["id"].as_str().unwrap_or("?"),
                a["agent_name"].as_str().unwrap_or("?"),
                a["approval_type"].as_str().unwrap_or("?"),
                a["description"].as_str().unwrap_or(""),
            );
        }
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
    }
}

pub fn cmd_approvals_respond(id: &str, approve: bool) {
    let base = require_daemon("approvals");
    let client = daemon_client();
    let endpoint = if approve { "approve" } else { "reject" };
    let body = daemon_json(
        client
            .post(format!("{base}/api/approvals/{id}/{endpoint}"))
            .send(),
    );
    if body.get("error").is_some() {
        ui::error(&format!(
            "Failed: {}",
            body["error"].as_str().unwrap_or("?")
        ));
    } else {
        ui::success(&format!("Approval {id} {endpoint}d."));
    }
}

// ---------------------------------------------------------------------------
// Cron
// ---------------------------------------------------------------------------

pub fn cmd_cron_list(json: bool) {
    let base = require_daemon("cron list");
    let client = daemon_client();
    let body = daemon_json(client.get(format!("{base}/api/cron/jobs")).send());
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
        return;
    }
    if let Some(arr) = body.as_array() {
        if arr.is_empty() {
            println!("No scheduled jobs.");
            return;
        }
        println!(
            "{:<38} {:<16} {:<20} {:<8} PROMPT",
            "ID", "AGENT", "SCHEDULE", "ENABLED"
        );
        println!("{}", "-".repeat(100));
        for j in arr {
            println!(
                "{:<38} {:<16} {:<20} {:<8} {}",
                j["id"].as_str().unwrap_or("?"),
                j["agent_id"].as_str().unwrap_or("?"),
                j["cron_expr"].as_str().unwrap_or("?"),
                if j["enabled"].as_bool().unwrap_or(false) {
                    "yes"
                } else {
                    "no"
                },
                j["prompt"]
                    .as_str()
                    .unwrap_or("")
                    .chars()
                    .take(40)
                    .collect::<String>(),
            );
        }
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
    }
}

pub fn cmd_cron_create(agent: &str, spec: &str, prompt: &str) {
    let base = require_daemon("cron create");
    let client = daemon_client();
    let body = daemon_json(
        client
            .post(format!("{base}/api/cron/jobs"))
            .json(&serde_json::json!({
                "agent_id": agent,
                "cron_expr": spec,
                "prompt": prompt,
            }))
            .send(),
    );
    if let Some(id) = body["id"].as_str() {
        ui::success(&format!("Cron job created: {id}"));
    } else {
        ui::error(&format!(
            "Failed: {}",
            body["error"].as_str().unwrap_or("?")
        ));
    }
}

pub fn cmd_cron_delete(id: &str) {
    let base = require_daemon("cron delete");
    let client = daemon_client();
    let body = daemon_json(client.delete(format!("{base}/api/cron/jobs/{id}")).send());
    if body.get("error").is_some() {
        ui::error(&format!(
            "Failed: {}",
            body["error"].as_str().unwrap_or("?")
        ));
    } else {
        ui::success(&format!("Cron job {id} deleted."));
    }
}

pub fn cmd_cron_toggle(id: &str, enable: bool) {
    let base = require_daemon("cron");
    let client = daemon_client();
    let endpoint = if enable { "enable" } else { "disable" };
    let body = daemon_json(
        client
            .post(format!("{base}/api/cron/jobs/{id}/{endpoint}"))
            .send(),
    );
    if body.get("error").is_some() {
        ui::error(&format!(
            "Failed: {}",
            body["error"].as_str().unwrap_or("?")
        ));
    } else {
        ui::success(&format!("Cron job {id} {endpoint}d."));
    }
}

// ---------------------------------------------------------------------------
// Security
// ---------------------------------------------------------------------------

pub fn cmd_security_status(json: bool) {
    let base = require_daemon("security status");
    let client = daemon_client();
    let body = daemon_json(client.get(format!("{base}/api/health/detail")).send());
    if json {
        let data = serde_json::json!({
            "audit_trail": "merkle_hash_chain_sha256",
            "taint_tracking": "information_flow_labels",
            "wasm_sandbox": "dual_metering_fuel_epoch",
            "wire_protocol": "ofp_hmac_sha256_mutual_auth",
            "api_keys": "zeroizing_auto_wipe",
            "manifests": "ed25519_signed",
            "agent_count": body.get("agent_count").and_then(|v| v.as_u64()),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&data).unwrap_or_default()
        );
        return;
    }
    ui::section("Security Status");
    ui::blank();
    ui::kv("Audit trail", "Merkle hash chain (SHA-256)");
    ui::kv("Taint tracking", "Information flow labels");
    ui::kv("WASM sandbox", "Dual metering (fuel + epoch)");
    ui::kv("Wire protocol", "OFP HMAC-SHA256 mutual auth");
    ui::kv("API keys", "Zeroizing<String> (auto-wipe on drop)");
    ui::kv("Manifests", "Ed25519 signed");
    if let Some(agents) = body.get("agent_count").and_then(|v| v.as_u64()) {
        ui::kv("Active agents", &agents.to_string());
    }
}

pub fn cmd_security_audit(limit: usize, json: bool) {
    let base = require_daemon("security audit");
    let client = daemon_client();
    let body = daemon_json(
        client
            .get(format!("{base}/api/audit/recent?limit={limit}"))
            .send(),
    );
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
        return;
    }
    if let Some(arr) = body.as_array() {
        if arr.is_empty() {
            println!("No audit entries.");
            return;
        }
        println!("{:<24} {:<16} {:<12} EVENT", "TIMESTAMP", "AGENT", "TYPE");
        println!("{}", "-".repeat(80));
        for entry in arr {
            println!(
                "{:<24} {:<16} {:<12} {}",
                entry["timestamp"].as_str().unwrap_or("?"),
                entry["agent_name"].as_str().unwrap_or("?"),
                entry["event_type"].as_str().unwrap_or("?"),
                entry["description"].as_str().unwrap_or(""),
            );
        }
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
    }
}

pub fn cmd_security_verify() {
    let base = require_daemon("security verify");
    let client = daemon_client();
    let body = daemon_json(client.get(format!("{base}/api/audit/verify")).send());
    if body["valid"].as_bool().unwrap_or(false) {
        ui::success("Audit trail integrity verified (Merkle chain valid).");
    } else {
        ui::error("Audit trail integrity check FAILED.");
        if let Some(msg) = body["error"].as_str() {
            ui::hint(msg);
        }
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Memory
// ---------------------------------------------------------------------------

pub fn cmd_memory_list(agent: &str, json: bool) {
    let base = require_daemon("memory list");
    let client = daemon_client();
    let body = daemon_json(
        client
            .get(format!("{base}/api/memory/agents/{agent}/kv"))
            .send(),
    );
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
        return;
    }
    if let Some(arr) = body.as_array() {
        if arr.is_empty() {
            println!("No memory entries for agent '{agent}'.");
            return;
        }
        println!("{:<30} VALUE", "KEY");
        println!("{}", "-".repeat(60));
        for kv in arr {
            println!(
                "{:<30} {}",
                kv["key"].as_str().unwrap_or("?"),
                kv["value"]
                    .as_str()
                    .unwrap_or("")
                    .chars()
                    .take(50)
                    .collect::<String>(),
            );
        }
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
    }
}

pub fn cmd_memory_get(agent: &str, key: &str, json: bool) {
    let base = require_daemon("memory get");
    let client = daemon_client();
    let body = daemon_json(
        client
            .get(format!("{base}/api/memory/agents/{agent}/kv/{key}"))
            .send(),
    );
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
        return;
    }
    if let Some(val) = body["value"].as_str() {
        println!("{val}");
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
    }
}

pub fn cmd_memory_set(agent: &str, key: &str, value: &str) {
    let base = require_daemon("memory set");
    let client = daemon_client();
    let body = daemon_json(
        client
            .put(format!("{base}/api/memory/agents/{agent}/kv/{key}"))
            .json(&serde_json::json!({"value": value}))
            .send(),
    );
    if body.get("error").is_some() {
        ui::error(&format!(
            "Failed: {}",
            body["error"].as_str().unwrap_or("?")
        ));
    } else {
        ui::success(&format!("Set {key} for agent '{agent}'."));
    }
}

pub fn cmd_memory_delete(agent: &str, key: &str) {
    let base = require_daemon("memory delete");
    let client = daemon_client();
    let body = daemon_json(
        client
            .delete(format!("{base}/api/memory/agents/{agent}/kv/{key}"))
            .send(),
    );
    if body.get("error").is_some() {
        ui::error(&format!(
            "Failed: {}",
            body["error"].as_str().unwrap_or("?")
        ));
    } else {
        ui::success(&format!("Deleted key '{key}' for agent '{agent}'."));
    }
}

// ---------------------------------------------------------------------------
// Devices
// ---------------------------------------------------------------------------

pub fn cmd_devices_list(json: bool) {
    let base = require_daemon("devices list");
    let client = daemon_client();
    let body = daemon_json(client.get(format!("{base}/api/pairing/devices")).send());
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
        return;
    }
    if let Some(arr) = body.as_array() {
        if arr.is_empty() {
            println!("No paired devices.");
            return;
        }
        println!("{:<38} {:<20} LAST SEEN", "ID", "NAME");
        println!("{}", "-".repeat(70));
        for d in arr {
            println!(
                "{:<38} {:<20} {}",
                d["id"].as_str().unwrap_or("?"),
                d["name"].as_str().unwrap_or("?"),
                d["last_seen"].as_str().unwrap_or("?"),
            );
        }
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
    }
}

pub fn cmd_devices_pair() {
    let base = require_daemon("qr");
    let client = daemon_client();
    let body = daemon_json(client.post(format!("{base}/api/pairing/request")).send());
    if let Some(qr) = body["qr_data"].as_str() {
        ui::section("Device Pairing");
        ui::blank();
        println!("  Scan this QR code with the OpenFang mobile app:");
        ui::blank();
        println!("  {qr}");
        ui::blank();
        if let Some(code) = body["pairing_code"].as_str() {
            ui::kv("Pairing code", code);
        }
        if let Some(expires) = body["expires_at"].as_str() {
            ui::kv("Expires", expires);
        }
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
    }
}

pub fn cmd_devices_remove(id: &str) {
    let base = require_daemon("devices remove");
    let client = daemon_client();
    let body = daemon_json(
        client
            .delete(format!("{base}/api/pairing/devices/{id}"))
            .send(),
    );
    if body.get("error").is_some() {
        ui::error(&format!(
            "Failed: {}",
            body["error"].as_str().unwrap_or("?")
        ));
    } else {
        ui::success(&format!("Device {id} removed."));
    }
}

// ---------------------------------------------------------------------------
// Webhooks
// ---------------------------------------------------------------------------

pub fn cmd_webhooks_list(json: bool) {
    let base = require_daemon("webhooks list");
    let client = daemon_client();
    let body = daemon_json(client.get(format!("{base}/api/triggers")).send());
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
        return;
    }
    if let Some(arr) = body.as_array() {
        if arr.is_empty() {
            println!("No webhooks configured.");
            return;
        }
        println!("{:<38} {:<16} URL", "ID", "AGENT");
        println!("{}", "-".repeat(80));
        for w in arr {
            println!(
                "{:<38} {:<16} {}",
                w["id"].as_str().unwrap_or("?"),
                w["agent_id"].as_str().unwrap_or("?"),
                w["url"].as_str().unwrap_or(""),
            );
        }
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
    }
}

pub fn cmd_webhooks_create(agent: &str, url: &str) {
    let base = require_daemon("webhooks create");
    let client = daemon_client();
    let body = daemon_json(
        client
            .post(format!("{base}/api/triggers"))
            .json(&serde_json::json!({
                "agent_id": agent,
                "pattern": {"webhook": {"url": url}},
                "prompt_template": "Webhook event: {{event}}",
            }))
            .send(),
    );
    if let Some(id) = body["id"].as_str() {
        ui::success(&format!("Webhook created: {id}"));
    } else {
        ui::error(&format!(
            "Failed: {}",
            body["error"].as_str().unwrap_or("?")
        ));
    }
}

pub fn cmd_webhooks_delete(id: &str) {
    let base = require_daemon("webhooks delete");
    let client = daemon_client();
    let body = daemon_json(client.delete(format!("{base}/api/triggers/{id}")).send());
    if body.get("error").is_some() {
        ui::error(&format!(
            "Failed: {}",
            body["error"].as_str().unwrap_or("?")
        ));
    } else {
        ui::success(&format!("Webhook {id} deleted."));
    }
}

pub fn cmd_webhooks_test(id: &str) {
    let base = require_daemon("webhooks test");
    let client = daemon_client();
    let body = daemon_json(client.post(format!("{base}/api/triggers/{id}/test")).send());
    if body["success"].as_bool().unwrap_or(false) {
        ui::success(&format!("Webhook {id} test payload sent successfully."));
    } else {
        ui::error(&format!(
            "Webhook test failed: {}",
            body["error"].as_str().unwrap_or("?")
        ));
    }
}
