//! Compliance rule engine for enterprise privacy regulations.
//!
//! Provides pre-built rule sets for common compliance frameworks:
//! - HIPAA (Health Insurance Portability and Accountability Act)
//! - PCI-DSS (Payment Card Industry Data Security Standard)
//! - GDPR (General Data Protection Regulation)
//!
//! Also supports user-defined custom rules that can be loaded from config.

use crate::config::SensitivityLevel;
use a3s_privacy::ClassificationRule;
use serde::{Deserialize, Serialize};

/// Compliance framework identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComplianceFramework {
    /// HIPAA — Protected Health Information
    Hipaa,
    /// PCI-DSS — Payment card data
    PciDss,
    /// GDPR — EU personal data
    Gdpr,
    /// Custom user-defined framework
    Custom,
}

impl std::fmt::Display for ComplianceFramework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hipaa => write!(f, "HIPAA"),
            Self::PciDss => write!(f, "PCI-DSS"),
            Self::Gdpr => write!(f, "GDPR"),
            Self::Custom => write!(f, "Custom"),
        }
    }
}

/// A compliance rule set containing classification rules and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRuleSet {
    /// Framework this rule set belongs to
    pub framework: ComplianceFramework,
    /// Human-readable name
    pub name: String,
    /// Description of the rule set
    pub description: String,
    /// Classification rules
    pub rules: Vec<ClassificationRule>,
    /// Whether TEE is mandatory for any match
    pub tee_mandatory: bool,
    /// Minimum sensitivity level for matches
    pub min_level: SensitivityLevel,
}

/// Compliance engine that manages multiple rule sets
pub struct ComplianceEngine {
    rule_sets: Vec<ComplianceRuleSet>,
}

impl ComplianceEngine {
    /// Create an empty compliance engine
    pub fn new() -> Self {
        Self {
            rule_sets: Vec::new(),
        }
    }

    /// Create a compliance engine with the specified frameworks enabled
    pub fn with_frameworks(frameworks: &[ComplianceFramework]) -> Self {
        let mut engine = Self::new();
        for framework in frameworks {
            match framework {
                ComplianceFramework::Hipaa => engine.add_rule_set(hipaa_rules()),
                ComplianceFramework::PciDss => engine.add_rule_set(pci_dss_rules()),
                ComplianceFramework::Gdpr => engine.add_rule_set(gdpr_rules()),
                ComplianceFramework::Custom => {} // Custom rules added separately
            }
        }
        engine
    }

    /// Add a rule set
    pub fn add_rule_set(&mut self, rule_set: ComplianceRuleSet) {
        self.rule_sets.push(rule_set);
    }

    /// Add custom user-defined rules
    pub fn add_custom_rules(&mut self, rules: Vec<ClassificationRule>) {
        if rules.is_empty() {
            return;
        }
        self.rule_sets.push(ComplianceRuleSet {
            framework: ComplianceFramework::Custom,
            name: "Custom Rules".to_string(),
            description: "User-defined classification rules".to_string(),
            rules,
            tee_mandatory: false,
            min_level: SensitivityLevel::Sensitive,
        });
    }

    /// Get all classification rules from all enabled frameworks
    pub fn all_rules(&self) -> Vec<ClassificationRule> {
        self.rule_sets
            .iter()
            .flat_map(|rs| rs.rules.clone())
            .collect()
    }

    /// Get rules for a specific framework
    pub fn rules_for(&self, framework: ComplianceFramework) -> Vec<ClassificationRule> {
        self.rule_sets
            .iter()
            .filter(|rs| rs.framework == framework)
            .flat_map(|rs| rs.rules.clone())
            .collect()
    }

    /// Check if TEE is mandatory for any enabled framework
    pub fn tee_mandatory(&self) -> bool {
        self.rule_sets.iter().any(|rs| rs.tee_mandatory)
    }

