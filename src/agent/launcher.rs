//! CLI process lifecycle management
//!
//! Manages Claude Code CLI processes: spawn, kill, relaunch, and restore.
//! Each session maps to one CLI process connected via `--sdk-url` WebSocket.

use crate::agent::session_store::AgentSessionStore;
use crate::agent::types::{AgentProcessInfo, AgentProcessState};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, RwLock};

/// Manages CLI process lifecycles
pub struct AgentLauncher {
    /// Session metadata
    sessions: Arc<RwLock<HashMap<String, AgentProcessInfo>>>,
    /// Live process handles
    processes: Arc<RwLock<HashMap<String, Child>>>,
    /// Gateway listen port (for constructing --sdk-url)
    port: u16,
    /// Persistence store
    store: Arc<AgentSessionStore>,
    /// Channel to notify bridge when a session needs relaunch
    relaunch_tx: mpsc::Sender<String>,
}

impl AgentLauncher {
    /// Create a new launcher
    pub fn new(
        port: u16,
        store: Arc<AgentSessionStore>,
        relaunch_tx: mpsc::Sender<String>,
    ) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            processes: Arc::new(RwLock::new(HashMap::new())),
            port,
            store,
            relaunch_tx,
        }
    }

    /// Spawn a new CLI process for a session
    pub async fn spawn(
        &self,
        session_id: &str,
        model: Option<String>,
        permission_mode: Option<String>,
        cwd: Option<String>,
    ) -> crate::Result<AgentProcessInfo> {
        let binary = resolve_claude_binary()?;
        let working_dir = cwd.unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("/tmp"))
                .to_string_lossy()
                .to_string()
        });

        let sdk_url = format!(
            "ws://127.0.0.1:{}/ws/agent/cli/{}",
            self.port, session_id
        );

        let mut args = vec![
            "--sdk-url".to_string(),
            sdk_url,
            "--print".to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--input-format".to_string(),
            "stream-json".to_string(),
            "--verbose".to_string(),
        ];

        if let Some(ref m) = model {
            args.push("--model".to_string());
            args.push(m.clone());
        }
        if let Some(ref pm) = permission_mode {
            args.push("--permission-mode".to_string());
            args.push(pm.clone());
        }

        // Headless mode with empty prompt
        args.push("-p".to_string());
        args.push(String::new());

        tracing::info!(
            session_id = session_id,
            binary = %binary.display(),
            cwd = %working_dir,
            "Spawning Claude Code CLI"
        );

        let child = Command::new(&binary)
            .args(&args)
            .current_dir(&working_dir)
            .env("CLAUDECODE", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                crate::error::Error::Gateway(format!(
                    "Failed to spawn Claude Code CLI at {}: {}",
                    binary.display(),
                    e
                ))
            })?;

        let pid = child.id();
        let now = now_millis();

        let info = AgentProcessInfo {
            session_id: session_id.to_string(),
            pid,
            state: AgentProcessState::Starting,
            exit_code: None,
            model,
            permission_mode,
            cwd: working_dir,
            created_at: now,
            cli_session_id: None,
            archived: false,
            name: None,
        };

        self.sessions
            .write()
            .await
            .insert(session_id.to_string(), info.clone());
        self.processes
            .write()
            .await
            .insert(session_id.to_string(), child);
        self.persist_state().await;

        // Spawn exit monitor
        self.spawn_exit_monitor(session_id.to_string(), now, None);

        Ok(info)
    }

    /// Spawn a CLI process with --resume for session recovery
    async fn spawn_with_resume(
        &self,
        session_id: &str,
        info: &AgentProcessInfo,
    ) -> crate::Result<()> {
        let binary = resolve_claude_binary()?;
        let sdk_url = format!(
            "ws://127.0.0.1:{}/ws/agent/cli/{}",
            self.port, session_id
        );

        let mut args = vec![
            "--sdk-url".to_string(),
            sdk_url,
            "--print".to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--input-format".to_string(),
            "stream-json".to_string(),
            "--verbose".to_string(),
        ];

        if let Some(ref m) = info.model {
            args.push("--model".to_string());
            args.push(m.clone());
        }
        if let Some(ref pm) = info.permission_mode {
            args.push("--permission-mode".to_string());
            args.push(pm.clone());
        }

        let resume_id = info.cli_session_id.clone();
        if let Some(ref cli_sid) = resume_id {
            args.push("--resume".to_string());
            args.push(cli_sid.clone());
        }

        args.push("-p".to_string());
        args.push(String::new());

        tracing::info!(
            session_id = session_id,
            resume = ?resume_id,
            "Relaunching Claude Code CLI"
        );

        let child = Command::new(&binary)
            .args(&args)
            .current_dir(&info.cwd)
            .env("CLAUDECODE", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                crate::error::Error::Gateway(format!(
                    "Failed to relaunch Claude Code CLI: {}",
                    e
                ))
            })?;

        let pid = child.id();
        let now = now_millis();

        {
            let mut sessions = self.sessions.write().await;
            if let Some(s) = sessions.get_mut(session_id) {
                s.pid = pid;
                s.state = AgentProcessState::Starting;
                s.exit_code = None;
            }
        }

        self.processes
            .write()
            .await
            .insert(session_id.to_string(), child);
        self.persist_state().await;

        self.spawn_exit_monitor(session_id.to_string(), now, resume_id);

        Ok(())
    }

    /// Mark a session as connected (called by bridge when CLI WS connects)
    pub async fn mark_connected(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(info) = sessions.get_mut(session_id) {
            info.state = AgentProcessState::Connected;
            tracing::info!(session_id = session_id, "CLI process connected");
        }
        drop(sessions);
        self.persist_state().await;
    }

    /// Store the CLI's internal session ID (from system.init)
    pub async fn set_cli_session_id(&self, session_id: &str, cli_session_id: String) {
        let mut sessions = self.sessions.write().await;
        if let Some(info) = sessions.get_mut(session_id) {
            info.cli_session_id = Some(cli_session_id);
        }
        drop(sessions);
        self.persist_state().await;
    }

    /// Kill a CLI process
    pub async fn kill(&self, session_id: &str) -> crate::Result<()> {
        let mut processes = self.processes.write().await;
        if let Some(mut child) = processes.remove(session_id) {
            tracing::info!(session_id = session_id, "Killing CLI process");

            // Try graceful shutdown first
            let _ = child.start_kill();

            // Wait up to 5 seconds
            let result = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;

            if result.is_err() {
                // Force kill
                tracing::warn!(session_id = session_id, "Force killing CLI process");
                let _ = child.kill().await;
            }
        }

        let mut sessions = self.sessions.write().await;
        if let Some(info) = sessions.get_mut(session_id) {
            info.state = AgentProcessState::Exited;
            info.exit_code = Some(-1);
        }
        drop(sessions);
        drop(processes);
        self.persist_state().await;

        Ok(())
    }

    /// Relaunch a CLI process (kill old, spawn new with --resume)
    pub async fn relaunch(&self, session_id: &str) -> crate::Result<()> {
        let info = {
            let sessions = self.sessions.read().await;
            sessions.get(session_id).cloned()
        };

        let info = info.ok_or_else(|| {
            crate::error::Error::Gateway(format!("Session not found: {}", session_id))
        })?;

        // Kill existing process
        self.kill(session_id).await?;

        // Spawn new process with resume
        self.spawn_with_resume(session_id, &info).await
    }

    /// Get info for a session
    pub async fn get_session(&self, session_id: &str) -> Option<AgentProcessInfo> {
        self.sessions.read().await.get(session_id).cloned()
    }

    /// Get all sessions
    pub async fn all_sessions(&self) -> Vec<AgentProcessInfo> {
        self.sessions.read().await.values().cloned().collect()
    }

    /// Remove a session entirely
    pub async fn remove_session(&self, session_id: &str) {
        self.sessions.write().await.remove(session_id);
        self.processes.write().await.remove(session_id);
        self.persist_state().await;
    }

    /// Set session name
    pub async fn set_name(&self, session_id: &str, name: String) {
        let mut sessions = self.sessions.write().await;
        if let Some(info) = sessions.get_mut(session_id) {
            info.name = Some(name);
        }
        drop(sessions);
        self.persist_state().await;
    }

    /// Set session archived status
    pub async fn set_archived(&self, session_id: &str, archived: bool) {
        let mut sessions = self.sessions.write().await;
        if let Some(info) = sessions.get_mut(session_id) {
            info.archived = archived;
        }
        drop(sessions);
        self.persist_state().await;
    }

    /// Restore sessions from disk after server restart
    pub async fn restore_from_disk(&self) {
        let infos = match self.store.load_launcher() {
            Some(infos) => infos,
            None => {
                tracing::debug!("No launcher state to restore");
                return;
            }
        };

        tracing::info!("Restoring {} agent sessions from disk", infos.len());

        let mut sessions = self.sessions.write().await;
        for mut info in infos {
            // Check if process is still alive
            if let Some(pid) = info.pid {
                if is_process_alive(pid) {
                    info.state = AgentProcessState::Starting; // Wait for WS reconnect
                    tracing::debug!(
                        session_id = %info.session_id,
                        pid = pid,
                        "Process still alive, waiting for reconnect"
                    );
                } else {
                    info.state = AgentProcessState::Exited;
                    info.exit_code = Some(-1);
                    tracing::debug!(
                        session_id = %info.session_id,
                        pid = pid,
                        "Process dead"
                    );
                }
            } else {
                info.state = AgentProcessState::Exited;
            }
            sessions.insert(info.session_id.clone(), info);
        }
    }

    /// Start the reconnect watchdog (10s timer)
    pub fn start_reconnect_watchdog(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let launcher = Arc::clone(self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;

                let stale_sessions: Vec<String> = {
                    let sessions = launcher.sessions.read().await;
                    sessions
                        .iter()
                        .filter(|(_, info)| {
                            info.state == AgentProcessState::Starting && !info.archived
                        })
                        .map(|(id, _)| id.clone())
                        .collect()
                };

                for session_id in stale_sessions {
                    tracing::info!(
                        session_id = %session_id,
                        "Reconnect watchdog: relaunching stale session"
                    );
                    if let Err(e) = launcher.relaunch(&session_id).await {
                        tracing::warn!(
                            session_id = %session_id,
                            "Failed to relaunch stale session: {}",
                            e
                        );
                    }
                }
            }
        })
    }

    /// Persist current session state to disk
    async fn persist_state(&self) {
        let infos: Vec<AgentProcessInfo> =
            self.sessions.read().await.values().cloned().collect();
        self.store.save_launcher(&infos);
    }

    /// Spawn a task to monitor process exit
    fn spawn_exit_monitor(
        &self,
        session_id: String,
        spawned_at: u64,
        resume_session_id: Option<String>,
    ) {
        let sessions = self.sessions.clone();
        let processes = self.processes.clone();
        let store = self.store.clone();

        tokio::spawn(async move {
            // Wait for the process to exit
            let exit_code = {
                let mut procs = processes.write().await;
                if let Some(child) = procs.get_mut(&session_id) {
                    match child.wait().await {
                        Ok(status) => status.code(),
                        Err(_) => Some(-1),
                    }
                } else {
                    return;
                }
            };

            let uptime_ms = now_millis().saturating_sub(spawned_at);

            tracing::info!(
                session_id = %session_id,
                exit_code = ?exit_code,
                uptime_ms = uptime_ms,
                "CLI process exited"
            );

            // If process exited within 5s and was a resume, clear cli_session_id
            {
                let mut guard = sessions.write().await;
                if let Some(info) = guard.get_mut(&session_id) {
                    info.state = AgentProcessState::Exited;
                    info.exit_code = exit_code;

                    if uptime_ms < 5000 && resume_session_id.is_some() {
                        tracing::warn!(
                            session_id = %session_id,
                            "Resume failed (exited in {}ms), clearing cli_session_id",
                            uptime_ms
                        );
                        info.cli_session_id = None;
                    }
                }
            }

            // Remove from processes map
            processes.write().await.remove(&session_id);

            // Persist
            let infos: Vec<AgentProcessInfo> =
                sessions.read().await.values().cloned().collect();
            store.save_launcher(&infos);
        });
    }
}

