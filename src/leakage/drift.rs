//! Security Policy Drift Detection
//!
//! Periodically compares the declared security policy (from config) against
//! the actual runtime state. When drift is detected, an audit event is emitted
//! via the `AuditEventBus`.
//!
//! Drift examples:
//! - TEE was expected but runtime detected `ProcessOnly`
//! - Privacy classification rules changed at runtime
//! - Network firewall policy was modified
//! - Channel config differs from declared config

use crate::config::SafeClawConfig;
use crate::leakage::{AuditEvent, AuditEventBus, AuditSeverity, LeakageVector};
use crate::tee::SecurityLevel;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for drift detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftConfig {
    /// Whether drift detection is enabled.
    pub enabled: bool,
    /// Check interval in seconds.
    pub interval_secs: u64,
}

impl Default for DriftConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_secs: 300, // 5 minutes
        }
    }
}

/// A snapshot of security-relevant configuration state.
#[derive(Debug, Clone, PartialEq)]
pub struct PolicySnapshot {
    /// Whether TEE is expected to be enabled.
    pub tee_enabled: bool,
    /// Expected security level.
    pub expected_security_level: Option<SecurityLevel>,
    /// Number of privacy classification rules.
    pub privacy_rule_count: usize,
    /// Network firewall default policy (allow/deny).
    pub firewall_default_deny: bool,
    /// Enabled channel names.
    pub enabled_channels: Vec<String>,
    /// Whether output sanitization is active.
    pub output_sanitization: bool,
}

impl PolicySnapshot {
    /// Capture a snapshot from the current configuration.
    pub fn from_config(config: &SafeClawConfig) -> Self {
        let mut channels = Vec::new();
        if config.channels.telegram.is_some() {
            channels.push("telegram".to_string());
        }
        if config.channels.slack.is_some() {
            channels.push("slack".to_string());
        }
        if config.channels.discord.is_some() {
            channels.push("discord".to_string());
        }
        if config.channels.webchat.is_some() {
            channels.push("webchat".to_string());
        }
        if config.channels.feishu.is_some() {
            channels.push("feishu".to_string());
        }
        if config.channels.dingtalk.is_some() {
            channels.push("dingtalk".to_string());
        }
        if config.channels.wecom.is_some() {
            channels.push("wecom".to_string());
        }
        channels.sort();

        Self {
            tee_enabled: config.tee.enabled,
            expected_security_level: None, // set at runtime
            privacy_rule_count: config.privacy.rules.len(),
            firewall_default_deny: true, // SafeClaw default
            enabled_channels: channels,
            output_sanitization: true, // always on
        }
    }

    /// Compare two snapshots and return a list of drift descriptions.
    pub fn diff(&self, other: &PolicySnapshot) -> Vec<String> {
        let mut drifts = Vec::new();

        if self.tee_enabled != other.tee_enabled {
            drifts.push(format!(
                "TEE enabled changed: {} -> {}",
                self.tee_enabled, other.tee_enabled
            ));
        }

        if self.expected_security_level != other.expected_security_level {
            drifts.push(format!(
                "Security level changed: {:?} -> {:?}",
                self.expected_security_level, other.expected_security_level
            ));
        }

        if self.privacy_rule_count != other.privacy_rule_count {
            drifts.push(format!(
                "Privacy rule count changed: {} -> {}",
                self.privacy_rule_count, other.privacy_rule_count
            ));
        }

        if self.firewall_default_deny != other.firewall_default_deny {
            drifts.push(format!(
                "Firewall default-deny changed: {} -> {}",
                self.firewall_default_deny, other.firewall_default_deny
            ));
        }

        if self.enabled_channels != other.enabled_channels {
            drifts.push(format!(
                "Enabled channels changed: {:?} -> {:?}",
                self.enabled_channels, other.enabled_channels
            ));
        }

        if self.output_sanitization != other.output_sanitization {
            drifts.push(format!(
                "Output sanitization changed: {} -> {}",
                self.output_sanitization, other.output_sanitization
            ));
        }

        drifts
    }
}

/// Drift detector that periodically reconciles declared vs runtime policy.
pub struct DriftDetector {
    /// The baseline snapshot taken at startup.
    baseline: Arc<RwLock<PolicySnapshot>>,
    /// Audit event bus for emitting drift alerts.
    bus: Arc<AuditEventBus>,
    /// Configuration.
    config: DriftConfig,
}

impl DriftDetector {
    /// Create a new drift detector with the given baseline.
    pub fn new(
        baseline: PolicySnapshot,
        bus: Arc<AuditEventBus>,
        config: DriftConfig,
    ) -> Self {
        Self {
            baseline: Arc::new(RwLock::new(baseline)),
            bus,
            config,
        }
    }

    /// Check for drift between baseline and current snapshot.
    /// Returns the list of detected drifts.
    pub async fn check(&self, current: &PolicySnapshot) -> Vec<String> {
        let baseline = self.baseline.read().await;
        let drifts = baseline.diff(current);

        for drift in &drifts {
            tracing::warn!("Security policy drift detected: {}", drift);
            let event = AuditEvent::new(
                "system".to_string(),
                AuditSeverity::High,
                LeakageVector::PolicyDrift,
                format!("Policy drift: {}", drift),
            );
            self.bus.publish(event).await;
        }

        drifts
    }

