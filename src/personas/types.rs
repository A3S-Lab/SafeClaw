//! Persona wire types
//!
//! Defines the agent persona schema with avatar configuration
//! compatible with react-nice-avatar on the frontend.

use serde::{Deserialize, Serialize};

/// Agent persona identity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPersona {
    pub id: String,
    pub name: String,
    pub description: String,
    pub avatar: AvatarConfig,
    pub system_prompt: String,
    pub default_model: String,
    pub default_permission_mode: String,
    pub builtin: bool,
    pub undeletable: bool,
}

/// Avatar configuration (react-nice-avatar compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvatarConfig {
    #[serde(default = "default_sex")]
    pub sex: String,
    #[serde(default = "default_face_color")]
    pub face_color: String,
    #[serde(default = "default_ear_size")]
    pub ear_size: String,
    #[serde(default = "default_eye_style")]
    pub eye_style: String,
    #[serde(default = "default_nose_style")]
    pub nose_style: String,
    #[serde(default = "default_mouth_style")]
    pub mouth_style: String,
    #[serde(default = "default_shirt_style")]
    pub shirt_style: String,
    #[serde(default = "default_glasses_style")]
    pub glasses_style: String,
    #[serde(default = "default_hair_color")]
    pub hair_color: String,
    #[serde(default = "default_hair_style")]
    pub hair_style: String,
    #[serde(default = "default_hat_style")]
    pub hat_style: String,
    #[serde(default = "default_hat_color")]
    pub hat_color: String,
    #[serde(default = "default_eye_brow_style")]
    pub eye_brow_style: String,
    #[serde(default = "default_shirt_color")]
    pub shirt_color: String,
    #[serde(default = "default_bg_color")]
    pub bg_color: String,
}

fn default_sex() -> String { "man".to_string() }
fn default_face_color() -> String { "#F9C9B6".to_string() }
fn default_ear_size() -> String { "small".to_string() }
fn default_eye_style() -> String { "circle".to_string() }
fn default_nose_style() -> String { "round".to_string() }
fn default_mouth_style() -> String { "smile".to_string() }
fn default_shirt_style() -> String { "polo".to_string() }
fn default_glasses_style() -> String { "none".to_string() }
fn default_hair_color() -> String { "#000".to_string() }
fn default_hair_style() -> String { "normal".to_string() }
fn default_hat_style() -> String { "none".to_string() }
fn default_hat_color() -> String { "#000".to_string() }
fn default_eye_brow_style() -> String { "up".to_string() }
fn default_shirt_color() -> String { "#6BD9E9".to_string() }
fn default_bg_color() -> String { "#E0DDFF".to_string() }

impl Default for AvatarConfig {
    fn default() -> Self {
        Self {
            sex: default_sex(),
            face_color: default_face_color(),
            ear_size: default_ear_size(),
            eye_style: default_eye_style(),
            nose_style: default_nose_style(),
            mouth_style: default_mouth_style(),
            shirt_style: default_shirt_style(),
            glasses_style: default_glasses_style(),
            hair_color: default_hair_color(),
            hair_style: default_hair_style(),
            hat_style: default_hat_style(),
            hat_color: default_hat_color(),
            eye_brow_style: default_eye_brow_style(),
            shirt_color: default_shirt_color(),
            bg_color: default_bg_color(),
        }
    }
}

/// Request body for creating a persona
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePersonaRequest {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub avatar: AvatarConfig,
    #[serde(default = "default_system_prompt")]
    pub system_prompt: String,
    #[serde(default = "default_model")]
    pub default_model: String,
    #[serde(default = "default_permission_mode")]
    pub default_permission_mode: String,
}

fn default_system_prompt() -> String { "You are a helpful assistant.".to_string() }
fn default_model() -> String { "claude-sonnet-4-20250514".to_string() }
fn default_permission_mode() -> String { "default".to_string() }

/// Request body for updating a persona
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePersonaRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub avatar: Option<AvatarConfig>,
    pub system_prompt: Option<String>,
    pub default_model: Option<String>,
    pub default_permission_mode: Option<String>,
}

/// User profile (hardcoded for now)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub id: u64,
    pub nickname: String,
    pub email: String,
    pub avatar: String,
}

/// Convert a name to a kebab-case ID
pub fn name_to_id(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persona_serialization() {
        let persona = AgentPersona {
            id: "financial-analyst".to_string(),
            name: "Financial Analyst".to_string(),
            description: "Senior financial analysis specialist".to_string(),
            avatar: AvatarConfig::default(),
            system_prompt: "You are a senior financial analyst.".to_string(),
            default_model: "claude-sonnet-4-20250514".to_string(),
            default_permission_mode: "default".to_string(),
            builtin: true,
            undeletable: true,
        };

        let json = serde_json::to_string(&persona).unwrap();
        assert!(json.contains("\"id\":\"financial-analyst\""));
        assert!(json.contains("\"systemPrompt\""));
        assert!(json.contains("\"defaultModel\""));
        assert!(json.contains("\"faceColor\":\"#F9C9B6\""));

        let parsed: AgentPersona = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "financial-analyst");
        assert!(parsed.builtin);
    }

    #[test]
    fn test_avatar_defaults() {
        let avatar = AvatarConfig::default();
        assert_eq!(avatar.sex, "man");
        assert_eq!(avatar.face_color, "#F9C9B6");
        assert_eq!(avatar.glasses_style, "none");
    }

    #[test]
    fn test_avatar_partial_deserialization() {
        let json = r##"{"sex":"woman","bgColor":"#FFE0E0"}"##;
        let avatar: AvatarConfig = serde_json::from_str(json).unwrap();
        assert_eq!(avatar.sex, "woman");
        assert_eq!(avatar.bg_color, "#FFE0E0");
        // Defaults for unspecified fields
        assert_eq!(avatar.face_color, "#F9C9B6");
    }

    #[test]
    fn test_create_persona_request() {
        let json = r#"{
            "name": "Tax Specialist",
            "description": "Corporate tax planning"
        }"#;
        let req: CreatePersonaRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Tax Specialist");
        assert_eq!(req.default_model, "claude-sonnet-4-20250514");
        assert_eq!(req.default_permission_mode, "default");
    }

    #[test]
    fn test_update_persona_request_partial() {
        let json = r#"{"name":"Senior Tax Specialist"}"#;
        let req: UpdatePersonaRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name.as_deref(), Some("Senior Tax Specialist"));
        assert!(req.description.is_none());
        assert!(req.avatar.is_none());
    }

    #[test]
    fn test_name_to_id() {
        assert_eq!(name_to_id("Financial Analyst"), "financial-analyst");
        assert_eq!(name_to_id("Tax & Legal Specialist"), "tax-legal-specialist");
        assert_eq!(name_to_id("  Spaces  Everywhere  "), "spaces-everywhere");
        assert_eq!(name_to_id("already-kebab"), "already-kebab");
    }

    #[test]
    fn test_user_profile_serialization() {
        let profile = UserProfile {
            id: 1,
            nickname: "Roy Lin".to_string(),
            email: "[email]".to_string(),
            avatar: "https://github.com/user.png".to_string(),
        };

        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("\"nickname\":\"Roy Lin\""));
    }
}
