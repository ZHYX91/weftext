//! Frozen processing choices. These describe a job, never document typography.
use crate::{KnowledgeError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProcessingLanguage {
    pub language: String,
    pub script: Option<String>,
    pub region: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProcessingDependency {
    pub id: String,
    pub version: String,
}

/// Core reauthorizes and revalidates this settings dependency for every scan
/// and unconfirmed plan. Copying defaults into options does not remove origin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SharedProcessingOrigin {
    pub source: String,
    pub base_id: String,
    pub revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProcessingSettings {
    pub version: String,
    #[serde(default)]
    pub languages: Vec<ProcessingLanguage>,
    pub scheme: String,
    pub input_format: String,
    pub document_type: String,
    #[serde(default)]
    pub dependencies: Vec<ProcessingDependency>,
}
impl Default for ProcessingSettings {
    fn default() -> Self {
        Self {
            version: "1".into(),
            languages: vec![],
            scheme: "general".into(),
            input_format: "typst".into(),
            document_type: "prose".into(),
            dependencies: vec![],
        }
    }
}

/// Empty legacy scopes are not universal. Universal use requires confirmation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProcessingRuleScope {
    #[serde(default)]
    pub universal: bool,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub scripts: Vec<String>,
    #[serde(default)]
    pub regions: Vec<String>,
    #[serde(default)]
    pub input_formats: Vec<String>,
    #[serde(default)]
    pub document_types: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<ProcessingDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProcessingDocument {
    pub schema_version: u32,
    pub defaults: ProcessingSettings,
    #[serde(default)]
    pub rules: Vec<crate::ProofreadRule>,
    #[serde(default)]
    pub scoped_allowlist: Vec<crate::ScopedAllowlistEntry>,
}
impl ProcessingDocument {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err(KnowledgeError::new(
                "processingVersion",
                weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_knowledge_processing_settings_version_unsupported()),
            ));
        }
        let options = crate::ProofreadOptions {
            processing: self.defaults.clone(),
            rules: self.rules.clone(),
            scoped_allowlist: self.scoped_allowlist.clone(),
            ..Default::default()
        };
        crate::proofreading::validate(&options, "")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProcessingMatch {
    Supported,
    Inapplicable,
    Unknown,
    MissingDependency,
}

fn token(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}
impl ProcessingLanguage {
    pub fn validate(&self) -> Result<()> {
        if !(2..=8).contains(&self.language.len())
            || !self.language.bytes().all(|b| b.is_ascii_alphabetic())
            || self
                .script
                .as_ref()
                .is_some_and(|s| s.len() != 4 || !s.bytes().all(|b| b.is_ascii_alphabetic()))
            || self.region.as_ref().is_some_and(|r| {
                !(r.len() == 2 && r.bytes().all(|b| b.is_ascii_alphabetic())
                    || r.len() == 3 && r.bytes().all(|b| b.is_ascii_digit()))
            })
        {
            return Err(KnowledgeError::new(
                "processingLanguage",
                weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_knowledge_processing_settings_locale_code_invalid()),
            ));
        }
        Ok(())
    }
    /// A native language tag is evidence only for components actually present.
    pub fn from_native_tag(tag: &str) -> Option<Self> {
        let parts: Vec<_> = tag.split('-').collect();
        if parts.len() > 3 || parts.len() == 3 && parts[1].len() != 4 {
            return None;
        }
        let mut result = Self {
            language: parts.first()?.to_ascii_lowercase(),
            ..Self::default()
        };
        for part in &parts[1..] {
            if part.len() == 4 && result.script.is_none() {
                result.script = Some((*part).into());
            } else if result.region.is_none() {
                result.region = Some(part.to_ascii_uppercase());
            } else {
                return None;
            }
        }
        result.validate().ok()?;
        Some(result)
    }
}
impl ProcessingSettings {
    pub fn validate(&self) -> Result<()> {
        if !token(&self.version, 100)
            || !token(&self.scheme, 100)
            || !token(&self.input_format, 50)
            || !token(&self.document_type, 100)
            || self.languages.len() > 32
            || self.dependencies.len() > 2000
        {
            return Err(KnowledgeError::new(
                "processingSettings",
                weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_knowledge_processing_settings_explicit_bounded_settings_required()),
            ));
        }
        for language in &self.languages {
            language.validate()?;
        }
        for dep in &self.dependencies {
            if !token(&dep.id, 100) || !token(&dep.version, 100) {
                return Err(KnowledgeError::new(
                    "processingDependency",
                    weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_knowledge_processing_settings_dependency_identity_version_required()),
                ));
            }
        }
        Ok(())
    }
}
impl ProcessingRuleScope {
    pub fn validate(&self) -> Result<()> {
        if self.universal
            && (!self.languages.is_empty() || !self.scripts.is_empty() || !self.regions.is_empty())
        {
            return Err(KnowledgeError::new(
                "processingScope",
                weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_knowledge_processing_settings_universal_scope_conflict()),
            ));
        }
        for values in [
            &self.languages,
            &self.scripts,
            &self.regions,
            &self.input_formats,
            &self.document_types,
        ] {
            if values.len() > 100 || values.iter().any(|v| !token(v, 100)) {
                return Err(KnowledgeError::new(
                    "processingScope",
                    weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_knowledge_processing_settings_rule_scope_invalid()),
                ));
            }
        }
        if self.dependencies.len() > 2000
            || self
                .dependencies
                .iter()
                .any(|d| !token(&d.id, 100) || !token(&d.version, 100))
        {
            return Err(KnowledgeError::new(
                "processingDependency",
                weftext_messages::ErrorDescription::new(weftext_messages::UiMessage::error_knowledge_processing_settings_rule_dependency_identity_version_required()),
            ));
        }
        Ok(())
    }
    pub fn matches(
        &self,
        language: Option<&ProcessingLanguage>,
        settings: &ProcessingSettings,
    ) -> ProcessingMatch {
        let accepts = |values: &[String], value: &str| {
            values.is_empty() || values.iter().any(|v| v.eq_ignore_ascii_case(value))
        };
        if !accepts(&self.input_formats, &settings.input_format)
            || !accepts(&self.document_types, &settings.document_type)
        {
            return ProcessingMatch::Inapplicable;
        }
        if !self.universal {
            if self.languages.is_empty() && self.scripts.is_empty() && self.regions.is_empty() {
                return ProcessingMatch::Unknown;
            }
            let Some(language) = language else {
                return ProcessingMatch::Unknown;
            };
            if !accepts(&self.languages, &language.language) {
                return ProcessingMatch::Inapplicable;
            }
            for (values, actual) in [
                (&self.scripts, &language.script),
                (&self.regions, &language.region),
            ] {
                if !values.is_empty() {
                    let Some(actual) = actual else {
                        return ProcessingMatch::Unknown;
                    };
                    if !accepts(values, actual) {
                        return ProcessingMatch::Inapplicable;
                    }
                }
            }
        }
        if self
            .dependencies
            .iter()
            .any(|need| !settings.dependencies.contains(need))
        {
            return ProcessingMatch::MissingDependency;
        }
        ProcessingMatch::Supported
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn region_and_script_never_infer_each_other_and_versions_are_exact() {
        let settings = ProcessingSettings::default();
        let scope = ProcessingRuleScope {
            languages: vec!["zh".into()],
            scripts: vec!["Hans".into()],
            regions: vec!["CN".into()],
            ..Default::default()
        };
        assert_eq!(
            scope.matches(
                ProcessingLanguage::from_native_tag("zh-CN").as_ref(),
                &settings
            ),
            ProcessingMatch::Unknown
        );
        assert_eq!(
            scope.matches(
                ProcessingLanguage::from_native_tag("zh-Hant-TW").as_ref(),
                &settings
            ),
            ProcessingMatch::Inapplicable
        );
        assert_eq!(
            scope.matches(
                ProcessingLanguage::from_native_tag("zh-Hans-CN").as_ref(),
                &settings
            ),
            ProcessingMatch::Supported
        );
        assert_eq!(
            scope.matches(
                ProcessingLanguage::from_native_tag("zh-Hans").as_ref(),
                &settings
            ),
            ProcessingMatch::Unknown
        );
        let general = ProcessingRuleScope {
            universal: true,
            dependencies: vec![ProcessingDependency {
                id: "dictionary".into(),
                version: "2".into(),
            }],
            ..Default::default()
        };
        assert_eq!(
            general.matches(None, &settings),
            ProcessingMatch::MissingDependency
        );
        let mut installed = settings;
        installed.dependencies.push(ProcessingDependency {
            id: "dictionary".into(),
            version: "1".into(),
        });
        assert_eq!(
            general.matches(None, &installed),
            ProcessingMatch::MissingDependency
        );
        installed.dependencies[0].version = "2".into();
        assert_eq!(
            general.matches(None, &installed),
            ProcessingMatch::Supported
        );
    }
    #[test]
    fn legacy_unknown_is_distinct_from_explicit_universal() {
        assert_eq!(
            ProcessingRuleScope::default().matches(None, &ProcessingSettings::default()),
            ProcessingMatch::Unknown
        );
        assert_eq!(
            ProcessingRuleScope {
                universal: true,
                ..Default::default()
            }
            .matches(None, &ProcessingSettings::default()),
            ProcessingMatch::Supported
        );
        assert!(
            ProcessingLanguage::from_native_tag("en-GB")
                .unwrap()
                .script
                .is_none()
        );
        assert!(ProcessingLanguage::from_native_tag("zh-CN-Hans-extra").is_none());
    }
}