    /// Get enabled frameworks
    pub fn enabled_frameworks(&self) -> Vec<ComplianceFramework> {
        self.rule_sets
            .iter()
            .map(|rs| rs.framework)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }

    /// Get rule set count
    pub fn rule_set_count(&self) -> usize {
        self.rule_sets.len()
    }

    /// Get total rule count
    pub fn rule_count(&self) -> usize {
        self.rule_sets.iter().map(|rs| rs.rules.len()).sum()
    }
}

impl Default for ComplianceEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ---- HIPAA Rules ----

/// HIPAA Protected Health Information (PHI) detection rules
pub fn hipaa_rules() -> ComplianceRuleSet {
    ComplianceRuleSet {
        framework: ComplianceFramework::Hipaa,
        name: "HIPAA PHI Detection".to_string(),
        description: "Detects Protected Health Information per HIPAA Safe Harbor de-identification"
            .to_string(),
        tee_mandatory: true,
        min_level: SensitivityLevel::HighlySensitive,
        rules: vec![
            // Medical Record Number (MRN) — typically 6-10 digits with optional prefix
            ClassificationRule {
                name: "hipaa_mrn".to_string(),
                pattern: r"(?i)\b(?:MRN|medical\s*record)\s*[#:]?\s*\d{6,10}\b".to_string(),
                level: SensitivityLevel::HighlySensitive,
                description: "Medical Record Number (HIPAA PHI)".to_string(),
            },
            // Health plan beneficiary number
            ClassificationRule {
                name: "hipaa_health_plan_id".to_string(),
                pattern: r"(?i)\b(?:health\s*plan|beneficiary|member)\s*(?:id|number|#)\s*[:]?\s*[A-Z0-9]{6,15}\b".to_string(),
                level: SensitivityLevel::HighlySensitive,
                description: "Health plan beneficiary number (HIPAA PHI)".to_string(),
            },
            // ICD-10 diagnosis codes (e.g., E11.9, J45.20)
            ClassificationRule {
                name: "hipaa_icd10".to_string(),
                pattern: r"\b[A-TV-Z]\d{2}(?:\.\d{1,4})?\b".to_string(),
                level: SensitivityLevel::Sensitive,
                description: "ICD-10 diagnosis code (HIPAA PHI context)".to_string(),
            },
            // DEA number (prescriber identifier)
            ClassificationRule {
                name: "hipaa_dea".to_string(),
                pattern: r"\b[ABFMPRabfmpr][A-Za-z]\d{7}\b".to_string(),
                level: SensitivityLevel::HighlySensitive,
                description: "DEA number (HIPAA PHI)".to_string(),
            },
            // NPI (National Provider Identifier) — 10 digits
            ClassificationRule {
                name: "hipaa_npi".to_string(),
                pattern: r"(?i)\bNPI\s*[#:]?\s*\d{10}\b".to_string(),
                level: SensitivityLevel::Sensitive,
                description: "National Provider Identifier (HIPAA)".to_string(),
            },
            // Drug/prescription names with dosage
            ClassificationRule {
                name: "hipaa_prescription".to_string(),
                pattern: r"(?i)\b(?:prescribed|taking|dosage|rx)\s*[:]?\s*\w+\s+\d+\s*(?:mg|ml|mcg|units?)\b".to_string(),
                level: SensitivityLevel::Sensitive,
                description: "Prescription information (HIPAA PHI)".to_string(),
            },
        ],
    }
}

// ---- PCI-DSS Rules ----

