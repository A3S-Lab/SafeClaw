//! SafeClaw - Secure Personal AI Assistant with TEE Support
//!
//! A privacy-focused personal AI assistant that combines multi-channel
//! messaging capabilities with hardware-isolated execution for sensitive data.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use safeclaw::{
    config::{
        ChannelsConfig, DingTalkConfig, DiscordConfig, FeishuConfig, GatewayConfig,
        ModelProviderConfig, ModelsConfig, SafeClawConfig, SlackConfig, TeeBackend, TeeConfig,
        TelegramConfig, WeComConfig, WebChatConfig,
    },
    gateway::GatewayBuilder,
};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "safeclaw")]
#[command(author = "A3S Lab Team")]
#[command(version)]
#[command(about = "Secure Personal AI Assistant with TEE Support")]
struct Cli {
    /// Configuration file path
    #[arg(short, long, env = "SAFECLAW_CONFIG")]
    config: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the SafeClaw gateway
    Gateway {
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Port to listen on
        #[arg(long, default_value = "18790")]
        port: u16,

        /// Disable TEE mode
        #[arg(long)]
        no_tee: bool,
    },

    /// Run the onboarding wizard
    Onboard {
        /// Install as system daemon
        #[arg(long)]
        install_daemon: bool,
    },

    /// Send a message
    Message {
        /// Target channel
        #[arg(short, long)]
        channel: String,

        /// Target chat ID
        #[arg(short = 't', long)]
        to: String,

        /// Message content
        #[arg(short, long)]
        message: String,

        /// Gateway URL to connect to
        #[arg(long, default_value = "http://127.0.0.1:18790")]
        gateway_url: String,
    },

    /// Run diagnostics
    Doctor,

