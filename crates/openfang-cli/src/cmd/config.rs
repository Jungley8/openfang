//! Config, vault, and reset commands.

use crate::daemon::restrict_file_permissions;
use crate::dotenv;
use crate::ui;
use chrono::Utc;
use colored::Colorize;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Rename the existing config.toml to a timestamped backup before writing a new one.
pub(crate) fn backup_existing_config(config_path: &Path) -> std::io::Result<Option<PathBuf>> {
    if !config_path.exists() {
        return Ok(None);
    }

    let file_name = config_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("config.toml");
    let timestamp = Utc::now().format("%Y%m%d%H%M%S");
    let backup_name = format!("{file_name}.backup-{timestamp}");
    let backup_path = config_path.with_file_name(&backup_name);

    std::fs::rename(config_path, &backup_path)?;
    Ok(Some(backup_path))
}

fn provider_to_env_var(provider: &str) -> String {
    match provider.to_lowercase().as_str() {
        "groq" => "GROQ_API_KEY".to_string(),
        "anthropic" => "ANTHROPIC_API_KEY".to_string(),
        "openai" => "OPENAI_API_KEY".to_string(),
        "gemini" => "GEMINI_API_KEY".to_string(),
        "google" => "GOOGLE_API_KEY".to_string(),
        "deepseek" => "DEEPSEEK_API_KEY".to_string(),
        "openrouter" => "OPENROUTER_API_KEY".to_string(),
        "together" => "TOGETHER_API_KEY".to_string(),
        "mistral" => "MISTRAL_API_KEY".to_string(),
        "fireworks" => "FIREWORKS_API_KEY".to_string(),
        "perplexity" => "PERPLEXITY_API_KEY".to_string(),
        "cohere" => "COHERE_API_KEY".to_string(),
        "xai" => "XAI_API_KEY".to_string(),
        "brave" => "BRAVE_API_KEY".to_string(),
        "tavily" => "TAVILY_API_KEY".to_string(),
        other => format!("{}_API_KEY", other.to_uppercase()),
    }
}

/// Test an API key by hitting the provider's models/health endpoint.
///
/// Returns true if the key is accepted (status != 401/403).
/// Returns true on timeout/network errors (best-effort — don't block setup).
pub(crate) fn test_api_key(provider: &str, env_var: &str) -> bool {
    let key = match std::env::var(env_var) {
        Ok(k) => k,
        Err(_) => return false,
    };

    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => return true, // can't build client — assume ok
    };

    let result = match provider.to_lowercase().as_str() {
        "groq" => client
            .get("https://api.groq.com/openai/v1/models")
            .bearer_auth(&key)
            .send(),
        "anthropic" => client
            .get("https://api.anthropic.com/v1/models")
            .header("x-api-key", &key)
            .header("anthropic-version", "2023-06-01")
            .send(),
        "openai" => client
            .get("https://api.openai.com/v1/models")
            .bearer_auth(&key)
            .send(),
        "gemini" | "google" => client
            .get(format!(
                "https://generativelanguage.googleapis.com/v1beta/models?key={key}"
            ))
            .send(),
        "deepseek" => client
            .get("https://api.deepseek.com/models")
            .bearer_auth(&key)
            .send(),
        "openrouter" => client
            .get("https://openrouter.ai/api/v1/models")
            .bearer_auth(&key)
            .send(),
        _ => return true, // unknown provider — skip test
    };

    match result {
        Ok(resp) => {
            let status = resp.status().as_u16();
            status != 401 && status != 403
        }
        Err(_) => true, // network error — don't block setup
    }
}

// ---------------------------------------------------------------------------
// Config commands
// ---------------------------------------------------------------------------

pub fn cmd_config_show() {
    let home = crate::openfang_home();
    let config_path = home.join("config.toml");

    if !config_path.exists() {
        println!("No configuration found at: {}", config_path.display());
        println!("Run `openfang init` to create one.");
        return;
    }

    let content = std::fs::read_to_string(&config_path).unwrap_or_else(|e| {
        eprintln!("Error reading config: {e}");
        std::process::exit(1);
    });

    println!("# {}\n", config_path.display());
    println!("{content}");
}

