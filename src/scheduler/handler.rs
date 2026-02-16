//! REST API handler for the proactive task scheduler
//!
//! Provides CRUD endpoints for scheduled tasks and manual trigger.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::TaskScheduler;

/// Shared state for the scheduler router
#[derive(Clone)]
pub struct SchedulerState {
    pub scheduler: Arc<TaskScheduler>,
}

/// Create task request body
#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub name: String,
    pub schedule: String,
    pub prompt: String,
    pub channel: String,
    pub chat_id: String,
    #[serde(default)]
    pub delivery: crate::config::DeliveryMode,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    120_000
}

/// Task response
#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub id: String,
    pub name: String,
    pub schedule: String,
    pub prompt: String,
    pub channel: String,
    pub chat_id: String,
    pub delivery: String,
    pub status: String,
    pub last_run: Option<String>,
    pub next_run: Option<String>,
    pub run_count: u64,
    pub fail_count: u64,
}

/// Build the scheduler router
pub fn scheduler_router(state: SchedulerState) -> Router {
    Router::new()
        .route("/scheduler/tasks", get(list_tasks).post(create_task))
        .route(
            "/scheduler/tasks/:id",
            get(get_task).delete(delete_task),
        )
        .route("/scheduler/tasks/:id/run", post(run_task))
        .route("/scheduler/tasks/:id/pause", post(pause_task))
        .route("/scheduler/tasks/:id/resume", post(resume_task))
        .route("/scheduler/tasks/:id/history", get(get_history))
        .with_state(state)
}

/// GET /scheduler/tasks — list all scheduled tasks
async fn list_tasks(
    State(state): State<SchedulerState>,
) -> Result<impl IntoResponse, StatusCode> {
    let jobs = state
        .scheduler
        .cron()
        .list_jobs()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tasks: Vec<TaskResponse> = jobs.into_iter().map(job_to_response).collect();
    Ok(Json(tasks))
}

/// POST /scheduler/tasks — create a new scheduled task
async fn create_task(
    State(state): State<SchedulerState>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let agent_config = a3s_cron::AgentJobConfig {
        model: String::new(),
        api_key: String::new(),
        workspace: None,
        system_prompt: None,
        base_url: None,
    };

    let job = state
        .scheduler
        .cron()
        .add_agent_job(&req.name, &req.schedule, &req.prompt, agent_config)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create scheduled task");
            StatusCode::BAD_REQUEST
        })?;

    Ok((StatusCode::CREATED, Json(job_to_response(job))))
}

/// GET /scheduler/tasks/:id — get a single task
async fn get_task(
    State(state): State<SchedulerState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let job = state
        .scheduler
        .cron()
        .get_job(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(job_to_response(job)))
}

/// DELETE /scheduler/tasks/:id — remove a task
async fn delete_task(
    State(state): State<SchedulerState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    state
        .scheduler
        .cron()
        .remove_job(&id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /scheduler/tasks/:id/run — manually trigger a task
async fn run_task(
    State(state): State<SchedulerState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let execution = state
        .scheduler
        .cron()
        .run_job(&id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(serde_json::json!({
        "execution_id": execution.id,
        "status": execution.status.to_string(),
        "started_at": execution.started_at.to_rfc3339(),
    })))
}

/// POST /scheduler/tasks/:id/pause — pause a task
async fn pause_task(
    State(state): State<SchedulerState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let job = state
        .scheduler
        .cron()
        .pause_job(&id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(job_to_response(job)))
}

/// POST /scheduler/tasks/:id/resume — resume a paused task
async fn resume_task(
    State(state): State<SchedulerState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let job = state
        .scheduler
        .cron()
        .resume_job(&id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(job_to_response(job)))
}

/// GET /scheduler/tasks/:id/history — get execution history
async fn get_history(
    State(state): State<SchedulerState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let history = state
        .scheduler
        .cron()
        .get_history(&id, 50)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let entries: Vec<serde_json::Value> = history
        .into_iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id,
                "status": e.status.to_string(),
                "started_at": e.started_at.to_rfc3339(),
                "ended_at": e.ended_at.map(|t| t.to_rfc3339()),
                "duration_ms": e.duration_ms,
                "error": e.error,
            })
        })
        .collect();

    Ok(Json(entries))
}

/// Convert a CronJob to a TaskResponse
fn job_to_response(job: a3s_cron::CronJob) -> TaskResponse {
    TaskResponse {
        id: job.id,
        name: job.name,
        schedule: job.schedule,
        prompt: job.command,
        channel: String::new(), // Channel info is in task_defs, not in CronJob
        chat_id: String::new(),
        delivery: "full".to_string(),
        status: job.status.to_string(),
        last_run: job.last_run.map(|t| t.to_rfc3339()),
        next_run: job.next_run.map(|t| t.to_rfc3339()),
        run_count: job.run_count,
        fail_count: job.fail_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_task_request_serde() {
        let json = serde_json::json!({
            "name": "daily-check",
            "schedule": "0 9 * * *",
            "prompt": "Check system health",
            "channel": "telegram",
            "chat_id": "12345"
        });
        let req: CreateTaskRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.name, "daily-check");
        assert_eq!(req.timeout_ms, 120_000);
        assert_eq!(req.delivery, crate::config::DeliveryMode::Full);
    }

    #[test]
    fn test_task_response_serialize() {
        let resp = TaskResponse {
            id: "abc".to_string(),
            name: "test".to_string(),
            schedule: "* * * * *".to_string(),
            prompt: "hello".to_string(),
            channel: "telegram".to_string(),
            chat_id: "123".to_string(),
            delivery: "full".to_string(),
            status: "active".to_string(),
            last_run: None,
            next_run: Some("2025-01-01T00:00:00+00:00".to_string()),
            run_count: 5,
            fail_count: 1,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["name"], "test");
        assert_eq!(json["run_count"], 5);
    }
}
