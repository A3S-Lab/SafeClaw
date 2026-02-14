//! HTTP handlers for the Privacy API
//!
//! Provides REST endpoints for privacy classification, semantic analysis,
//! and compliance rule management:
//! - POST /api/v1/privacy/classify   — regex-based classification
//! - POST /api/v1/privacy/analyze    — semantic PII disclosure detection
//! - POST /api/v1/privacy/scan       — combined scan (regex + semantic + compliance)
//! - GET  /api/v1/privacy/compliance/frameworks — list available frameworks
//! - GET  /api/v1/privacy/compliance/rules      — list rules (filterable by framework)

use crate::config::SensitivityLevel;
use crate::events::types::ApiError;
use crate::privacy::classifier::Classifier;
use crate::privacy::compliance::{ComplianceEngine, ComplianceFramework};
use crate::privacy::semantic::{SemanticAnalyzer, SemanticMatch};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Shared state for privacy handlers
#[derive(Clone)]
pub struct PrivacyState {
    pub classifier: Arc<Classifier>,
    pub semantic: Arc<SemanticAnalyzer>,
    pub compliance: Arc<ComplianceEngine>,
}

/// Create the privacy router
pub fn privacy_router(state: PrivacyState) -> Router {
    Router::new()
        .route("/api/v1/privacy/classify", post(classify))
        .route("/api/v1/privacy/analyze", post(analyze))
        .route("/api/v1/privacy/scan", post(scan))
        .route(
            "/api/v1/privacy/compliance/frameworks",
            get(list_frameworks),
        )
        .route("/api/v1/privacy/compliance/rules", get(list_rules))
        .with_state(state)
}

// =============================================================================
// Request / Response types
// =============================================================================

/// Request body for classification endpoints
#[derive(Debug, Deserialize)]
pub struct ClassifyRequest {
    pub text: String,
}

/// Response from regex classification
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassifyResponse {
    pub level: String,
    pub requires_tee: bool,
    pub matches: Vec<ClassifyMatch>,
}

/// A single regex classification match
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassifyMatch {
    pub rule: String,
    pub level: String,
    pub start: usize,
    pub end: usize,
    pub redacted: String,
}

/// Response from semantic analysis
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeResponse {
    pub level: String,
    pub requires_tee: bool,
    pub matches: Vec<SemanticMatchResponse>,
}

/// A single semantic match in the response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticMatchResponse {
    pub category: String,
    pub trigger: String,
    pub redacted_value: String,
    pub level: String,
    pub confidence: f64,
    pub start: usize,
    pub end: usize,
}

impl From<&SemanticMatch> for SemanticMatchResponse {
    fn from(m: &SemanticMatch) -> Self {
        Self {
            category: format!("{:?}", m.category),
            trigger: m.trigger.clone(),
            redacted_value: m.redacted_value.clone(),
            level: format!("{:?}", m.level),
            confidence: m.confidence,
            start: m.start,
            end: m.end,
        }
    }
}

/// Combined scan response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResponse {
    pub level: String,
    pub requires_tee: bool,
    pub regex_matches: Vec<ClassifyMatch>,
    pub semantic_matches: Vec<SemanticMatchResponse>,
    pub compliance_matches: Vec<ComplianceMatch>,
}

/// A compliance rule match
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceMatch {
    pub rule: String,
    pub framework: String,
    pub description: String,
    pub level: String,
}

/// Query params for listing compliance rules
#[derive(Debug, Deserialize)]
pub struct RulesQuery {
    pub framework: Option<String>,
}

/// Framework info response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameworkInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub rule_count: usize,
    pub tee_mandatory: bool,
}

/// Rule info response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleInfo {
    pub name: String,
    pub framework: String,
    pub pattern: String,
    pub level: String,
    pub description: String,
}

// =============================================================================
// Helpers
// =============================================================================

/// Convert SensitivityLevel to a numeric rank for comparison
fn level_rank(level: SensitivityLevel) -> u8 {
    match level {
        SensitivityLevel::Public => 0,
        SensitivityLevel::Normal => 1,
        SensitivityLevel::Sensitive => 2,
        SensitivityLevel::HighlySensitive => 3,
        SensitivityLevel::Critical => 4,
    }
}

// =============================================================================
// Handlers
// =============================================================================

