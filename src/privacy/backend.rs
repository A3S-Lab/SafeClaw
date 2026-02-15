//! Pluggable classifier backend architecture
//!
//! Defines the `ClassifierBackend` trait for pluggable PII classification,
//! and `CompositeClassifier` that chains multiple backends together.
//!
//! **Threat model**: Defends against A1 (malicious user) at AS-2 (PII classification).
//! See `docs/threat-model.md` §4 AS-2, §5.
//!
//! ## Architecture
//!
//! ```text
//! Input text → [RegexBackend] → [SemanticBackend] → (optional) [LlmBackend]
//!                   ↓                  ↓                         ↓
//!              merge results → deduplicate by span → ClassificationResult
//! ```
//!
//! ## Accuracy labeling
//!
//! Every `PiiMatch` includes a `backend` field identifying which classifier
//! caught it. This enables audit trails and accuracy analysis.

use async_trait::async_trait;
use crate::config::SensitivityLevel;

/// A single PII match found by a classifier backend.
#[derive(Debug, Clone)]
pub struct PiiMatch {
    /// Rule or pattern name that matched
    pub rule_name: String,
    /// Sensitivity level of the match
    pub level: SensitivityLevel,
    /// Start byte offset in the input text
    pub start: usize,
    /// End byte offset in the input text
    pub end: usize,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Which backend produced this match
    pub backend: String,
}

/// Pluggable classification backend interface.
///
/// Implementations can use regex, semantic analysis, LLM calls, or any
/// other technique to detect PII in text.
#[async_trait]
pub trait ClassifierBackend: Send + Sync {
    /// Classify text and return all PII matches found.
    async fn classify(&self, text: &str) -> Vec<PiiMatch>;

    /// Minimum confidence this backend can guarantee.
    ///
    /// Used by `CompositeClassifier` to resolve overlapping matches:
    /// when two backends find PII at the same span, the one with
    /// higher confidence floor wins.
    fn confidence_floor(&self) -> f64;

    /// Human-readable name for this backend (used in audit logs).
    fn name(&self) -> &str;
}

/// Regex-based classifier backend.
///
/// Wraps the existing `a3s_privacy::RegexClassifier`. Fast, high-precision,
/// but low recall for semantic PII (addresses in prose, passwords in context).
pub struct RegexBackend {
    inner: a3s_privacy::RegexClassifier,
}

impl RegexBackend {
    /// Create from existing classification rules
    pub fn new(
        rules: Vec<crate::config::ClassificationRule>,
        default_level: SensitivityLevel,
    ) -> Result<Self, String> {
        let inner = a3s_privacy::RegexClassifier::new(&rules, default_level)
            .map_err(|e| format!("Failed to compile classification rules: {}", e))?;
        Ok(Self { inner })
    }
}

#[async_trait]
impl ClassifierBackend for RegexBackend {
    async fn classify(&self, text: &str) -> Vec<PiiMatch> {
        let result = self.inner.classify(text);
        result
            .matches
            .into_iter()
            .map(|m| PiiMatch {
                rule_name: m.rule_name,
                level: m.level,
                start: m.start,
                end: m.end,
                confidence: 0.95, // Regex matches are high-precision
                backend: "regex".to_string(),
            })
            .collect()
    }

    fn confidence_floor(&self) -> f64 {
        0.90
    }

    fn name(&self) -> &str {
        "regex"
    }
}

/// Semantic analysis classifier backend.
///
/// Wraps the existing `SemanticAnalyzer` for context-aware PII detection
/// (e.g., "my password is hunter2", "I live at 123 Main St").
pub struct SemanticBackend {
    inner: crate::privacy::SemanticAnalyzer,
}

impl SemanticBackend {
    /// Create a new semantic backend
    pub fn new(analyzer: crate::privacy::SemanticAnalyzer) -> Self {
        Self { inner: analyzer }
    }
}

#[async_trait]
impl ClassifierBackend for SemanticBackend {
    async fn classify(&self, text: &str) -> Vec<PiiMatch> {
        let result = self.inner.analyze(text);
        result
            .matches
            .into_iter()
            .map(|m| PiiMatch {
                rule_name: format!("semantic:{:?}", m.category),
                level: m.level,
                start: m.start,
                end: m.end,
                confidence: m.confidence,
                backend: "semantic".to_string(),
            })
            .collect()
    }

    fn confidence_floor(&self) -> f64 {
        0.60
    }

    fn name(&self) -> &str {
        "semantic"
    }
}

/// Composite classifier that chains multiple backends and merges results.
///
/// Default chain: Regex → Semantic → (optional) LLM.
/// Results are merged with deduplication by span overlap:
/// when two matches overlap, the one with higher confidence wins.
pub struct CompositeClassifier {
    backends: Vec<Box<dyn ClassifierBackend>>,
}

impl CompositeClassifier {
    /// Create a new composite classifier with the given backends.
    ///
    /// Backends are evaluated in order. All results are merged.
    pub fn new(backends: Vec<Box<dyn ClassifierBackend>>) -> Self {
        Self { backends }
    }

    /// Classify text through all backends and merge results.
    pub async fn classify(&self, text: &str) -> CompositeResult {
        let mut all_matches = Vec::new();

        for backend in &self.backends {
            let matches = backend.classify(text).await;
            all_matches.extend(matches);
        }

        // Deduplicate overlapping matches — highest confidence wins
        let deduped = deduplicate_matches(all_matches);

        // Determine overall sensitivity level
        let overall_level = deduped
            .iter()
            .map(|m| m.level)
            .max()
            .unwrap_or(SensitivityLevel::Normal);

        let requires_tee = overall_level >= SensitivityLevel::Sensitive;

        CompositeResult {
            level: overall_level,
            matches: deduped,
            requires_tee,
        }
    }