pub fn cmd_config_edit() {
    let home = crate::openfang_home();
    let config_path = home.join("config.toml");

    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| {
            if cfg!(windows) {
                "notepad".to_string()
            } else {
                "vi".to_string()
            }
        });

    let status = std::process::Command::new(&editor)
        .arg(&config_path)
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            eprintln!("Editor exited with: {s}");
        }
        Err(e) => {
            eprintln!("Failed to open editor '{editor}': {e}");
            eprintln!("Set $EDITOR to your preferred editor.");
        }
    }
}

pub fn cmd_config_get(key: &str) {
    let home = crate::openfang_home();
    let config_path = home.join("config.toml");

    if !config_path.exists() {
        ui::error_with_fix("No config file found", "Run `openfang init` first");
        std::process::exit(1);
    }

    let content = std::fs::read_to_string(&config_path).unwrap_or_else(|e| {
        ui::error(&format!("Failed to read config: {e}"));
        std::process::exit(1);
    });

    let table: toml::Value = toml::from_str(&content).unwrap_or_else(|e| {
        ui::error_with_fix(
            &format!("Config parse error: {e}"),
            "Fix your config.toml syntax, or run `openfang config edit`",
        );
        std::process::exit(1);
    });

    // Navigate dotted path
    let mut current = &table;
    for part in key.split('.') {
        match current.get(part) {
            Some(v) => current = v,
            None => {
                ui::error(&format!("Key not found: {key}"));
                std::process::exit(1);
            }
        }
    }

    // Print value
    match current {
        toml::Value::String(s) => println!("{s}"),
        toml::Value::Integer(i) => println!("{i}"),
        toml::Value::Float(f) => println!("{f}"),
        toml::Value::Boolean(b) => println!("{b}"),
        other => println!("{other}"),
    }
}

pub fn cmd_config_set(key: &str, value: &str) {
    let home = crate::openfang_home();
    let config_path = home.join("config.toml");

    if !config_path.exists() {
        ui::error_with_fix("No config file found", "Run `openfang init` first");
        std::process::exit(1);
    }

    let content = std::fs::read_to_string(&config_path).unwrap_or_else(|e| {
        ui::error(&format!("Failed to read config: {e}"));
        std::process::exit(1);
    });

    let mut table: toml::Value = toml::from_str(&content).unwrap_or_else(|e| {
        ui::error_with_fix(
            &format!("Config parse error: {e}"),
            "Fix your config.toml syntax first",
        );
        std::process::exit(1);
    });

    // Navigate to parent and set key
    let parts: Vec<&str> = key.split('.').collect();
    if parts.is_empty() {
        ui::error("Empty key");
        std::process::exit(1);
    }

    let mut current = &mut table;
    for part in &parts[..parts.len() - 1] {
        current = current
            .as_table_mut()
            .and_then(|t| t.get_mut(*part))
            .unwrap_or_else(|| {
                ui::error(&format!("Key path not found: {key}"));
                std::process::exit(1);
            });
    }

    let last_key = parts[parts.len() - 1];
    let tbl = current.as_table_mut().unwrap_or_else(|| {
        ui::error(&format!("Parent of '{key}' is not a table"));
        std::process::exit(1);
    });

    // Try to preserve type: if the existing value is an integer, parse as int, etc.
    let new_value = if let Some(existing) = tbl.get(last_key) {
        match existing {
            toml::Value::Integer(_) => value
                .parse::<i64>()
                .map(toml::Value::Integer)
                .unwrap_or_else(|_| toml::Value::String(value.to_string())),
            toml::Value::Float(_) => value
                .parse::<f64>()
                .map(toml::Value::Float)
                .unwrap_or_else(|_| toml::Value::String(value.to_string())),
            toml::Value::Boolean(_) => value
                .parse::<bool>()
                .map(toml::Value::Boolean)
                .unwrap_or_else(|_| toml::Value::String(value.to_string())),
            _ => toml::Value::String(value.to_string()),
        }
    } else {
        toml::Value::String(value.to_string())
    };

    tbl.insert(last_key.to_string(), new_value);

    // Write back (note: this strips comments — warned in help text)
    let serialized = toml::to_string_pretty(&table).unwrap_or_else(|e| {
        ui::error(&format!("Failed to serialize config: {e}"));
        std::process::exit(1);
    });

    std::fs::write(&config_path, &serialized).unwrap_or_else(|e| {
        ui::error(&format!("Failed to write config: {e}"));
        std::process::exit(1);
    });
    restrict_file_permissions(&config_path);

    ui::success(&format!("Set {key} = {value}"));
}