/// POST /api/v1/privacy/classify — regex-based classification
async fn classify(
    State(state): State<PrivacyState>,
    Json(request): Json<ClassifyRequest>,
) -> impl IntoResponse {
    let result = state.classifier.classify(&request.text);

    let matches = result
        .matches
        .iter()
        .map(|m| ClassifyMatch {
            rule: m.rule_name.clone(),
            level: format!("{:?}", m.level),
            start: m.start,
            end: m.end,
            redacted: m.redacted.clone(),
        })
        .collect();

    Json(ClassifyResponse {
        level: format!("{:?}", result.level),
        requires_tee: level_rank(result.level) >= level_rank(SensitivityLevel::Sensitive),
        matches,
    })
}

/// POST /api/v1/privacy/analyze — semantic PII disclosure detection
async fn analyze(
    State(state): State<PrivacyState>,
    Json(request): Json<ClassifyRequest>,
) -> impl IntoResponse {
    let result = state.semantic.analyze(&request.text);

    Json(AnalyzeResponse {
        level: format!("{:?}", result.level),
        requires_tee: result.requires_tee,
        matches: result.matches.iter().map(SemanticMatchResponse::from).collect(),
    })
}

/// POST /api/v1/privacy/scan — combined scan (regex + semantic + compliance)
async fn scan(
    State(state): State<PrivacyState>,
    Json(request): Json<ClassifyRequest>,
) -> impl IntoResponse {
    // Run all three analyzers
    let regex_result = state.classifier.classify(&request.text);
    let semantic_result = state.semantic.analyze(&request.text);

    // Run compliance rules via regex
    let compliance_rules = state.compliance.all_rules();
    let mut compliance_matches = Vec::new();
    for rule in &compliance_rules {
        if let Ok(re) = regex::Regex::new(&rule.pattern) {
            if re.is_match(&request.text) {
                // Determine framework from rule name prefix
                let framework = if rule.name.starts_with("hipaa") {
                    "HIPAA"
                } else if rule.name.starts_with("pci") {
                    "PCI-DSS"
                } else if rule.name.starts_with("gdpr") {
                    "GDPR"
                } else {
                    "Custom"
                };
                compliance_matches.push(ComplianceMatch {
                    rule: rule.name.clone(),
                    framework: framework.to_string(),
                    description: rule.description.clone(),
                    level: format!("{:?}", rule.level),
                });
            }
        }
    }

    // Determine overall level (highest of all)
    let mut max_level = regex_result.level;
    if level_rank(semantic_result.level) > level_rank(max_level) {
        max_level = semantic_result.level;
    }
    for rule in &compliance_rules {
        if compliance_matches.iter().any(|m| m.rule == rule.name)
            && level_rank(rule.level) > level_rank(max_level)
        {
            max_level = rule.level;
        }
    }

    let requires_tee = level_rank(max_level) >= level_rank(SensitivityLevel::Sensitive)
        || state.compliance.tee_mandatory();

    Json(ScanResponse {
        level: format!("{:?}", max_level),
        requires_tee,
        regex_matches: regex_result
            .matches
            .iter()
            .map(|m| ClassifyMatch {
                rule: m.rule_name.clone(),
                level: format!("{:?}", m.level),
                start: m.start,
                end: m.end,
                redacted: m.redacted.clone(),
            })
            .collect(),
        semantic_matches: semantic_result
            .matches
            .iter()
            .map(SemanticMatchResponse::from)
            .collect(),
        compliance_matches,
    })
}

/// GET /api/v1/privacy/compliance/frameworks
async fn list_frameworks(State(state): State<PrivacyState>) -> impl IntoResponse {
    let mut frameworks = Vec::new();

    // Always show all available frameworks with their info
    for (framework, rule_set_fn) in [
        (ComplianceFramework::Hipaa, crate::privacy::compliance::hipaa_rules as fn() -> _),
        (ComplianceFramework::PciDss, crate::privacy::compliance::pci_dss_rules),
        (ComplianceFramework::Gdpr, crate::privacy::compliance::gdpr_rules),
    ] {
        let rule_set = rule_set_fn();
        let enabled = state.compliance.enabled_frameworks().contains(&framework);
        frameworks.push(serde_json::json!({
            "id": format!("{}", framework).to_lowercase().replace('-', "_"),
            "name": rule_set.name,
            "description": rule_set.description,
            "ruleCount": rule_set.rules.len(),
            "teeMandatory": rule_set.tee_mandatory,
            "enabled": enabled,
        }));
    }

    Json(frameworks)
}