/// Resolve the Claude Code CLI binary path
fn resolve_claude_binary() -> crate::Result<PathBuf> {
    // Check PATH via `which`
    if let Ok(output) = std::process::Command::new("which")
        .arg("claude")
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(PathBuf::from(path));
            }
        }
    }

    // Check common locations
    let home = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let candidates = [
        home.join(".claude").join("local").join("claude"),
        PathBuf::from("/usr/local/bin/claude"),
        PathBuf::from("/opt/homebrew/bin/claude"),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return Ok(candidate.clone());
        }
    }

    Err(crate::error::Error::Gateway(
        "Claude Code CLI not found. Install it from https://claude.ai/code".to_string(),
    ))
}

/// Check if a process is alive by sending signal 0
fn is_process_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

/// Current time in milliseconds since UNIX epoch
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now_millis() {
        let now = now_millis();
        assert!(now > 1_700_000_000_000); // After 2023
    }

    #[test]
    fn test_is_process_alive_self() {
        let pid = std::process::id();
        assert!(is_process_alive(pid));
    }

    #[test]
    fn test_is_process_alive_nonexistent() {
        // PID 99999999 is very unlikely to exist
        assert!(!is_process_alive(99_999_999));
    }

    #[test]
    fn test_resolve_claude_binary() {
        // This test just verifies the function doesn't panic
        // It may or may not find claude depending on the environment
        let _result = resolve_claude_binary();
    }

    #[tokio::test]
    async fn test_launcher_session_lifecycle() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = Arc::new(AgentSessionStore::new(dir.path().to_path_buf()));
        std::fs::create_dir_all(dir.path()).unwrap();
        let (tx, _rx) = mpsc::channel(10);

        let launcher = AgentLauncher::new(3456, store, tx);

        // No sessions initially
        assert!(launcher.all_sessions().await.is_empty());
        assert!(launcher.get_session("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_launcher_mark_connected() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = Arc::new(AgentSessionStore::new(dir.path().to_path_buf()));
        std::fs::create_dir_all(dir.path()).unwrap();
        let (tx, _rx) = mpsc::channel(10);

        let launcher = AgentLauncher::new(3456, store, tx);

        // Insert a fake session
        {
            let mut sessions = launcher.sessions.write().await;
            sessions.insert(
                "test-1".to_string(),
                AgentProcessInfo {
                    session_id: "test-1".to_string(),
                    pid: Some(1),
                    state: AgentProcessState::Starting,
                    exit_code: None,
                    model: None,
                    permission_mode: None,
                    cwd: "/tmp".to_string(),
                    created_at: now_millis(),
                    cli_session_id: None,
                    archived: false,
                    name: None,
                },
            );
        }

        launcher.mark_connected("test-1").await;

        let info = launcher.get_session("test-1").await.unwrap();
        assert_eq!(info.state, AgentProcessState::Connected);
    }

    #[tokio::test]
    async fn test_launcher_set_cli_session_id() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = Arc::new(AgentSessionStore::new(dir.path().to_path_buf()));
        std::fs::create_dir_all(dir.path()).unwrap();
        let (tx, _rx) = mpsc::channel(10);

        let launcher = AgentLauncher::new(3456, store, tx);

        {
            let mut sessions = launcher.sessions.write().await;
            sessions.insert(
                "test-1".to_string(),
                AgentProcessInfo {
                    session_id: "test-1".to_string(),
                    pid: Some(1),
                    state: AgentProcessState::Connected,
                    exit_code: None,
                    model: None,
                    permission_mode: None,
                    cwd: "/tmp".to_string(),
                    created_at: now_millis(),
                    cli_session_id: None,
                    archived: false,
                    name: None,
                },
            );
        }

        launcher
            .set_cli_session_id("test-1", "cli-abc".to_string())
            .await;

        let info = launcher.get_session("test-1").await.unwrap();
        assert_eq!(info.cli_session_id.as_deref(), Some("cli-abc"));
    }

    #[tokio::test]
    async fn test_launcher_set_name() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = Arc::new(AgentSessionStore::new(dir.path().to_path_buf()));
        std::fs::create_dir_all(dir.path()).unwrap();
        let (tx, _rx) = mpsc::channel(10);

        let launcher = AgentLauncher::new(3456, store, tx);

        {
            let mut sessions = launcher.sessions.write().await;
            sessions.insert(
                "test-1".to_string(),
                AgentProcessInfo {
                    session_id: "test-1".to_string(),
                    pid: None,
                    state: AgentProcessState::Exited,
                    exit_code: None,
                    model: None,
                    permission_mode: None,
                    cwd: "/tmp".to_string(),
                    created_at: now_millis(),
                    cli_session_id: None,
                    archived: false,
                    name: None,
                },
            );
        }

        launcher
            .set_name("test-1", "My Session".to_string())
            .await;

        let info = launcher.get_session("test-1").await.unwrap();
        assert_eq!(info.name.as_deref(), Some("My Session"));
    }

    #[tokio::test]
    async fn test_launcher_set_archived() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = Arc::new(AgentSessionStore::new(dir.path().to_path_buf()));
        std::fs::create_dir_all(dir.path()).unwrap();
        let (tx, _rx) = mpsc::channel(10);

        let launcher = AgentLauncher::new(3456, store, tx);

        {
            let mut sessions = launcher.sessions.write().await;
            sessions.insert(
                "test-1".to_string(),
                AgentProcessInfo {
                    session_id: "test-1".to_string(),
                    pid: None,
                    state: AgentProcessState::Exited,
                    exit_code: None,
                    model: None,
                    permission_mode: None,
                    cwd: "/tmp".to_string(),
                    created_at: now_millis(),
                    cli_session_id: None,
                    archived: false,
                    name: None,
                },
            );
        }

        launcher.set_archived("test-1", true).await;
        let info = launcher.get_session("test-1").await.unwrap();
        assert!(info.archived);

        launcher.set_archived("test-1", false).await;
        let info = launcher.get_session("test-1").await.unwrap();
        assert!(!info.archived);
    }

    #[tokio::test]
    async fn test_launcher_remove_session() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = Arc::new(AgentSessionStore::new(dir.path().to_path_buf()));
        std::fs::create_dir_all(dir.path()).unwrap();
        let (tx, _rx) = mpsc::channel(10);

        let launcher = AgentLauncher::new(3456, store, tx);

        {
            let mut sessions = launcher.sessions.write().await;
            sessions.insert(
                "test-1".to_string(),
                AgentProcessInfo {
                    session_id: "test-1".to_string(),
                    pid: None,
                    state: AgentProcessState::Exited,
                    exit_code: None,
                    model: None,
                    permission_mode: None,
                    cwd: "/tmp".to_string(),
                    created_at: now_millis(),
                    cli_session_id: None,
                    archived: false,
                    name: None,
                },
            );
        }

        launcher.remove_session("test-1").await;
        assert!(launcher.get_session("test-1").await.is_none());
    }

    #[tokio::test]
    async fn test_launcher_restore_from_disk_empty() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = Arc::new(AgentSessionStore::new(dir.path().to_path_buf()));
        std::fs::create_dir_all(dir.path()).unwrap();
        let (tx, _rx) = mpsc::channel(10);

        let launcher = AgentLauncher::new(3456, store, tx);
        launcher.restore_from_disk().await;

        assert!(launcher.all_sessions().await.is_empty());
    }
}
