//! Proactive Task Scheduler (Phase 14)
//!
//! Integrates `a3s-cron` with SafeClaw's agent engine and channel adapters
//! to run scheduled agent tasks autonomously and deliver results to channels.

mod handler;

use crate::agent::AgentEngine;
use crate::channels::{ChannelAdapter, OutboundMessage};
use crate::config::{DeliveryMode, ScheduledTaskDef, SchedulerConfig};
use a3s_cron::{AgentExecutor, AgentJobConfig, CronManager};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub use handler::{scheduler_router, SchedulerState};

/// Wraps `AgentEngine` to implement the `AgentExecutor` trait for cron jobs.
struct EngineExecutor {
    engine: Arc<AgentEngine>,
}

#[async_trait::async_trait]
impl AgentExecutor for EngineExecutor {
    async fn execute(
        &self,
        _config: &AgentJobConfig,
        prompt: &str,
        _working_dir: &str,
    ) -> Result<String, String> {
        // Use a dedicated session prefix so scheduled runs don't collide
        // with interactive sessions.
        let session_id = format!("sched-{}", uuid::Uuid::new_v4());
        self.engine
            .generate_response(&session_id, prompt)
            .await
            .map_err(|e| format!("Agent execution failed: {}", e))
    }
}

/// Proactive task scheduler that runs agent prompts on cron schedules
/// and delivers results to configured channels.
pub struct TaskScheduler {
    cron: Arc<CronManager>,
    /// Channel adapters keyed by channel name (e.g., "telegram")
    channels: Arc<HashMap<String, Arc<dyn ChannelAdapter>>>,
    /// Last result per task name (for diff delivery mode)
    last_results: Arc<RwLock<HashMap<String, String>>>,
    config: SchedulerConfig,
}

impl TaskScheduler {
    /// Create a new task scheduler.
    ///
    /// `channels` maps channel names to their adapters for result delivery.
    pub async fn new(
        engine: Arc<AgentEngine>,
        channels: HashMap<String, Arc<dyn ChannelAdapter>>,
        config: SchedulerConfig,
        workspace: &str,
    ) -> crate::Result<Self> {
        let mut cron = CronManager::new(workspace).await.map_err(|e| {
            crate::Error::Gateway(format!("Failed to create CronManager: {}", e))
        })?;

        cron.set_agent_executor(Arc::new(EngineExecutor { engine }));

        Ok(Self {
            cron: Arc::new(cron),
            channels: Arc::new(channels),
            last_results: Arc::new(RwLock::new(HashMap::new())),
            config,
        })
    }

    /// Get a reference to the underlying CronManager.
    pub fn cron(&self) -> &Arc<CronManager> {
        &self.cron
    }

    /// Register all tasks from config and start the scheduler loop.
    pub async fn start(&self) -> crate::Result<()> {
        // Register configured tasks as agent-mode cron jobs
        for task in &self.config.tasks {
            if let Err(e) = self.register_task(task).await {
                tracing::warn!(task = %task.name, error = %e, "Failed to register scheduled task");
            }
        }

        // Start the cron tick loop
        self.cron.start().await.map_err(|e| {
            crate::Error::Gateway(format!("Failed to start scheduler: {}", e))
        })?;

        // Spawn result delivery listener
        self.spawn_delivery_loop();

        tracing::info!(
            tasks = self.config.tasks.len(),
            "Proactive task scheduler started"
        );
        Ok(())
    }

    /// Register a single task definition as a cron agent job.
    async fn register_task(&self, task: &ScheduledTaskDef) -> crate::Result<()> {
        // Build a minimal AgentJobConfig — the actual execution goes through
        // EngineExecutor which ignores these fields, but CronManager requires them.
        let agent_config = AgentJobConfig {
            model: String::new(),
            api_key: String::new(),
            workspace: None,
            system_prompt: None,
            base_url: None,
        };

        let mut job = self
            .cron
            .add_agent_job(&task.name, &task.schedule, &task.prompt, agent_config)
            .await
            .map_err(|e| {
                crate::Error::Gateway(format!("Failed to add task '{}': {}", task.name, e))
            })?;

        job.timeout_ms = task.timeout_ms;

        tracing::info!(
            name = %task.name,
            schedule = %task.schedule,
            channel = %task.channel,
            chat_id = %task.chat_id,
            "Registered scheduled task"
        );
        Ok(())
    }

