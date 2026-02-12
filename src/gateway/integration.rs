//! A3S Gateway integration — generates routing config and provides backend API
//!
//! When SafeClaw runs behind a3s-gateway, this module generates the TOML
//! configuration that a3s-gateway needs to route traffic to SafeClaw.

use crate::config::SafeClawConfig;
use serde::Serialize;

/// Generated a3s-gateway routing configuration for SafeClaw
#[derive(Debug, Clone, Serialize)]
pub struct GatewayRoutingConfig {
    /// Router configurations
    pub routers: Vec<RouterEntry>,
    /// Service configuration
    pub service: ServiceEntry,
    /// Middleware configurations
    pub middlewares: Vec<MiddlewareEntry>,
}

/// A router entry for a3s-gateway
#[derive(Debug, Clone, Serialize)]
pub struct RouterEntry {
    /// Router name
    pub name: String,
    /// Routing rule
    pub rule: String,
    /// Target service
    pub service: String,
    /// Entrypoints
    pub entrypoints: Vec<String>,
    /// Middleware names
    pub middlewares: Vec<String>,
}

/// A service entry for a3s-gateway
#[derive(Debug, Clone, Serialize)]
pub struct ServiceEntry {
    /// Service name
    pub name: String,
    /// Backend URL
    pub url: String,
    /// Health check path
    pub health_check_path: String,
    /// Sticky session cookie (if conversation affinity enabled)
    pub sticky_cookie: Option<String>,
}

/// A middleware entry for a3s-gateway
#[derive(Debug, Clone, Serialize)]
pub struct MiddlewareEntry {
    /// Middleware name
    pub name: String,
    /// Middleware type
    pub middleware_type: String,
    /// Configuration (serialized as TOML key-value pairs)
    pub config: std::collections::HashMap<String, String>,
}