/// PCI-DSS payment card data detection rules
pub fn pci_dss_rules() -> ComplianceRuleSet {
    ComplianceRuleSet {
        framework: ComplianceFramework::PciDss,
        name: "PCI-DSS Cardholder Data".to_string(),
        description: "Detects payment card data per PCI-DSS requirements".to_string(),
        tee_mandatory: true,
        min_level: SensitivityLevel::HighlySensitive,
        rules: vec![
            // Primary Account Number (PAN) — Visa
            ClassificationRule {
                name: "pci_visa".to_string(),
                pattern: r"\b4\d{3}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b".to_string(),
                level: SensitivityLevel::HighlySensitive,
                description: "Visa card number (PCI-DSS)".to_string(),
            },
            // Mastercard
            ClassificationRule {
                name: "pci_mastercard".to_string(),
                pattern: r"\b5[1-5]\d{2}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b".to_string(),
                level: SensitivityLevel::HighlySensitive,
                description: "Mastercard number (PCI-DSS)".to_string(),
            },
            // American Express
            ClassificationRule {
                name: "pci_amex".to_string(),
                pattern: r"\b3[47]\d{2}[-\s]?\d{6}[-\s]?\d{5}\b".to_string(),
                level: SensitivityLevel::HighlySensitive,
                description: "American Express card number (PCI-DSS)".to_string(),
            },
            // CVV/CVC (3-4 digits, context-aware)
            ClassificationRule {
                name: "pci_cvv".to_string(),
                pattern: r"(?i)\b(?:cvv|cvc|cvv2|cvc2|security\s*code)\s*[:#]?\s*\d{3,4}\b"
                    .to_string(),
                level: SensitivityLevel::Critical,
                description: "Card verification value (PCI-DSS)".to_string(),
            },
            // Expiration date (MM/YY or MM/YYYY)
            ClassificationRule {
                name: "pci_expiry".to_string(),
                pattern: r"(?i)\b(?:exp(?:iry|iration)?|valid\s*(?:thru|through))\s*[:#]?\s*(?:0[1-9]|1[0-2])\s*[/\-]\s*(?:\d{2}|\d{4})\b".to_string(),
                level: SensitivityLevel::HighlySensitive,
                description: "Card expiration date (PCI-DSS)".to_string(),
            },
            // Magnetic stripe data pattern (Track 1/2)
            ClassificationRule {
                name: "pci_track_data".to_string(),
                pattern: r"%B\d{13,19}\^[A-Z\s/]+\^\d{4}".to_string(),
                level: SensitivityLevel::Critical,
                description: "Magnetic stripe track data (PCI-DSS)".to_string(),
            },
        ],
    }
}

// ---- GDPR Rules ----

