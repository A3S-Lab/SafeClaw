//! SafeClaw - Secure Personal AI Assistant with TEE Support
//!
//! A privacy-focused personal AI assistant that combines multi-channel
//! messaging capabilities with hardware-isolated execution for sensitive data.

use anyhow::Result;
use clap::{Parser, Subcommand};
use safeclaw::{
    config::SafeClawConfig,
    gateway::{Gateway, GatewayBuilder},
};
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
        } => {
            send_message(&channel, &to, &message).await?;
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

async fn run_gateway(config: SafeClawConfig, host: String, port: u16, tee_enabled: bool) -> Result<()> {
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

async fn run_onboard(install_daemon: bool) -> Result<()> {
    println!("🦞 Welcome to SafeClaw Onboarding!");
    println!();
    println!("SafeClaw is a secure personal AI assistant that protects your privacy");
    println!("by running sensitive computations in a Trusted Execution Environment (TEE).");
    println!();

    // TODO: Implement interactive onboarding wizard
    println!("Onboarding wizard coming soon...");

    if install_daemon {
        println!();
        println!("Daemon installation coming soon...");
    }

    Ok(())
}

async fn send_message(channel: &str, to: &str, message: &str) -> Result<()> {
    println!("Sending message to {} on {}: {}", to, channel, message);

    // TODO: Connect to running gateway and send message
    println!("Message sending coming soon...");

    Ok(())
}

async fn run_doctor() -> Result<()> {
    println!("🔍 SafeClaw Doctor");
    println!();

    // Check TEE availability
    println!("Checking TEE availability...");
    #[cfg(target_os = "macos")]
    {
        println!("  ✓ macOS detected - Apple Hypervisor Framework available");
    }
    #[cfg(target_os = "linux")]
    {
        println!("  Checking KVM...");
        if std::path::Path::new("/dev/kvm").exists() {
            println!("  ✓ KVM available");
        } else {
            println!("  ✗ KVM not available");
        }
    }

    // Check configuration
    println!();
    println!("Checking configuration...");
    let config_path = dirs::config_dir()
        .map(|p| p.join("safeclaw").join("config.toml"));
    if let Some(path) = config_path {
        if path.exists() {
            println!("  ✓ Configuration file found: {}", path.display());
        } else {
            println!("  ℹ No configuration file found (using defaults)");
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
                .or_else(|| std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config")))
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