    /// Show configuration
    Config {
        /// Show default configuration
        #[arg(long)]
        default: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("safeclaw={},tower_http=debug", log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = if let Some(config_path) = cli.config {
        let content = std::fs::read_to_string(&config_path)?;
        toml::from_str(&content)?
    } else {
        SafeClawConfig::default()
    };

    match cli.command {
        Commands::Gateway { host, port, no_tee } => {
            run_gateway(config, host, port, !no_tee).await?;
        }
        Commands::Onboard { install_daemon } => {
            run_onboard(install_daemon).await?;
        }
        Commands::Message {
            channel,
            to,
            message,
            gateway_url,
        } => {
            send_message(&channel, &to, &message, &gateway_url).await?;
        }
        Commands::Doctor => {
            run_doctor().await?;
        }
        Commands::Config { default } => {
            show_config(if default { None } else { Some(&config) })?;
        }
    }

    Ok(())
}

async fn run_gateway(
    config: SafeClawConfig,
    host: String,
    port: u16,
    tee_enabled: bool,
) -> Result<()> {
    tracing::info!("Starting SafeClaw Gateway");

    let gateway = GatewayBuilder::new()
        .config(config)
        .host(host)
        .port(port)
        .tee_enabled(tee_enabled)
        .build();

    gateway.start().await?;

    tracing::info!("SafeClaw Gateway is running. Press Ctrl+C to stop.");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;

    tracing::info!("Shutting down...");
    gateway.stop().await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// send_message — POST to running gateway
// ---------------------------------------------------------------------------

async fn send_message(channel: &str, to: &str, message: &str, gateway_url: &str) -> Result<()> {
    let client = reqwest::Client::new();

    // Health check
    let health_url = format!("{}/health", gateway_url);
    let health_resp = client
        .get(&health_url)
        .send()
        .await
        .context("Failed to connect to gateway. Is it running?")?;

    if !health_resp.status().is_success() {
        anyhow::bail!(
            "Gateway health check failed with status {}",
            health_resp.status()
        );
    }

    let health: serde_json::Value = health_resp
        .json()
        .await
        .context("Failed to parse health response")?;
    println!(
        "Gateway is healthy (version: {})",
        health["version"].as_str().unwrap_or("unknown")
    );

    // Send message
    let msg_url = format!("{}/message", gateway_url);
    let body = serde_json::json!({
        "channel": channel,
        "chat_id": to,
        "content": message,
    });

    let resp = client
        .post(&msg_url)
        .json(&body)
        .send()
        .await
        .context("Failed to send message to gateway")?;

    let status = resp.status();
    let resp_body: serde_json::Value = resp
        .json()
        .await
        .context("Failed to parse message response")?;

    if status.is_success() {
        println!(
            "Message sent! ID: {}, Status: {}",
            resp_body["message_id"].as_str().unwrap_or("unknown"),
            resp_body["status"].as_str().unwrap_or("unknown"),
        );
    } else {
        anyhow::bail!(
            "Failed to send message (HTTP {}): {}",
            status,
            resp_body["status"].as_str().unwrap_or("unknown error"),
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// prompt_input — interactive stdin helper
// ---------------------------------------------------------------------------

fn prompt_input(question: &str, default: &str) -> String {
    prompt_input_from(
        question,
        default,
        &mut io::stdin().lock(),
        &mut io::stdout(),
    )
}

fn prompt_input_from(
    question: &str,
    default: &str,
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
) -> String {
    if default.is_empty() {
        let _ = write!(writer, "{}: ", question);
    } else {
        let _ = write!(writer, "{} [{}]: ", question, default);
    }
    let _ = writer.flush();

    let mut input = String::new();
    let _ = reader.read_line(&mut input);
    let trimmed = input.trim();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed.to_string()
    }
}

fn prompt_yes_no(question: &str, default_yes: bool) -> bool {
    let default_str = if default_yes { "Y/n" } else { "y/N" };
    let answer = prompt_input(&format!("{} [{}]", question, default_str), "");
    if answer.is_empty() {
        return default_yes;
    }
    matches!(answer.to_lowercase().as_str(), "y" | "yes")
}

// ---------------------------------------------------------------------------
// run_onboard — interactive setup wizard
// ---------------------------------------------------------------------------

async fn run_onboard(install_daemon: bool) -> Result<()> {
    println!("SafeClaw Onboarding Wizard");
    println!("=========================");
    println!();
    println!("SafeClaw is a secure personal AI assistant that protects your privacy");
    println!("by running sensitive computations in a Trusted Execution Environment (TEE).");
    println!();

    // Step 1: Gateway Config
    println!("--- Step 1: Gateway Configuration ---");
    let host = prompt_input("Gateway host", "127.0.0.1");
    let port: u16 = prompt_input("Gateway port", "18790")
        .parse()
        .unwrap_or(18790);
    println!();

    // Step 2: TEE Configuration
    println!("--- Step 2: TEE Configuration ---");
    let tee_enabled = prompt_yes_no("Enable TEE?", true);
    let mut tee_backend = TeeBackend::A3sBox;
    let mut memory_mb: u32 = 2048;
    let mut cpu_cores: u32 = 2;

    if tee_enabled {
        let backend_str = prompt_input(
            "TEE backend (a3s_box / intel_sgx / amd_sev / arm_trustzone)",
            "a3s_box",
        );
        tee_backend = match backend_str.as_str() {
            "intel_sgx" => TeeBackend::IntelSgx,
            "amd_sev" => TeeBackend::AmdSev,
            "arm_trustzone" => TeeBackend::ArmTrustzone,
            _ => TeeBackend::A3sBox,
        };
        memory_mb = prompt_input("Memory (MB)", "2048").parse().unwrap_or(2048);
        cpu_cores = prompt_input("CPU cores", "2").parse().unwrap_or(2);
    }
    println!();

    // Step 3: Channel Selection
    println!("--- Step 3: Channel Selection ---");
    let mut channels = ChannelsConfig::default();

    // Telegram
    if prompt_yes_no("Enable Telegram?", false) {
        let bot_token_ref = prompt_input("  Telegram bot_token_ref", "telegram_bot_token");
        let users_str = prompt_input(
            "  Allowed user IDs (comma-separated, or empty for none)",
            "",
        );
        let allowed_users: Vec<i64> = users_str
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        channels.telegram = Some(TelegramConfig {
            bot_token_ref,
            allowed_users,
            dm_policy: "pairing".to_string(),
        });
    }

    // Slack
    if prompt_yes_no("Enable Slack?", false) {
        let bot_token_ref = prompt_input("  Slack bot_token_ref", "slack_bot_token");
        let app_token_ref = prompt_input("  Slack app_token_ref", "slack_app_token");
        channels.slack = Some(SlackConfig {
            bot_token_ref,
            app_token_ref,
            allowed_workspaces: vec![],
            dm_policy: "pairing".to_string(),
        });
    }

    // Discord
    if prompt_yes_no("Enable Discord?", false) {
        let bot_token_ref = prompt_input("  Discord bot_token_ref", "discord_bot_token");
        channels.discord = Some(DiscordConfig {
            bot_token_ref,
            allowed_guilds: vec![],
            dm_policy: "pairing".to_string(),
        });
    }

    // WebChat
    if prompt_yes_no("Enable WebChat?", true) {
        channels.webchat = Some(WebChatConfig::default());
    }

    // Feishu
    if prompt_yes_no("Enable Feishu?", false) {
        let app_id = prompt_input("  Feishu app_id", "");
        let app_secret_ref = prompt_input("  Feishu app_secret_ref", "feishu_app_secret");
        channels.feishu = Some(FeishuConfig {
            app_id,
            app_secret_ref,
            encrypt_key_ref: String::new(),
            verification_token_ref: String::new(),
            allowed_users: vec![],
            dm_policy: "pairing".to_string(),
        });
    }

    // DingTalk
    if prompt_yes_no("Enable DingTalk?", false) {
        let app_key_ref = prompt_input("  DingTalk app_key_ref", "dingtalk_app_key");
        let app_secret_ref = prompt_input("  DingTalk app_secret_ref", "dingtalk_app_secret");
        let robot_code = prompt_input("  DingTalk robot_code", "");
        channels.dingtalk = Some(DingTalkConfig {
            app_key_ref,
            app_secret_ref,
            robot_code,
            allowed_users: vec![],
            dm_policy: "pairing".to_string(),
        });
    }

    // WeCom
    if prompt_yes_no("Enable WeCom?", false) {
        let corp_id = prompt_input("  WeCom corp_id", "");
        let agent_id: u32 = prompt_input("  WeCom agent_id", "1000001")
            .parse()
            .unwrap_or(1000001);
        let secret_ref = prompt_input("  WeCom secret_ref", "wecom_secret");
        channels.wecom = Some(WeComConfig {
            corp_id,
            agent_id,
            secret_ref,
            encoding_aes_key_ref: String::new(),
            token_ref: String::new(),
            allowed_users: vec![],
            dm_policy: "pairing".to_string(),
        });
    }
    println!();

    // Step 4: AI Model Provider
    println!("--- Step 4: AI Model Provider ---");
    let provider = prompt_input("Default provider (anthropic / openai)", "anthropic");
    let api_key_ref = prompt_input("API key reference name", &format!("{}_api_key", provider));

    let mut providers = HashMap::new();
    providers.insert(
        provider.clone(),
        ModelProviderConfig {
            api_key_ref,
            base_url: None,
            default_model: match provider.as_str() {
                "openai" => "gpt-4o".to_string(),
                _ => "claude-sonnet-4-20250514".to_string(),
            },
            models: match provider.as_str() {
                "openai" => vec![
                    "gpt-4o".to_string(),
                    "gpt-4o-mini".to_string(),
                    "o1".to_string(),
                ],
                _ => vec![
                    "claude-opus-4-20250514".to_string(),
                    "claude-sonnet-4-20250514".to_string(),
                    "claude-haiku-3-5-20241022".to_string(),
                ],
            },
        },
    );
    println!();

    // Step 5: Build and write config
    println!("--- Step 5: Writing Configuration ---");
    let config = SafeClawConfig {
        gateway: GatewayConfig {
            host,
            port,
            ..GatewayConfig::default()
        },
        tee: TeeConfig {
            enabled: tee_enabled,
            backend: tee_backend,
            memory_mb,
            cpu_cores,
            ..TeeConfig::default()
        },
        channels,
        models: ModelsConfig {
            default_provider: provider,
            providers,
        },
        ..Default::default()
    };

    let toml_str =
        toml::to_string_pretty(&config).context("Failed to serialize configuration to TOML")?;

    let config_dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("safeclaw");
    std::fs::create_dir_all(&config_dir).with_context(|| {
        format!(
            "Failed to create config directory: {}",
            config_dir.display()
        )
    })?;

    let config_path = config_dir.join("config.toml");
    std::fs::write(&config_path, &toml_str)
        .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

    println!("Configuration written to: {}", config_path.display());

    // Step 6: Daemon installation
    if install_daemon {
        println!();
        println!("--- Step 6: Daemon Installation ---");
        install_system_daemon(&config_path)?;
    }

    println!();
    println!("Onboarding complete! Start the gateway with:");
    println!("  safeclaw gateway");

    Ok(())
}

// ---------------------------------------------------------------------------
// Daemon installation helpers
// ---------------------------------------------------------------------------

fn install_system_daemon(config_path: &std::path::Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        install_launchd_daemon(config_path)?;
    }
    #[cfg(target_os = "linux")]
    {
        install_systemd_daemon(config_path)?;
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = config_path;
        println!("Daemon installation is not supported on this platform.");
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn install_launchd_daemon(config_path: &std::path::Path) -> Result<()> {
    let plist_content = generate_launchd_plist(config_path)?;
    let home = std::env::var("HOME").context("HOME not set")?;
    let agents_dir = PathBuf::from(&home).join("Library/LaunchAgents");
    std::fs::create_dir_all(&agents_dir).with_context(|| {
        format!(
            "Failed to create LaunchAgents directory: {}",
            agents_dir.display()
        )
    })?;

    let plist_path = agents_dir.join("com.a3s.safeclaw.plist");
    std::fs::write(&plist_path, &plist_content)
        .with_context(|| format!("Failed to write plist: {}", plist_path.display()))?;

    println!("LaunchAgent installed: {}", plist_path.display());
    println!("Loading daemon...");

    let status = std::process::Command::new("launchctl")
        .args(["load", "-w"])
        .arg(&plist_path)
        .status()
        .context("Failed to run launchctl")?;

    if status.success() {
        println!("Daemon loaded successfully.");
    } else {
        println!("Warning: launchctl load returned non-zero exit code.");
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn install_systemd_daemon(config_path: &std::path::Path) -> Result<()> {
    let unit_content = generate_systemd_unit(config_path)?;
    let home = std::env::var("HOME").context("HOME not set")?;
    let units_dir = PathBuf::from(&home).join(".config/systemd/user");
    std::fs::create_dir_all(&units_dir).with_context(|| {
        format!(
            "Failed to create systemd user directory: {}",
            units_dir.display()
        )
    })?;

    let unit_path = units_dir.join("safeclaw.service");
    std::fs::write(&unit_path, &unit_content)
        .with_context(|| format!("Failed to write unit file: {}", unit_path.display()))?;

    println!("Systemd user unit installed: {}", unit_path.display());
    println!("Enabling and starting daemon...");

    let _ = std::process::Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();
    let status = std::process::Command::new("systemctl")
        .args(["--user", "enable", "--now", "safeclaw.service"])
        .status()
        .context("Failed to run systemctl")?;

    if status.success() {
        println!("Daemon enabled and started.");
    } else {
        println!("Warning: systemctl enable returned non-zero exit code.");
    }

    Ok(())
}

#[allow(dead_code)]
fn generate_launchd_plist(config_path: &std::path::Path) -> Result<String> {
    let safeclaw_bin = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("safeclaw"));
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.a3s.safeclaw</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>--config</string>
        <string>{}</string>
        <string>gateway</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/safeclaw.stdout.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/safeclaw.stderr.log</string>
</dict>
</plist>"#,
        safeclaw_bin.display(),
        config_path.display(),
    ))
}

#[allow(dead_code)]
fn generate_systemd_unit(config_path: &std::path::Path) -> Result<String> {
    let safeclaw_bin = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("safeclaw"));
    Ok(format!(
        r#"[Unit]
Description=SafeClaw - Secure Personal AI Assistant
After=network.target

[Service]
Type=simple
ExecStart={} --config {} gateway
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target"#,
        safeclaw_bin.display(),
        config_path.display(),
    ))
}

// ---------------------------------------------------------------------------
// Doctor and Config commands (unchanged)
// ---------------------------------------------------------------------------

async fn run_doctor() -> Result<()> {
    println!("SafeClaw Doctor");
    println!();

    // Check TEE availability
    println!("Checking TEE availability...");
    #[cfg(target_os = "macos")]
    {
        println!("  macOS detected - Apple Hypervisor Framework available");
    }
    #[cfg(target_os = "linux")]
    {
        println!("  Checking KVM...");
        if std::path::Path::new("/dev/kvm").exists() {
            println!("  KVM available");
        } else {
            println!("  KVM not available");
        }
    }

    // Check configuration
    println!();
    println!("Checking configuration...");
    let config_path = dirs::config_dir().map(|p| p.join("safeclaw").join("config.toml"));
    if let Some(path) = config_path {
        if path.exists() {
            println!("  Configuration file found: {}", path.display());
        } else {
            println!("  No configuration file found (using defaults)");
        }
    }

    println!();
    println!("Doctor check complete!");

    Ok(())
}

fn show_config(config: Option<&SafeClawConfig>) -> Result<()> {
    let config = config.cloned().unwrap_or_default();
    let toml = toml::to_string_pretty(&config)?;
    println!("{}", toml);
    Ok(())
}

mod dirs {
    use std::path::PathBuf;

    pub fn config_dir() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join("Library/Application Support"))
        }
        #[cfg(target_os = "linux")]
        {
            std::env::var("XDG_CONFIG_HOME")
                .ok()
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var("HOME")
                        .ok()
                        .map(|h| PathBuf::from(h).join(".config"))
                })
        }
        #[cfg(target_os = "windows")]
        {
            std::env::var("APPDATA").ok().map(PathBuf::from)
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_prompt_input_with_default() {
        // Empty input returns default
        let mut reader = Cursor::new(b"\n");
        let mut writer = Vec::new();
        let result = prompt_input_from("Host", "127.0.0.1", &mut reader, &mut writer);
        assert_eq!(result, "127.0.0.1");

        // Non-empty input overrides default
        let mut reader = Cursor::new(b"0.0.0.0\n");
        let mut writer = Vec::new();
        let result = prompt_input_from("Host", "127.0.0.1", &mut reader, &mut writer);
        assert_eq!(result, "0.0.0.0");

        // Whitespace-only input returns default
        let mut reader = Cursor::new(b"   \n");
        let mut writer = Vec::new();
        let result = prompt_input_from("Host", "127.0.0.1", &mut reader, &mut writer);
        assert_eq!(result, "127.0.0.1");

        // Verify prompt format includes default
        let mut reader = Cursor::new(b"\n");
        let mut writer = Vec::new();
        prompt_input_from("Port", "18790", &mut reader, &mut writer);
        let output = String::from_utf8(writer).unwrap();
        assert!(output.contains("[18790]"));

        // Verify prompt format without default
        let mut reader = Cursor::new(b"value\n");
        let mut writer = Vec::new();
        prompt_input_from("Token", "", &mut reader, &mut writer);
        let output = String::from_utf8(writer).unwrap();
        assert!(output.contains("Token:"));
        assert!(!output.contains("[]"));
    }

    #[test]
    fn test_build_config_from_wizard() {
        // Simulate building config from wizard values
        let mut providers = HashMap::new();
        providers.insert(
            "anthropic".to_string(),
            ModelProviderConfig {
                api_key_ref: "my_anthropic_key".to_string(),
                base_url: None,
                default_model: "claude-sonnet-4-20250514".to_string(),
                models: vec!["claude-sonnet-4-20250514".to_string()],
            },
        );

        let config = SafeClawConfig {
            gateway: GatewayConfig {
                host: "0.0.0.0".to_string(),
                port: 9090,
                ..GatewayConfig::default()
            },
            tee: TeeConfig {
                enabled: true,
                backend: TeeBackend::IntelSgx,
                memory_mb: 4096,
                cpu_cores: 4,
                ..TeeConfig::default()
            },
            channels: ChannelsConfig {
                telegram: Some(TelegramConfig {
                    bot_token_ref: "my_tg_token".to_string(),
                    allowed_users: vec![12345],
                    dm_policy: "pairing".to_string(),
                }),
                webchat: Some(WebChatConfig::default()),
                ..Default::default()
            },
            models: ModelsConfig {
                default_provider: "anthropic".to_string(),
                providers,
            },
            ..Default::default()
        };

        // Verify serialization round-trip
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: SafeClawConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.gateway.host, "0.0.0.0");
        assert_eq!(parsed.gateway.port, 9090);
        assert!(parsed.tee.enabled);
        assert_eq!(parsed.tee.memory_mb, 4096);
        assert_eq!(parsed.tee.cpu_cores, 4);
        assert!(parsed.channels.telegram.is_some());
        assert!(parsed.channels.webchat.is_some());
        assert!(parsed.channels.slack.is_none());
        assert_eq!(parsed.models.default_provider, "anthropic");

        let tg = parsed.channels.telegram.unwrap();
        assert_eq!(tg.bot_token_ref, "my_tg_token");
        assert_eq!(tg.allowed_users, vec![12345]);
    }