    /// Spawn a background task that listens for job completion events
    /// and delivers results to the configured channels.
    fn spawn_delivery_loop(&self) {
        let mut event_rx = self.cron.subscribe();
        let cron = self.cron.clone();
        let channels = self.channels.clone();
        let last_results = self.last_results.clone();

        // Build a lookup from job name → task def for delivery config
        let task_defs: HashMap<String, ScheduledTaskDef> = self
            .config
            .tasks
            .iter()
            .map(|t| (t.name.clone(), t.clone()))
            .collect();

        tokio::spawn(async move {
            loop {
                match event_rx.recv().await {
                    Ok(a3s_cron::SchedulerEvent::JobCompleted {
                        job_id,
                        execution_id,
                    }) => {
                        if let Err(e) = deliver_result(
                            &cron,
                            &channels,
                            &last_results,
                            &task_defs,
                            &job_id,
                            &execution_id,
                        )
                        .await
                        {
                            tracing::error!(
                                job_id = %job_id,
                                error = %e,
                                "Failed to deliver scheduled task result"
                            );
                        }
                    }
                    Ok(a3s_cron::SchedulerEvent::JobFailed {
                        job_id, error, ..
                    }) => {
                        // Deliver error notification
                        if let Ok(Some(job)) = cron.get_job(&job_id).await {
                            if let Some(task_def) = task_defs.get(&job.name) {
                                if let Some(adapter) = channels.get(&task_def.channel) {
                                    let msg = OutboundMessage::new(
                                        &task_def.channel,
                                        &task_def.chat_id,
                                        &format!(
                                            "⚠️ Scheduled task **{}** failed:\n```\n{}\n```",
                                            job.name, error
                                        ),
                                    );
                                    if let Err(e) = adapter.send_message(msg).await {
                                        tracing::error!(
                                            task = %job.name,
                                            error = %e,
                                            "Failed to deliver error notification"
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Ok(a3s_cron::SchedulerEvent::Stopped) => {
                        tracing::info!("Scheduler delivery loop stopped");
                        break;
                    }
                    Ok(_) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(skipped = n, "Scheduler delivery loop lagged");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        });
    }

    /// Stop the scheduler.
    pub async fn stop(&self) {
        self.cron.stop().await;
    }
}

/// Deliver a successful job result to the configured channel.
async fn deliver_result(
    cron: &CronManager,
    channels: &HashMap<String, Arc<dyn ChannelAdapter>>,
    last_results: &RwLock<HashMap<String, String>>,
    task_defs: &HashMap<String, ScheduledTaskDef>,
    job_id: &str,
    execution_id: &str,
) -> crate::Result<()> {
    let job = cron
        .get_job(job_id)
        .await
        .map_err(|e| crate::Error::Gateway(format!("Failed to load job: {}", e)))?
        .ok_or_else(|| crate::Error::Gateway(format!("Job {} not found", job_id)))?;

    let task_def = task_defs
        .get(&job.name)
        .ok_or_else(|| crate::Error::Gateway(format!("No task def for job '{}'", job.name)))?;

    let adapter = channels
        .get(&task_def.channel)
        .ok_or_else(|| {
            crate::Error::Gateway(format!("Channel '{}' not available", task_def.channel))
        })?;

    // Get execution output
    let history = cron
        .get_history(job_id, 1)
        .await
        .map_err(|e| crate::Error::Gateway(format!("Failed to load history: {}", e)))?;

    let execution = history
        .into_iter()
        .find(|e| e.id == execution_id)
        .ok_or_else(|| {
            crate::Error::Gateway(format!("Execution {} not found", execution_id))
        })?;

    let output = if execution.stdout.is_empty() {
        "(no output)".to_string()
    } else {
        execution.stdout.clone()
    };

    // Apply delivery mode
    let content = match task_def.delivery {
        DeliveryMode::Full => {
            format!("📋 **{}**\n\n{}", job.name, output)
        }
        DeliveryMode::Summary => {
            let summary = if output.len() > 500 {
                format!("{}…", &output[..500])
            } else {
                output.clone()
            };
            format!("📋 **{}** (summary)\n\n{}", job.name, summary)
        }
        DeliveryMode::Diff => {
            let mut last = last_results.write().await;
            let prev = last.get(&job.name);
            if prev.map(|p| p == &output).unwrap_or(false) {
                // No change — skip delivery
                return Ok(());
            }
            last.insert(job.name.clone(), output.clone());
            format!("📋 **{}** (changed)\n\n{}", job.name, output)
        }
    };

    let msg = OutboundMessage::new(&task_def.channel, &task_def.chat_id, &content);
    adapter.send_message(msg).await.map_err(|e| {
        crate::Error::Gateway(format!("Failed to send to {}: {}", task_def.channel, e))
    })?;

    tracing::info!(
        task = %job.name,
        channel = %task_def.channel,
        "Delivered scheduled task result"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DeliveryMode;

    #[test]
    fn test_delivery_mode_default() {
        let mode = DeliveryMode::default();
        assert_eq!(mode, DeliveryMode::Full);
    }

    #[test]
    fn test_scheduler_config_default() {
        let config = SchedulerConfig::default();
        assert!(!config.enabled);
        assert!(config.tasks.is_empty());
    }

    #[test]
    fn test_scheduled_task_def_serde() {
        let json = serde_json::json!({
            "name": "daily-report",
            "schedule": "0 9 * * *",
            "prompt": "Generate a daily status report",
            "channel": "telegram",
            "chat_id": "12345",
            "delivery": "summary",
            "timeout_ms": 60000
        });
        let task: ScheduledTaskDef = serde_json::from_value(json).unwrap();
        assert_eq!(task.name, "daily-report");
        assert_eq!(task.schedule, "0 9 * * *");
        assert_eq!(task.delivery, DeliveryMode::Summary);
        assert_eq!(task.timeout_ms, 60000);
    }

    #[test]
    fn test_scheduled_task_def_defaults() {
        let json = serde_json::json!({
            "name": "test",
            "schedule": "* * * * *",
            "prompt": "hello",
            "channel": "webchat",
            "chat_id": "c1"
        });
        let task: ScheduledTaskDef = serde_json::from_value(json).unwrap();
        assert_eq!(task.delivery, DeliveryMode::Full);
        assert_eq!(task.timeout_ms, 120_000);
    }
}