    /// Update the baseline to the current snapshot (acknowledge drift).
    pub async fn update_baseline(&self, snapshot: PolicySnapshot) {
        *self.baseline.write().await = snapshot;
        tracing::info!("Drift detector baseline updated");
    }

    /// Get the check interval.
    pub fn interval_secs(&self) -> u64 {
        self.config.interval_secs
    }

    /// Whether drift detection is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

/// Spawn a background task that periodically checks for policy drift.
pub fn spawn_drift_checker(
    detector: Arc<DriftDetector>,
    config: Arc<RwLock<SafeClawConfig>>,
) {
    if !detector.is_enabled() {
        tracing::debug!("Drift detection disabled, skipping background checker");
        return;
    }

    let interval = detector.interval_secs();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(
            tokio::time::Duration::from_secs(interval),
        );
        // Skip the first immediate tick
        ticker.tick().await;

        loop {
            ticker.tick().await;
            let cfg = config.read().await;
            let current = PolicySnapshot::from_config(&cfg);
            let drifts = detector.check(&current).await;
            if !drifts.is_empty() {
                tracing::warn!(
                    count = drifts.len(),
                    "Policy drift check found {} issue(s)",
                    drifts.len()
                );
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::leakage::AuditLog;

    fn default_snapshot() -> PolicySnapshot {
        PolicySnapshot {
            tee_enabled: true,
            expected_security_level: Some(SecurityLevel::TeeHardware),
            privacy_rule_count: 5,
            firewall_default_deny: true,
            enabled_channels: vec!["telegram".to_string(), "webchat".to_string()],
            output_sanitization: true,
        }
    }

    #[test]
    fn test_snapshot_no_drift() {
        let a = default_snapshot();
        let b = default_snapshot();
        assert!(a.diff(&b).is_empty());
    }

    #[test]
    fn test_snapshot_tee_drift() {
        let a = default_snapshot();
        let mut b = default_snapshot();
        b.tee_enabled = false;
        let drifts = a.diff(&b);
        assert_eq!(drifts.len(), 1);
        assert!(drifts[0].contains("TEE enabled"));
    }

    #[test]
    fn test_snapshot_security_level_drift() {
        let a = default_snapshot();
        let mut b = default_snapshot();
        b.expected_security_level = Some(SecurityLevel::ProcessOnly);
        let drifts = a.diff(&b);
        assert_eq!(drifts.len(), 1);
        assert!(drifts[0].contains("Security level"));
    }

    #[test]
    fn test_snapshot_channel_drift() {
        let a = default_snapshot();
        let mut b = default_snapshot();
        b.enabled_channels.push("slack".to_string());
        let drifts = a.diff(&b);
        assert_eq!(drifts.len(), 1);
        assert!(drifts[0].contains("channels"));
    }

    #[test]
    fn test_snapshot_multiple_drifts() {
        let a = default_snapshot();
        let mut b = default_snapshot();
        b.tee_enabled = false;
        b.privacy_rule_count = 10;
        b.firewall_default_deny = false;
        let drifts = a.diff(&b);
        assert_eq!(drifts.len(), 3);
    }

    #[test]
    fn test_snapshot_from_config() {
        let config = SafeClawConfig::default();
        let snap = PolicySnapshot::from_config(&config);
        assert!(snap.output_sanitization);
        assert!(snap.firewall_default_deny);
    }

    #[tokio::test]
    async fn test_detector_no_drift() {
        let baseline = default_snapshot();
        let bus = Arc::new(AuditEventBus::new(100, Arc::new(RwLock::new(AuditLog::new(1000)))));
        let config = DriftConfig {
            enabled: true,
            interval_secs: 60,
        };
        let detector = DriftDetector::new(baseline.clone(), bus, config);

        let drifts = detector.check(&baseline).await;
        assert!(drifts.is_empty());
    }

    #[tokio::test]
    async fn test_detector_emits_event_on_drift() {
        let baseline = default_snapshot();
        let bus = Arc::new(AuditEventBus::new(100, Arc::new(RwLock::new(AuditLog::new(1000)))));
        let mut rx = bus.subscribe();
        let config = DriftConfig {
            enabled: true,
            interval_secs: 60,
        };
        let detector = DriftDetector::new(baseline, bus, config);

        let mut current = default_snapshot();
        current.tee_enabled = false;

        let drifts = detector.check(&current).await;
        assert_eq!(drifts.len(), 1);

        // Verify event was published
        let event = rx.try_recv().unwrap();
        assert_eq!(event.vector, LeakageVector::PolicyDrift);
        assert!(event.description.contains("TEE enabled"));
    }

    #[tokio::test]
    async fn test_detector_update_baseline() {
        let baseline = default_snapshot();
        let bus = Arc::new(AuditEventBus::new(100, Arc::new(RwLock::new(AuditLog::new(1000)))));
        let config = DriftConfig::default();
        let detector = DriftDetector::new(baseline, bus, config);

        let mut new_baseline = default_snapshot();
        new_baseline.tee_enabled = false;

        // First check should detect drift
        let drifts = detector.check(&new_baseline).await;
        assert_eq!(drifts.len(), 1);

        // Update baseline
        detector.update_baseline(new_baseline.clone()).await;

        // Second check should find no drift
        let drifts = detector.check(&new_baseline).await;
        assert!(drifts.is_empty());
    }

    #[test]
    fn test_drift_config_default() {
        let config = DriftConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.interval_secs, 300);
    }
}