pub fn cmd_config_unset(key: &str) {
    let home = crate::openfang_home();
    let config_path = home.join("config.toml");

    if !config_path.exists() {
        ui::error_with_fix("No config file found", "Run `openfang init` first");
        std::process::exit(1);
    }

    let content = std::fs::read_to_string(&config_path).unwrap_or_else(|e| {
        ui::error(&format!("Failed to read config: {e}"));
        std::process::exit(1);
    });

    let mut table: toml::Value = toml::from_str(&content).unwrap_or_else(|e| {
        ui::error_with_fix(
            &format!("Config parse error: {e}"),
            "Fix your config.toml syntax first",
        );
        std::process::exit(1);
    });

    // Navigate to parent table and remove the final key
    let parts: Vec<&str> = key.split('.').collect();
    if parts.is_empty() {
        ui::error("Empty key");
        std::process::exit(1);
    }

    let mut current = &mut table;
    for part in &parts[..parts.len() - 1] {
        current = current
            .as_table_mut()
            .and_then(|t| t.get_mut(*part))
            .unwrap_or_else(|| {
                ui::error(&format!("Key path not found: {key}"));
                std::process::exit(1);
            });
    }

    let last_key = parts[parts.len() - 1];
    let tbl = current.as_table_mut().unwrap_or_else(|| {
        ui::error(&format!("Parent of '{key}' is not a table"));
        std::process::exit(1);
    });

    if tbl.remove(last_key).is_none() {
        ui::error(&format!("Key not found: {key}"));
        std::process::exit(1);
    }

    // Write back (note: this strips comments — warned in help text)
    let serialized = toml::to_string_pretty(&table).unwrap_or_else(|e| {
        ui::error(&format!("Failed to serialize config: {e}"));
        std::process::exit(1);
    });

    std::fs::write(&config_path, &serialized).unwrap_or_else(|e| {
        ui::error(&format!("Failed to write config: {e}"));
        std::process::exit(1);
    });
    restrict_file_permissions(&config_path);

    ui::success(&format!("Removed key: {key}"));
}

pub fn cmd_config_set_key(provider: &str) {
    let env_var = provider_to_env_var(provider);

    let key = crate::prompt_input(&format!("  Paste your {provider} API key: "));
    if key.is_empty() {
        ui::error("No key provided. Cancelled.");
        return;
    }

    match dotenv::save_env_key(&env_var, &key) {
        Ok(()) => {
            ui::success(&format!("Saved {env_var} to ~/.openfang/.env"));
            // Test the key
            print!("  Testing key... ");
            io::stdout().flush().unwrap();
            if test_api_key(provider, &env_var) {
                println!("{}", "OK".bright_green());
            } else {
                println!("{}", "could not verify (may still work)".bright_yellow());
            }
        }
        Err(e) => {
            ui::error(&format!("Failed to save key: {e}"));
            std::process::exit(1);
        }
    }
}

pub fn cmd_config_delete_key(provider: &str) {
    let env_var = provider_to_env_var(provider);

    match dotenv::remove_env_key(&env_var) {
        Ok(()) => ui::success(&format!("Removed {env_var} from ~/.openfang/.env")),
        Err(e) => {
            ui::error(&format!("Failed to remove key: {e}"));
            std::process::exit(1);
        }
    }
}

