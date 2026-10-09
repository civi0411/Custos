use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::new_id;

/// Recipe represents a reproducible definition of an experiment or task execution.
/// It defines *how* an execution should happen, separating the commands from the logs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub command: String,
    pub environment_spec: EnvironmentSpec,
    pub inputs: Vec<RecipeInput>,
    pub outputs: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl Recipe {
    pub fn new(
        name: impl Into<String>,
        command: impl Into<String>,
        environment_spec: EnvironmentSpec,
    ) -> Self {
        Self {
            id: new_id("recipe"),
            name: name.into(),
            description: None,
            command: command.into(),
            environment_spec,
            inputs: Vec::new(),
            outputs: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_input(mut self, input: RecipeInput) -> Self {
        self.inputs.push(input);
        self
    }

    pub fn with_output(mut self, output: impl Into<String>) -> Self {
        self.outputs.push(output.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSpec {
    pub python_version: Option<String>,
    pub requirements: Vec<String>,
    pub container_image: Option<String>,
    pub hardware: Option<String>,
}

impl EnvironmentSpec {
    pub fn new() -> Self {
        Self {
            python_version: None,
            requirements: Vec::new(),
            container_image: None,
            hardware: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeInput {
    pub name: String,
    pub source_uri: String,
    pub required: bool,
}

impl RecipeInput {
    pub fn new(name: impl Into<String>, source_uri: impl Into<String>, required: bool) -> Self {
        Self {
            name: name.into(),
            source_uri: source_uri.into(),
            required,
        }
    }
}
