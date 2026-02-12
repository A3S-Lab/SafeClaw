//! Persona store with builtin defaults and custom persona persistence
//!
//! Builtin personas are loaded from a hardcoded list. Custom personas are
//! persisted as JSON files under `~/.safeclaw/personas/`.

use crate::events::types::ApiError;
use crate::personas::types::*;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Persona store with builtin + custom personas
pub struct PersonaStore {
    custom_dir: PathBuf,
    personas: Arc<RwLock<Vec<AgentPersona>>>,
}

impl PersonaStore {
    /// Create a new persona store, loading builtins + custom from disk
    pub async fn new(custom_dir: PathBuf) -> std::io::Result<Self> {
        tokio::fs::create_dir_all(&custom_dir).await?;

        let mut personas = builtin_personas();
        let custom = Self::load_custom_from_disk(&custom_dir);
        personas.extend(custom);

        Ok(Self {
            custom_dir,
            personas: Arc::new(RwLock::new(personas)),
        })
    }

    /// Default directory (~/.safeclaw/personas/)
    pub fn default_dir() -> PathBuf {
        dirs_next::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".safeclaw")
            .join("personas")
    }

    /// List all personas
    pub async fn list(&self) -> Vec<AgentPersona> {
        self.personas.read().await.clone()
    }

    /// Get a persona by ID
    pub async fn get(&self, id: &str) -> Option<AgentPersona> {
        self.personas.read().await.iter().find(|p| p.id == id).cloned()
    }

    /// Create a custom persona
    pub async fn create(&self, req: CreatePersonaRequest) -> Result<AgentPersona, ApiError> {
        let id = name_to_id(&req.name);

        // Check for duplicate ID
        {
            let personas = self.personas.read().await;
            if personas.iter().any(|p| p.id == id) {
                return Err(ApiError::bad_request(format!(
                    "Persona with id '{}' already exists",
                    id
                )));
            }
        }

        let persona = AgentPersona {
            id,
            name: req.name,
            description: req.description,
            avatar: req.avatar,
            system_prompt: req.system_prompt,
            default_model: req.default_model,
            default_permission_mode: req.default_permission_mode,
            builtin: false,
            undeletable: false,
        };

        {
            let mut personas = self.personas.write().await;
            personas.push(persona.clone());
        }

        self.persist_custom(&persona);
        Ok(persona)
    }

    /// Update a custom persona (builtin personas return error)
    pub async fn update(
        &self,
        id: &str,
        req: UpdatePersonaRequest,
    ) -> Result<AgentPersona, ApiError> {
        let mut personas = self.personas.write().await;

        let persona = personas
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| ApiError::not_found(format!("Persona '{}' not found", id)))?;

        if persona.builtin {
            return Err(ApiError {
                error: crate::events::types::ApiErrorDetail {
                    code: "FORBIDDEN".to_string(),
                    message: "Cannot modify builtin personas".to_string(),
                },
            });
        }

        if let Some(name) = req.name {
            persona.name = name;
        }
        if let Some(description) = req.description {
            persona.description = description;
        }
        if let Some(avatar) = req.avatar {
            persona.avatar = avatar;
        }
        if let Some(system_prompt) = req.system_prompt {
            persona.system_prompt = system_prompt;
        }
        if let Some(default_model) = req.default_model {
            persona.default_model = default_model;
        }
        if let Some(default_permission_mode) = req.default_permission_mode {
            persona.default_permission_mode = default_permission_mode;
        }

        let updated = persona.clone();
        self.persist_custom(&updated);
        Ok(updated)
    }

    /// Load custom personas from disk
    fn load_custom_from_disk(dir: &std::path::Path) -> Vec<AgentPersona> {
        let mut personas = Vec::new();
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return personas,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            match std::fs::read_to_string(&path) {
                Ok(data) => match serde_json::from_str::<AgentPersona>(&data) {
                    Ok(p) => personas.push(p),
                    Err(e) => tracing::warn!("Failed to parse persona {}: {}", path.display(), e),
                },
                Err(e) => tracing::warn!("Failed to read persona {}: {}", path.display(), e),
            }
        }

        personas
    }

    /// Persist a custom persona to disk (fire-and-forget)
    fn persist_custom(&self, persona: &AgentPersona) {
        let dir = self.custom_dir.clone();
        let persona = persona.clone();
        tokio::spawn(async move {
            let path = dir.join(format!("{}.json", persona.id));
            match serde_json::to_string_pretty(&persona) {
                Ok(json) => {
                    if let Err(e) = tokio::fs::write(&path, json).await {
                        tracing::warn!("Failed to persist persona {}: {}", persona.id, e);
                    }
                }
                Err(e) => tracing::warn!("Failed to serialize persona {}: {}", persona.id, e),
            }
        });
    }
}

