//! Privacy classifier for detecting sensitive data

use crate::config::{ClassificationRule, SensitivityLevel};
use crate::error::{Error, Result};
use regex::Regex;

/// Classification result for a piece of data
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    /// Overall sensitivity level
    pub level: SensitivityLevel,
    /// Individual matches found
    pub matches: Vec<Match>,
    /// Whether TEE processing is required
    pub requires_tee: bool,
}

/// A single match found during classification
#[derive(Debug, Clone)]
pub struct Match {
    /// Rule name that matched
    pub rule_name: String,
    /// Sensitivity level of the match
    pub level: SensitivityLevel,
    /// Start position in the text
    pub start: usize,
    /// End position in the text
    pub end: usize,
    /// The matched text (redacted for display)
    pub redacted: String,
}

/// Privacy classifier for detecting sensitive data
pub struct Classifier {
    rules: Vec<CompiledRule>,
    default_level: SensitivityLevel,
}

struct CompiledRule {
    name: String,
    pattern: Regex,
    level: SensitivityLevel,
    #[allow(dead_code)]
    description: String,
}

impl Classifier {
    /// Create a new classifier with the given rules
    pub fn new(rules: Vec<ClassificationRule>, default_level: SensitivityLevel) -> Result<Self> {
        let compiled_rules = rules
            .into_iter()
            .map(|rule| {
                let pattern = Regex::new(&rule.pattern).map_err(|e| {
                    Error::Privacy(format!(
                        "Invalid regex pattern for rule '{}': {}",
                        rule.name, e
                    ))
                })?;

                Ok(CompiledRule {
                    name: rule.name,
                    pattern,
                    level: rule.level,
                    description: rule.description,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            rules: compiled_rules,
            default_level,
        })
    }

    /// Classify a piece of text
    pub fn classify(&self, text: &str) -> ClassificationResult {
        let mut matches = Vec::new();
        let mut highest_level = self.default_level;

        for rule in &self.rules {
            for mat in rule.pattern.find_iter(text) {
                let matched_text = mat.as_str();
                let redacted = redact_text(matched_text, &rule.name);

                matches.push(Match {
                    rule_name: rule.name.clone(),
                    level: rule.level,
                    start: mat.start(),
                    end: mat.end(),
                    redacted,
                });

                if rule.level as u8 > highest_level as u8 {
                    highest_level = rule.level;
                }
            }
        }

        let requires_tee = matches!(
            highest_level,
            SensitivityLevel::Sensitive | SensitivityLevel::HighlySensitive
        );

        ClassificationResult {
            level: highest_level,
            matches,
            requires_tee,
        }
    }

    /// Redact sensitive data in text
    pub fn redact(&self, text: &str) -> String {
        let mut result = text.to_string();
        let classification = self.classify(text);

        // Sort matches by position (reverse order to maintain positions)
        let mut matches = classification.matches;
        matches.sort_by(|a, b| b.start.cmp(&a.start));

        for mat in matches {
            result.replace_range(mat.start..mat.end, &mat.redacted);
        }

        result
    }

    /// Check if text contains any sensitive data
    pub fn contains_sensitive(&self, text: &str) -> bool {
        self.rules.iter().any(|rule| rule.pattern.is_match(text))
    }

    /// Get the highest sensitivity level in text
    pub fn get_sensitivity_level(&self, text: &str) -> SensitivityLevel {
        self.classify(text).level
    }
}

/// Redact text based on the type of sensitive data
fn redact_text(text: &str, rule_name: &str) -> String {
    let len = text.len();
    match rule_name {
        "credit_card" => {
            if len >= 4 {
                format!("****-****-****-{}", &text[len - 4..])
            } else {
                "[REDACTED]".to_string()
            }
        }
        "ssn" => "***-**-****".to_string(),
        "email" => {
            if let Some(at_pos) = text.find('@') {
                let domain = &text[at_pos..];
                format!("****{}", domain)
            } else {
                "[REDACTED]".to_string()
            }
        }
        "phone" => {
            if len >= 4 {
                format!("***-***-{}", &text[len - 4..])
            } else {
                "[REDACTED]".to_string()
            }
        }
        "api_key" => "[API_KEY_REDACTED]".to_string(),
        _ => "[REDACTED]".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::default_classification_rules;

    fn create_test_classifier() -> Classifier {
        Classifier::new(default_classification_rules(), SensitivityLevel::Normal).unwrap()
    }

    #[test]
    fn test_classify_credit_card() {
        let classifier = create_test_classifier();
        let text = "My card number is 4111-1111-1111-1111";

        let result = classifier.classify(text);
        assert_eq!(result.level, SensitivityLevel::HighlySensitive);
        assert!(result.requires_tee);
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.matches[0].rule_name, "credit_card");
    }

    #[test]
    fn test_classify_email() {
        let classifier = create_test_classifier();
        let text = "Contact me at test@example.com";

        let result = classifier.classify(text);
        assert_eq!(result.level, SensitivityLevel::Sensitive);
        assert!(result.requires_tee);
    }

    #[test]
    fn test_classify_normal_text() {
        let classifier = create_test_classifier();
        let text = "Hello, how are you today?";

        let result = classifier.classify(text);
        assert_eq!(result.level, SensitivityLevel::Normal);
        assert!(!result.requires_tee);
        assert!(result.matches.is_empty());
    }

    #[test]
    fn test_redact() {
        let classifier = create_test_classifier();
        let text = "My SSN is 123-45-6789 and email is test@example.com";

        let redacted = classifier.redact(text);
        assert!(redacted.contains("***-**-****"));
        assert!(redacted.contains("****@example.com"));
        assert!(!redacted.contains("123-45-6789"));
    }

    #[test]
    fn test_multiple_matches() {
        let classifier = create_test_classifier();
        let text = "Card: 4111-1111-1111-1111, SSN: 123-45-6789";

        let result = classifier.classify(text);
        assert_eq!(result.matches.len(), 2);
        assert_eq!(result.level, SensitivityLevel::HighlySensitive);
    }
}

// Note: default_classification_rules is defined in config.rs
