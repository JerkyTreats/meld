use super::config::ProviderType;

/// Validation result for provider configuration.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub provider_name: String,
    pub checks: Vec<(String, bool)>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn new(provider_name: String) -> Self {
        Self {
            provider_name,
            checks: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_check(&mut self, description: &str, passed: bool) {
        self.checks.push((description.to_string(), passed));
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn total_checks(&self) -> usize {
        self.checks.len()
    }

    pub fn passed_checks(&self) -> usize {
        self.checks.iter().filter(|(_, passed)| *passed).count()
    }
}

pub fn provider_type_slug(provider_type: ProviderType) -> &'static str {
    match provider_type {
        ProviderType::OpenAI => "openai",
        ProviderType::Anthropic => "anthropic",
        ProviderType::Ollama => "ollama",
        ProviderType::LocalCustom => "local",
    }
}