/// Generate a3s-gateway TOML configuration for SafeClaw
///
/// This produces a TOML snippet that can be placed in a3s-gateway's
/// configuration directory for automatic discovery via file provider.
pub fn generate_gateway_config(config: &SafeClawConfig) -> String {
    let gw = &config.a3s_gateway;
    let backend_url = format!("http://{}:{}", config.gateway.host, config.gateway.port);

    let mut toml = String::new();

    // Comment header
    toml.push_str("# Auto-generated a3s-gateway config for SafeClaw\n");
    toml.push_str("# Place in a3s-gateway's provider.file.directory\n\n");

    // API router
    toml.push_str(&format!("[routers.{}-api]\n", gw.service_name));
    toml.push_str(&format!("rule = \"{}\"\n", gw.api_rule));
    toml.push_str(&format!("service = \"{}\"\n", gw.service_name));
    toml.push_str(&format!(
        "entrypoints = [{}]\n",
        gw.entrypoints
            .iter()
            .map(|e| format!("\"{}\"", e))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    if !gw.middlewares.is_empty() {
        toml.push_str(&format!(
            "middlewares = [{}]\n",
            gw.middlewares
                .iter()
                .map(|m| format!("\"{}\"", m))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    toml.push('\n');

    // WebSocket router
    toml.push_str(&format!("[routers.{}-ws]\n", gw.service_name));
    toml.push_str(&format!("rule = \"{}\"\n", gw.ws_rule));
    toml.push_str(&format!("service = \"{}\"\n", gw.service_name));
    toml.push_str(&format!(
        "entrypoints = [{}]\n",
        gw.entrypoints
            .iter()
            .map(|e| format!("\"{}\"", e))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    toml.push('\n');

    // Webhook router
    toml.push_str(&format!("[routers.{}-webhook]\n", gw.service_name));
    toml.push_str(&format!("rule = \"{}\"\n", gw.webhook_rule));
    toml.push_str(&format!("service = \"{}\"\n", gw.service_name));
    toml.push_str(&format!(
        "entrypoints = [{}]\n",
        gw.entrypoints
            .iter()
            .map(|e| format!("\"{}\"", e))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    toml.push('\n');

    // Service with load balancer
    toml.push_str(&format!("[services.{}.load_balancer]\n", gw.service_name));
    toml.push_str("strategy = \"round-robin\"\n");
    toml.push_str("health_check = { path = \"/health\", interval = \"10s\" }\n");

    if gw.conversation_affinity {
        toml.push_str(&format!(
            "sticky = {{ cookie = \"{}\" }}\n",
            gw.affinity_cookie
        ));
    }

    toml.push_str(&format!(
        "[[services.{}.load_balancer.servers]]\n",
        gw.service_name
    ));
    toml.push_str(&format!("url = \"{}\"\n", backend_url));
    toml.push('\n');

    // Token metering middleware
    if gw.token_metering && gw.max_tokens_per_minute > 0 {
        toml.push_str(&format!("[middlewares.{}-token-meter]\n", gw.service_name));
        toml.push_str("type = \"token-meter\"\n");
        toml.push_str(&format!(
            "max_tokens_per_minute = {}\n",
            gw.max_tokens_per_minute
        ));
        toml.push_str("header = \"X-Token-Count\"\n");
    }

    toml
}

/// Generate a structured routing config (for programmatic use)
pub fn generate_routing_config(config: &SafeClawConfig) -> GatewayRoutingConfig {
    let gw = &config.a3s_gateway;
    let backend_url = format!("http://{}:{}", config.gateway.host, config.gateway.port);

    let routers = vec![
        RouterEntry {
            name: format!("{}-api", gw.service_name),
            rule: gw.api_rule.clone(),
            service: gw.service_name.clone(),
            entrypoints: gw.entrypoints.clone(),
            middlewares: gw.middlewares.clone(),
        },
        RouterEntry {
            name: format!("{}-ws", gw.service_name),
            rule: gw.ws_rule.clone(),
            service: gw.service_name.clone(),
            entrypoints: gw.entrypoints.clone(),
            middlewares: vec![],
        },
        RouterEntry {
            name: format!("{}-webhook", gw.service_name),
            rule: gw.webhook_rule.clone(),
            service: gw.service_name.clone(),
            entrypoints: gw.entrypoints.clone(),
            middlewares: vec![],
        },
    ];

    let service = ServiceEntry {
        name: gw.service_name.clone(),
        url: backend_url,
        health_check_path: "/health".to_string(),
        sticky_cookie: if gw.conversation_affinity {
            Some(gw.affinity_cookie.clone())
        } else {
            None
        },
    };

    let mut middlewares = Vec::new();
    if gw.token_metering && gw.max_tokens_per_minute > 0 {
        let mut meter_config = std::collections::HashMap::new();
        meter_config.insert("type".to_string(), "token-meter".to_string());
        meter_config.insert(
            "max_tokens_per_minute".to_string(),
            gw.max_tokens_per_minute.to_string(),
        );
        meter_config.insert("header".to_string(), "X-Token-Count".to_string());

        middlewares.push(MiddlewareEntry {
            name: format!("{}-token-meter", gw.service_name),
            middleware_type: "token-meter".to_string(),
            config: meter_config,
        });
    }

    GatewayRoutingConfig {
        routers,
        service,
        middlewares,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::A3sGatewayConfig;

    fn test_config() -> SafeClawConfig {
        let mut config = SafeClawConfig::default();
        config.a3s_gateway.enabled = true;
        config.gateway.host = "127.0.0.1".to_string();
        config.gateway.port = 18790;
        config
    }

    #[test]
    fn test_generate_gateway_config_toml() {
        let config = test_config();
        let toml = generate_gateway_config(&config);

        assert!(toml.contains("[routers.safeclaw-api]"));
        assert!(toml.contains("[routers.safeclaw-ws]"));
        assert!(toml.contains("[routers.safeclaw-webhook]"));
        assert!(toml.contains("[services.safeclaw.load_balancer]"));
        assert!(toml.contains("http://127.0.0.1:18790"));
        assert!(toml.contains("safeclaw_session"));
        assert!(toml.contains("token-meter"));
    }

    #[test]
    fn test_generate_gateway_config_no_affinity() {
        let mut config = test_config();
        config.a3s_gateway.conversation_affinity = false;
        let toml = generate_gateway_config(&config);

        assert!(!toml.contains("sticky"));
    }

    #[test]
    fn test_generate_gateway_config_no_token_metering() {
        let mut config = test_config();
        config.a3s_gateway.token_metering = false;
        let toml = generate_gateway_config(&config);

        assert!(!toml.contains("token-meter"));
    }

    #[test]
    fn test_generate_routing_config() {
        let config = test_config();
        let routing = generate_routing_config(&config);

        assert_eq!(routing.routers.len(), 3);
        assert_eq!(routing.service.name, "safeclaw");
        assert_eq!(routing.service.url, "http://127.0.0.1:18790");
        assert!(routing.service.sticky_cookie.is_some());
        assert_eq!(routing.middlewares.len(), 1);
    }

    #[test]
    fn test_routing_config_router_names() {
        let config = test_config();
        let routing = generate_routing_config(&config);

        let names: Vec<&str> = routing.routers.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"safeclaw-api"));
        assert!(names.contains(&"safeclaw-ws"));
        assert!(names.contains(&"safeclaw-webhook"));
    }

    #[test]
    fn test_routing_config_custom_service_name() {
        let mut config = test_config();
        config.a3s_gateway.service_name = "my-safeclaw".to_string();
        let routing = generate_routing_config(&config);

        assert_eq!(routing.service.name, "my-safeclaw");
        assert_eq!(routing.routers[0].name, "my-safeclaw-api");
    }

    #[test]
    fn test_default_a3s_gateway_config() {
        let config = A3sGatewayConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.service_name, "safeclaw");
        assert!(config.conversation_affinity);
        assert!(config.token_metering);
        assert_eq!(config.max_tokens_per_minute, 10000);
    }
}