pub fn cmd_config_test_key(provider: &str) {
    let env_var = provider_to_env_var(provider);

    if std::env::var(&env_var).is_err() {
        ui::error(&format!("{env_var} not set"));
        ui::hint(&format!("Set it: openfang config set-key {provider}"));
        std::process::exit(1);
    }

    print!("  Testing {provider} ({env_var})... ");
    io::stdout().flush().unwrap();
    if test_api_key(provider, &env_var) {
        println!("{}", "OK".bright_green());
    } else {
        println!("{}", "FAILED (401/403)".bright_red());
        ui::hint(&format!("Update key: openfang config set-key {provider}"));
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Vault commands
// ---------------------------------------------------------------------------

pub fn cmd_vault_init() {
    let home = crate::openfang_home();
    let vault_path = home.join("vault.enc");
    let mut vault = openfang_extensions::vault::CredentialVault::new(vault_path);

    match vault.init() {
        Ok(()) => ui::success("Credential vault initialized."),
        Err(e) => {
            ui::error(&e.to_string());
            std::process::exit(1);
        }
    }
}

pub fn cmd_vault_set(key: &str) {
    use zeroize::Zeroizing;

    let home = crate::openfang_home();
    let vault_path = home.join("vault.enc");
    let mut vault = openfang_extensions::vault::CredentialVault::new(vault_path);

    if !vault.exists() {
        ui::error("Vault not initialized. Run: openfang vault init");
        std::process::exit(1);
    }

    if let Err(e) = vault.unlock() {
        ui::error(&format!("Could not unlock vault: {e}"));
        std::process::exit(1);
    }

    let value = crate::prompt_input(&format!("Enter value for {key}: "));
    if value.is_empty() {
        ui::error("Empty value — not stored.");
        std::process::exit(1);
    }

    match vault.set(key.to_string(), Zeroizing::new(value)) {
        Ok(()) => ui::success(&format!("Stored '{key}' in vault.")),
        Err(e) => {
            ui::error(&format!("Failed to store: {e}"));
            std::process::exit(1);
        }
    }
}

pub fn cmd_vault_list() {
    let home = crate::openfang_home();
    let vault_path = home.join("vault.enc");
    let mut vault = openfang_extensions::vault::CredentialVault::new(vault_path);

    if !vault.exists() {
        println!("Vault not initialized. Run: openfang vault init");
        return;
    }

    if let Err(e) = vault.unlock() {
        ui::error(&format!("Could not unlock vault: {e}"));
        std::process::exit(1);
    }

    let keys = vault.list_keys();
    if keys.is_empty() {
        println!("Vault is empty.");
    } else {
        println!("Stored credentials ({}):", keys.len());
        for key in keys {
            println!("  {key}");
        }
    }
}

pub fn cmd_vault_remove(key: &str) {
    let home = crate::openfang_home();
    let vault_path = home.join("vault.enc");
    let mut vault = openfang_extensions::vault::CredentialVault::new(vault_path);

    if !vault.exists() {
        ui::error("Vault not initialized.");
        std::process::exit(1);
    }
    if let Err(e) = vault.unlock() {
        ui::error(&format!("Could not unlock vault: {e}"));
        std::process::exit(1);
    }

    match vault.remove(key) {
        Ok(true) => ui::success(&format!("Removed '{key}' from vault.")),
        Ok(false) => println!("Key '{key}' not found in vault."),
        Err(e) => {
            ui::error(&format!("Failed to remove: {e}"));
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Reset
// ---------------------------------------------------------------------------

pub fn cmd_reset(confirm: bool) {
    let openfang_dir = match dirs::home_dir() {
        Some(h) => h.join(".openfang"),
        None => {
            ui::error("Could not determine home directory");
            std::process::exit(1);
        }
    };

    if !openfang_dir.exists() {
        println!(
            "Nothing to reset — {} does not exist.",
            openfang_dir.display()
        );
        return;
    }

    if !confirm {
        println!("  This will delete all data in {}", openfang_dir.display());
        println!("  Including: config, database, agent manifests, credentials.");
        println!();
        let answer = crate::prompt_input("  Are you sure? Type 'yes' to confirm: ");
        if answer.trim() != "yes" {
            println!("  Cancelled.");
            return;
        }
    }

    match std::fs::remove_dir_all(&openfang_dir) {
        Ok(()) => ui::success(&format!("Removed {}", openfang_dir.display())),
        Err(e) => {
            ui::error(&format!("Failed to remove {}: {e}", openfang_dir.display()));
            std::process::exit(1);
        }
    }
}