    /// Check if text contains any sensitive data
    pub async fn contains_sensitive(&self, text: &str) -> bool {
        let result = self.classify(text).await;
        !result.matches.is_empty()
    }
}

/// Result from the composite classifier, including backend attribution.
#[derive(Debug, Clone)]
pub struct CompositeResult {
    /// Overall sensitivity level (max across all matches)
    pub level: SensitivityLevel,
    /// All deduplicated matches with backend attribution
    pub matches: Vec<PiiMatch>,
    /// Whether TEE processing is required
    pub requires_tee: bool,
}

/// Deduplicate overlapping matches by keeping the highest-confidence one.
///
/// Two matches overlap if their byte ranges intersect. When they do,
/// the match with higher confidence is kept.
fn deduplicate_matches(mut matches: Vec<PiiMatch>) -> Vec<PiiMatch> {
    if matches.len() <= 1 {
        return matches;
    }

    // Sort by start position, then by confidence descending
    matches.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then(b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal))
    });

    let mut result: Vec<PiiMatch> = Vec::new();

    for m in matches {
        // Check if this match overlaps with the last kept match
        if let Some(last) = result.last() {
            if m.start < last.end {
                // Overlapping — keep the one with higher confidence
                if m.confidence > last.confidence {
                    result.pop();
                    result.push(m);
                }
                // Otherwise skip this match (lower confidence)
                continue;
            }
        }
        result.push(m);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::default_classification_rules;

    #[tokio::test]
    async fn test_regex_backend() {
        let backend =
            RegexBackend::new(default_classification_rules(), SensitivityLevel::Normal).unwrap();
        let matches = backend.classify("My card is 4111-1111-1111-1111").await;
        assert!(!matches.is_empty());
        assert_eq!(matches[0].backend, "regex");
        assert_eq!(matches[0].rule_name, "credit_card");
    }

    #[tokio::test]
    async fn test_regex_backend_no_match() {
        let backend =
            RegexBackend::new(default_classification_rules(), SensitivityLevel::Normal).unwrap();
        let matches = backend.classify("Hello, how are you?").await;
        assert!(matches.is_empty());
    }

    #[tokio::test]
    async fn test_semantic_backend() {
        let analyzer = crate::privacy::SemanticAnalyzer::new();
        let backend = SemanticBackend::new(analyzer);
        let matches = backend.classify("my password is hunter2").await;
        assert!(!matches.is_empty());
        assert_eq!(matches[0].backend, "semantic");
    }

    #[tokio::test]
    async fn test_composite_classifier_merges() {
        let regex =
            RegexBackend::new(default_classification_rules(), SensitivityLevel::Normal).unwrap();
        let semantic = SemanticBackend::new(crate::privacy::SemanticAnalyzer::new());

        let composite = CompositeClassifier::new(vec![Box::new(regex), Box::new(semantic)]);

        let result = composite.classify("My SSN is 123-45-6789").await;
        assert!(!result.matches.is_empty());
        assert!(result.requires_tee);
    }

    #[tokio::test]
    async fn test_composite_classifier_normal_text() {
        let regex =
            RegexBackend::new(default_classification_rules(), SensitivityLevel::Normal).unwrap();
        let composite = CompositeClassifier::new(vec![Box::new(regex)]);

        let result = composite.classify("Hello world").await;
        assert!(result.matches.is_empty());
        assert_eq!(result.level, SensitivityLevel::Normal);
        assert!(!result.requires_tee);
    }

    #[test]
    fn test_deduplicate_no_overlap() {
        let matches = vec![
            PiiMatch {
                rule_name: "a".into(),
                level: SensitivityLevel::Sensitive,
                start: 0,
                end: 5,
                confidence: 0.9,
                backend: "regex".into(),
            },
            PiiMatch {
                rule_name: "b".into(),
                level: SensitivityLevel::Sensitive,
                start: 10,
                end: 15,
                confidence: 0.8,
                backend: "semantic".into(),
            },
        ];
        let result = deduplicate_matches(matches);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_deduplicate_overlap_keeps_higher_confidence() {
        let matches = vec![
            PiiMatch {
                rule_name: "regex_ssn".into(),
                level: SensitivityLevel::HighlySensitive,
                start: 10,
                end: 21,
                confidence: 0.95,
                backend: "regex".into(),
            },
            PiiMatch {
                rule_name: "semantic_ssn".into(),
                level: SensitivityLevel::Sensitive,
                start: 10,
                end: 21,
                confidence: 0.70,
                backend: "semantic".into(),
            },
        ];
        let result = deduplicate_matches(matches);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].backend, "regex");
    }

    #[test]
    fn test_deduplicate_empty() {
        let result = deduplicate_matches(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_deduplicate_single() {
        let matches = vec![PiiMatch {
            rule_name: "a".into(),
            level: SensitivityLevel::Sensitive,
            start: 0,
            end: 5,
            confidence: 0.9,
            backend: "regex".into(),
        }];
        let result = deduplicate_matches(matches);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_confidence_floor() {
        let regex =
            RegexBackend::new(default_classification_rules(), SensitivityLevel::Normal).unwrap();
        assert!(regex.confidence_floor() > 0.8);

        let semantic = SemanticBackend::new(crate::privacy::SemanticAnalyzer::new());
        assert!(semantic.confidence_floor() < regex.confidence_floor());
    }

    #[test]
    fn test_backend_names() {
        let regex =
            RegexBackend::new(default_classification_rules(), SensitivityLevel::Normal).unwrap();
        assert_eq!(regex.name(), "regex");

        let semantic = SemanticBackend::new(crate::privacy::SemanticAnalyzer::new());
        assert_eq!(semantic.name(), "semantic");
    }
}