/// GET /api/v1/privacy/compliance/rules?framework=hipaa
async fn list_rules(
    State(state): State<PrivacyState>,
    Query(params): Query<RulesQuery>,
) -> impl IntoResponse {
    let rules = if let Some(fw) = &params.framework {
        let framework = match fw.to_lowercase().as_str() {
            "hipaa" => Some(ComplianceFramework::Hipaa),
            "pci-dss" | "pci_dss" | "pcidss" => Some(ComplianceFramework::PciDss),
            "gdpr" => Some(ComplianceFramework::Gdpr),
            "custom" => Some(ComplianceFramework::Custom),
            _ => None,
        };

        match framework {
            Some(fw) => state.compliance.rules_for(fw),
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::to_value(ApiError::bad_request(format!(
                        "Unknown framework: {}. Valid: hipaa, pci-dss, gdpr, custom",
                        fw
                    )))
                    .unwrap()),
                );
            }
        }
    } else {
        state.compliance.all_rules()
    };

    let rule_infos: Vec<serde_json::Value> = rules
        .iter()
        .map(|r| {
            let framework = if r.name.starts_with("hipaa") {
                "HIPAA"
            } else if r.name.starts_with("pci") {
                "PCI-DSS"
            } else if r.name.starts_with("gdpr") {
                "GDPR"
            } else {
                "Custom"
            };
            serde_json::json!({
                "name": r.name,
                "framework": framework,
                "pattern": r.pattern,
                "level": format!("{:?}", r.level),
                "description": r.description,
            })
        })
        .collect();

    (StatusCode::OK, Json(serde_json::to_value(rule_infos).unwrap()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::privacy::compliance;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn make_state() -> PrivacyState {
        let classifier = Arc::new(Classifier::new(vec![], SensitivityLevel::Normal).unwrap());
        let semantic = Arc::new(SemanticAnalyzer::new());
        let compliance = Arc::new(ComplianceEngine::with_frameworks(&[
            ComplianceFramework::Hipaa,
            ComplianceFramework::PciDss,
            ComplianceFramework::Gdpr,
        ]));
        PrivacyState {
            classifier,
            semantic,
            compliance,
        }
    }

    fn make_app() -> Router {
        privacy_router(make_state())
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let body = axum::body::to_bytes(response.into_body(), 1024 * 64)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn test_classify_normal_text() {
        let app = make_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/privacy/classify")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"hello world"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["level"], "Normal");
        assert_eq!(json["requiresTee"], false);
        assert_eq!(json["matches"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_analyze_password_disclosure() {
        let app = make_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/privacy/analyze")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"my password is hunter2"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["requiresTee"], true);
        let matches = json["matches"].as_array().unwrap();
        assert!(!matches.is_empty());
        assert_eq!(matches[0]["category"], "Password");
    }

    #[tokio::test]
    async fn test_analyze_clean_text() {
        let app = make_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/privacy/analyze")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"the weather is nice today"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        let json = body_json(resp).await;
        assert_eq!(json["level"], "Normal");
        assert_eq!(json["matches"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_scan_combined() {
        let app = make_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/privacy/scan")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"text":"my password is hunter2 and my IP is 192.168.1.1"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["requiresTee"], true);
        // Should have semantic match for password
        assert!(!json["semanticMatches"].as_array().unwrap().is_empty());
        // Should have compliance match for IP (GDPR)
        assert!(!json["complianceMatches"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_list_frameworks() {
        let app = make_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/privacy/compliance/frameworks")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert!(arr.iter().any(|f| f["name"] == "HIPAA PHI Detection"));
    }

    #[tokio::test]
    async fn test_list_rules_all() {
        let app = make_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/privacy/compliance/rules")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        let arr = json.as_array().unwrap();
        assert!(arr.len() > 10); // HIPAA + PCI-DSS + GDPR rules
    }

    #[tokio::test]
    async fn test_list_rules_by_framework() {
        let app = make_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/privacy/compliance/rules?framework=hipaa")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        let arr = json.as_array().unwrap();
        assert!(!arr.is_empty());
        for rule in arr {
            assert_eq!(rule["framework"], "HIPAA");
        }
    }

    #[tokio::test]
    async fn test_list_rules_unknown_framework() {
        let app = make_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/privacy/compliance/rules?framework=unknown")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
