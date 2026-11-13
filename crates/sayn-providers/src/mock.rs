use crate::error::ProviderError;
use crate::provider::{GenerationProvider, GenerationRequest};
use sayn_ir::{GENERATION_SCHEMA_VERSION, GeneratedOperation, GenerationResponse};

/// Deterministic test double. It is never selected by the CLI: its output is
/// canned, not model-generated.
pub struct MockProvider {
    pub provider_name: String,
}

impl Default for MockProvider {
    fn default() -> Self {
        Self {
            provider_name: "mock".to_string(),
        }
    }
}

impl MockProvider {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            provider_name: name.into(),
        }
    }
}

impl GenerationProvider for MockProvider {
    fn name(&self) -> &str {
        &self.provider_name
    }

    fn model(&self) -> &str {
        "mock-model"
    }

    fn generate(&self, request: &GenerationRequest) -> Result<GenerationResponse, ProviderError> {
        let page_tsx = format!(
            "export default function Page() {{\n  return <main><h1>Mock application</h1><p>Target: {}</p></main>;\n}}\n",
            request.target
        );

        Ok(GenerationResponse {
            schema_version: GENERATION_SCHEMA_VERSION,
            operations: vec![
                GeneratedOperation::Create {
                    path: "app/page.tsx".into(),
                    content: page_tsx,
                },
                GeneratedOperation::Create {
                    path: "package.json".into(),
                    content: r#"{"name": "generated-mock", "version": "0.1.0"}"#.into(),
                },
            ],
            acceptance_checks: Vec::new(),
            summary: "Mock generation".into(),
        })
    }
}