/// GDPR personal data detection rules
pub fn gdpr_rules() -> ComplianceRuleSet {
    ComplianceRuleSet {
        framework: ComplianceFramework::Gdpr,
        name: "GDPR Personal Data".to_string(),
        description: "Detects personal data categories under GDPR Article 4 and Article 9"
            .to_string(),
        tee_mandatory: false,
        min_level: SensitivityLevel::Sensitive,
        rules: vec![
            // EU National ID patterns (generic)
            ClassificationRule {
                name: "gdpr_national_id".to_string(),
                pattern: r"(?i)\b(?:national\s*id|identity\s*(?:card|number)|personalausweis|carte\s*d'identit[eé]|DNI)\s*[:#]?\s*[A-Z0-9]{6,15}\b".to_string(),
                level: SensitivityLevel::HighlySensitive,
                description: "National identity number (GDPR)".to_string(),
            },
            // EU passport number
            ClassificationRule {
                name: "gdpr_passport".to_string(),
                pattern: r"(?i)\bpassport\s*(?:number|no|#)\s*[:#]?\s*[A-Z0-9]{6,12}\b".to_string(),
                level: SensitivityLevel::HighlySensitive,
                description: "Passport number (GDPR)".to_string(),
            },
            // IBAN (International Bank Account Number)
            ClassificationRule {
                name: "gdpr_iban".to_string(),
                pattern: r"\b[A-Z]{2}\d{2}\s?[A-Z0-9]{4}\s?(?:\d{4}\s?){2,7}\d{1,4}\b".to_string(),
                level: SensitivityLevel::HighlySensitive,
                description: "IBAN bank account number (GDPR)".to_string(),
            },
            // EU VAT number
            ClassificationRule {
                name: "gdpr_vat".to_string(),
                pattern: r"\b(?:ATU|BE0|BG|CY|CZ|DE|DK|EE|EL|ES[A-Z]|FI|FR[A-Z0-9]{2}|HR|HU|IE|IT|LT|LU|LV|MT|NL|PL|PT|RO|SE|SI|SK)\d{7,12}\b".to_string(),
                level: SensitivityLevel::Sensitive,
                description: "EU VAT identification number (GDPR)".to_string(),
            },
            // IP address (personal data under GDPR)
            ClassificationRule {
                name: "gdpr_ip_address".to_string(),
                pattern: r"\b(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\b".to_string(),
                level: SensitivityLevel::Sensitive,
                description: "IP address (GDPR personal data)".to_string(),
            },
            // Ethnic/racial origin (GDPR Article 9 special category)
            ClassificationRule {
                name: "gdpr_special_ethnic".to_string(),
                pattern: r"(?i)\b(?:ethnicity|ethnic\s*origin|race|racial)\s*[:#]?\s*\w+".to_string(),
                level: SensitivityLevel::HighlySensitive,
                description: "Ethnic/racial data (GDPR Article 9)".to_string(),
            },
            // Religious/philosophical beliefs (GDPR Article 9)
            ClassificationRule {
                name: "gdpr_special_religion".to_string(),
                pattern: r"(?i)\b(?:religion|religious\s*belief|faith|philosophical\s*belief)\s*[:#]?\s*\w+".to_string(),
                level: SensitivityLevel::HighlySensitive,
                description: "Religious belief data (GDPR Article 9)".to_string(),
            },
            // Biometric data reference
            ClassificationRule {
                name: "gdpr_biometric".to_string(),
                pattern: r"(?i)\b(?:fingerprint|biometric|facial\s*recognition|retina\s*scan|voice\s*print)\s*(?:data|id|hash|template)\b".to_string(),
                level: SensitivityLevel::HighlySensitive,
                description: "Biometric data reference (GDPR Article 9)".to_string(),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- ComplianceEngine tests ----

    #[test]
    fn test_empty_engine() {
        let engine = ComplianceEngine::new();
        assert_eq!(engine.rule_count(), 0);
        assert_eq!(engine.rule_set_count(), 0);
        assert!(!engine.tee_mandatory());
    }

    #[test]
    fn test_with_hipaa() {
        let engine = ComplianceEngine::with_frameworks(&[ComplianceFramework::Hipaa]);
        assert!(engine.rule_count() > 0);
        assert!(engine.tee_mandatory());
        assert!(engine.enabled_frameworks().contains(&ComplianceFramework::Hipaa));
    }

    #[test]
    fn test_with_pci_dss() {
        let engine = ComplianceEngine::with_frameworks(&[ComplianceFramework::PciDss]);
        assert!(engine.rule_count() > 0);
        assert!(engine.tee_mandatory());
    }

    #[test]
    fn test_with_gdpr() {
        let engine = ComplianceEngine::with_frameworks(&[ComplianceFramework::Gdpr]);
        assert!(engine.rule_count() > 0);
        assert!(!engine.tee_mandatory()); // GDPR doesn't mandate TEE
    }

    #[test]
    fn test_multiple_frameworks() {
        let engine = ComplianceEngine::with_frameworks(&[
            ComplianceFramework::Hipaa,
            ComplianceFramework::PciDss,
            ComplianceFramework::Gdpr,
        ]);
        assert_eq!(engine.rule_set_count(), 3);
        assert!(engine.tee_mandatory());
        let total = hipaa_rules().rules.len() + pci_dss_rules().rules.len() + gdpr_rules().rules.len();
        assert_eq!(engine.rule_count(), total);
    }

    #[test]
    fn test_custom_rules() {
        let mut engine = ComplianceEngine::new();
        engine.add_custom_rules(vec![ClassificationRule {
            name: "custom_employee_id".to_string(),
            pattern: r"\bEMP-\d{6}\b".to_string(),
            level: SensitivityLevel::Sensitive,
            description: "Employee ID".to_string(),
        }]);
        assert_eq!(engine.rule_count(), 1);
        assert!(engine.enabled_frameworks().contains(&ComplianceFramework::Custom));
    }

    #[test]
    fn test_empty_custom_rules_ignored() {
        let mut engine = ComplianceEngine::new();
        engine.add_custom_rules(vec![]);
        assert_eq!(engine.rule_set_count(), 0);
    }

    #[test]
    fn test_rules_for_framework() {
        let engine = ComplianceEngine::with_frameworks(&[
            ComplianceFramework::Hipaa,
            ComplianceFramework::PciDss,
        ]);
        let hipaa = engine.rules_for(ComplianceFramework::Hipaa);
        let pci = engine.rules_for(ComplianceFramework::PciDss);
        assert_eq!(hipaa.len(), hipaa_rules().rules.len());
        assert_eq!(pci.len(), pci_dss_rules().rules.len());
    }

    // ---- Rule pattern validation ----

    #[test]
    fn test_hipaa_rules_compile() {
        let rules = hipaa_rules();
        for rule in &rules.rules {
            assert!(
                regex::Regex::new(&rule.pattern).is_ok(),
                "HIPAA rule '{}' has invalid pattern: {}",
                rule.name,
                rule.pattern
            );
        }
    }

    #[test]
    fn test_pci_dss_rules_compile() {
        let rules = pci_dss_rules();
        for rule in &rules.rules {
            assert!(
                regex::Regex::new(&rule.pattern).is_ok(),
                "PCI-DSS rule '{}' has invalid pattern: {}",
                rule.name,
                rule.pattern
            );
        }
    }

    #[test]
    fn test_gdpr_rules_compile() {
        let rules = gdpr_rules();
        for rule in &rules.rules {
            assert!(
                regex::Regex::new(&rule.pattern).is_ok(),
                "GDPR rule '{}' has invalid pattern: {}",
                rule.name,
                rule.pattern
            );
        }
    }

    // ---- Pattern matching tests ----

    #[test]
    fn test_hipaa_mrn_match() {
        let re = regex::Regex::new(&hipaa_rules().rules[0].pattern).unwrap();
        assert!(re.is_match("MRN: 12345678"));
        assert!(re.is_match("medical record #1234567890"));
    }

    #[test]
    fn test_pci_visa_match() {
        let re = regex::Regex::new(&pci_dss_rules().rules[0].pattern).unwrap();
        assert!(re.is_match("4111-1111-1111-1111"));
        assert!(re.is_match("4111 1111 1111 1111"));
        assert!(re.is_match("4111111111111111"));
    }

    #[test]
    fn test_pci_cvv_match() {
        let re = regex::Regex::new(&pci_dss_rules().rules[3].pattern).unwrap();
        assert!(re.is_match("CVV: 123"));
        assert!(re.is_match("security code: 4567"));
    }

    #[test]
    fn test_gdpr_iban_match() {
        let re = regex::Regex::new(&gdpr_rules().rules[2].pattern).unwrap();
        assert!(re.is_match("DE89 3704 0044 0532 0130 00"));
        assert!(re.is_match("GB29NWBK60161331926819"));
    }

    #[test]
    fn test_gdpr_ip_match() {
        let re = regex::Regex::new(&gdpr_rules().rules[4].pattern).unwrap();
        assert!(re.is_match("192.168.1.1"));
        assert!(re.is_match("10.0.0.1"));
        assert!(!re.is_match("999.999.999.999"));
    }

    #[test]
    fn test_framework_display() {
        assert_eq!(ComplianceFramework::Hipaa.to_string(), "HIPAA");
        assert_eq!(ComplianceFramework::PciDss.to_string(), "PCI-DSS");
        assert_eq!(ComplianceFramework::Gdpr.to_string(), "GDPR");
        assert_eq!(ComplianceFramework::Custom.to_string(), "Custom");
    }
}