/// Builtin personas shipped with SafeClaw
fn builtin_personas() -> Vec<AgentPersona> {
    vec![
        AgentPersona {
            id: "financial-analyst".to_string(),
            name: "Financial Analyst".to_string(),
            description: "Senior financial analysis and reporting specialist".to_string(),
            avatar: AvatarConfig {
                sex: "woman".to_string(),
                hair_style: "womanLong".to_string(),
                shirt_color: "#6BD9E9".to_string(),
                bg_color: "#E0DDFF".to_string(),
                ..Default::default()
            },
            system_prompt: "You are a senior financial analyst with expertise in financial modeling, reporting, and market analysis.".to_string(),
            default_model: "claude-sonnet-4-20250514".to_string(),
            default_permission_mode: "default".to_string(),
            builtin: true,
            undeletable: true,
        },
        AgentPersona {
            id: "fullstack-engineer".to_string(),
            name: "Fullstack Engineer".to_string(),
            description: "Senior fullstack development specialist".to_string(),
            avatar: AvatarConfig {
                sex: "man".to_string(),
                hair_style: "thick".to_string(),
                shirt_color: "#FC909F".to_string(),
                bg_color: "#E0F4FF".to_string(),
                ..Default::default()
            },
            system_prompt: "You are a senior fullstack engineer proficient in modern web technologies, system design, and DevOps.".to_string(),
            default_model: "claude-sonnet-4-20250514".to_string(),
            default_permission_mode: "default".to_string(),
            builtin: true,
            undeletable: true,
        },
        AgentPersona {
            id: "risk-analyst".to_string(),
            name: "Risk Analyst".to_string(),
            description: "Enterprise risk assessment and compliance specialist".to_string(),
            avatar: AvatarConfig {
                sex: "man".to_string(),
                hair_style: "normal".to_string(),
                glasses_style: "round".to_string(),
                shirt_color: "#77311D".to_string(),
                bg_color: "#FFEDEF".to_string(),
                ..Default::default()
            },
            system_prompt: "You are a risk analyst specializing in enterprise risk management, compliance, and regulatory frameworks.".to_string(),
            default_model: "claude-sonnet-4-20250514".to_string(),
            default_permission_mode: "default".to_string(),
            builtin: true,
            undeletable: true,
        },
        AgentPersona {
            id: "devops-engineer".to_string(),
            name: "DevOps Engineer".to_string(),
            description: "Infrastructure and CI/CD automation specialist".to_string(),
            avatar: AvatarConfig {
                sex: "man".to_string(),
                hair_style: "mohawk".to_string(),
                shirt_color: "#9287FF".to_string(),
                bg_color: "#E8FFE0".to_string(),
                ..Default::default()
            },
            system_prompt: "You are a DevOps engineer with expertise in cloud infrastructure, CI/CD pipelines, and container orchestration.".to_string(),
            default_model: "claude-sonnet-4-20250514".to_string(),
            default_permission_mode: "default".to_string(),
            builtin: true,
            undeletable: false,
        },
        AgentPersona {
            id: "data-scientist".to_string(),
            name: "Data Scientist".to_string(),
            description: "Machine learning and data analytics specialist".to_string(),
            avatar: AvatarConfig {
                sex: "woman".to_string(),
                hair_style: "womanShort".to_string(),
                glasses_style: "square".to_string(),
                shirt_color: "#F4D150".to_string(),
                bg_color: "#FFF5E0".to_string(),
                ..Default::default()
            },
            system_prompt: "You are a data scientist with expertise in machine learning, statistical analysis, and data visualization.".to_string(),
            default_model: "claude-sonnet-4-20250514".to_string(),
            default_permission_mode: "default".to_string(),
            builtin: true,
            undeletable: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn make_store() -> (PersonaStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = PersonaStore::new(dir.path().to_path_buf()).await.unwrap();
        (store, dir)
    }

    #[tokio::test]
    async fn test_list_includes_builtins() {
        let (store, _dir) = make_store().await;
        let personas = store.list().await;
        assert!(personas.len() >= 5);
        assert!(personas.iter().any(|p| p.id == "financial-analyst"));
        assert!(personas.iter().all(|p| p.builtin));
    }

    #[tokio::test]
    async fn test_get_builtin() {
        let (store, _dir) = make_store().await;
        let persona = store.get("fullstack-engineer").await;
        assert!(persona.is_some());
        let p = persona.unwrap();
        assert_eq!(p.name, "Fullstack Engineer");
        assert!(p.builtin);
    }

    #[tokio::test]
    async fn test_get_not_found() {
        let (store, _dir) = make_store().await;
        assert!(store.get("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_create_custom_persona() {
        let (store, _dir) = make_store().await;

        let req = CreatePersonaRequest {
            name: "Tax Specialist".to_string(),
            description: "Corporate tax planning".to_string(),
            avatar: AvatarConfig::default(),
            system_prompt: "You are a tax specialist.".to_string(),
            default_model: "claude-sonnet-4-20250514".to_string(),
            default_permission_mode: "default".to_string(),
        };

        let persona = store.create(req).await.unwrap();
        assert_eq!(persona.id, "tax-specialist");
        assert!(!persona.builtin);
        assert!(!persona.undeletable);

        // Should be findable
        let fetched = store.get("tax-specialist").await.unwrap();
        assert_eq!(fetched.name, "Tax Specialist");
    }

    #[tokio::test]
    async fn test_create_duplicate_id_fails() {
        let (store, _dir) = make_store().await;

        let req = CreatePersonaRequest {
            name: "Financial Analyst".to_string(),
            description: "Duplicate".to_string(),
            avatar: AvatarConfig::default(),
            system_prompt: "test".to_string(),
            default_model: "claude-sonnet-4-20250514".to_string(),
            default_permission_mode: "default".to_string(),
        };

        let result = store.create(req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_custom_persona() {
        let (store, _dir) = make_store().await;

        // Create first
        let req = CreatePersonaRequest {
            name: "My Agent".to_string(),
            description: "Original".to_string(),
            avatar: AvatarConfig::default(),
            system_prompt: "Original prompt".to_string(),
            default_model: "claude-sonnet-4-20250514".to_string(),
            default_permission_mode: "default".to_string(),
        };
        store.create(req).await.unwrap();

        // Update
        let update = UpdatePersonaRequest {
            name: Some("My Updated Agent".to_string()),
            description: None,
            avatar: None,
            system_prompt: Some("Updated prompt".to_string()),
            default_model: None,
            default_permission_mode: None,
        };

        let updated = store.update("my-agent", update).await.unwrap();
        assert_eq!(updated.name, "My Updated Agent");
        assert_eq!(updated.system_prompt, "Updated prompt");
        assert_eq!(updated.description, "Original"); // unchanged
    }

    #[tokio::test]
    async fn test_update_builtin_fails() {
        let (store, _dir) = make_store().await;

        let update = UpdatePersonaRequest {
            name: Some("Hacked".to_string()),
            description: None,
            avatar: None,
            system_prompt: None,
            default_model: None,
            default_permission_mode: None,
        };

        let result = store.update("financial-analyst", update).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_not_found() {
        let (store, _dir) = make_store().await;

        let update = UpdatePersonaRequest {
            name: Some("X".to_string()),
            description: None,
            avatar: None,
            system_prompt: None,
            default_model: None,
            default_permission_mode: None,
        };

        let result = store.update("nonexistent", update).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_persistence_round_trip() {
        let dir = TempDir::new().unwrap();

        // Create store, add custom persona
        {
            let store = PersonaStore::new(dir.path().to_path_buf()).await.unwrap();
            let req = CreatePersonaRequest {
                name: "Persisted Agent".to_string(),
                description: "Should survive reload".to_string(),
                avatar: AvatarConfig::default(),
                system_prompt: "test".to_string(),
                default_model: "claude-sonnet-4-20250514".to_string(),
                default_permission_mode: "default".to_string(),
            };
            store.create(req).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        // Reload
        let store = PersonaStore::new(dir.path().to_path_buf()).await.unwrap();
        let persona = store.get("persisted-agent").await;
        assert!(persona.is_some());
        assert_eq!(persona.unwrap().description, "Should survive reload");
    }
}