    #[test]
    fn test_daemon_plist_generation() {
        let config_path =
            PathBuf::from("/Users/test/Library/Application Support/safeclaw/config.toml");
        let plist = generate_launchd_plist(&config_path).unwrap();

        assert!(plist.contains("<key>Label</key>"));
        assert!(plist.contains("com.a3s.safeclaw"));
        assert!(plist.contains("<key>RunAtLoad</key>"));
        assert!(plist.contains("<true/>"));
        assert!(plist.contains("<key>KeepAlive</key>"));
        assert!(plist.contains("--config"));
        assert!(plist.contains("gateway"));
        assert!(plist.contains("/Users/test/Library/Application Support/safeclaw/config.toml"));
        assert!(plist.contains("safeclaw.stdout.log"));
        assert!(plist.contains("safeclaw.stderr.log"));
    }

    #[test]
    fn test_daemon_systemd_generation() {
        let config_path = PathBuf::from("/home/test/.config/safeclaw/config.toml");
        let unit = generate_systemd_unit(&config_path).unwrap();

        assert!(unit.contains("[Unit]"));
        assert!(unit.contains("[Service]"));
        assert!(unit.contains("[Install]"));
        assert!(unit.contains("Description=SafeClaw"));
        assert!(unit.contains("Type=simple"));
        assert!(unit.contains("Restart=on-failure"));
        assert!(unit.contains("--config"));
        assert!(unit.contains("/home/test/.config/safeclaw/config.toml"));
        assert!(unit.contains("gateway"));
        assert!(unit.contains("WantedBy=default.target"));
    }
}
