// Abstract Syntax Tree types for URL templates

use serde::{Deserialize, Serialize};

/// Represents a parsed template as a list of parts
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub parts: Vec<TemplatePart>,
}

impl Template {
    pub fn new(parts: Vec<TemplatePart>) -> Self {
        Self { parts }
    }

    /// Get all variables in this template
    pub fn variables(&self) -> Vec<&VariableExpr> {
        self.parts
            .iter()
            .filter_map(|part| match part {
                TemplatePart::Variable(var) => Some(var),
                _ => None,
            })
            .collect()
    }
}

/// A template consists of literal strings and variable expressions
#[derive(Debug, Clone, PartialEq)]
pub enum TemplatePart {
    Literal(String),
    Variable(VariableExpr),
}

/// A variable expression with optional modifiers and pipeline operations
#[derive(Debug, Clone, PartialEq)]
pub struct VariableExpr {
    pub name: String,
    pub is_optional: bool,
    pub default: Option<String>,
    pub pipelines: Vec<PipelineOp>,
}

impl VariableExpr {
    pub fn new(name: String) -> Self {
        Self {
            name,
            is_optional: false,
            default: None,
            pipelines: Vec::new(),
        }
    }

    pub fn with_optional(mut self, optional: bool) -> Self {
        self.is_optional = optional;
        self
    }

    pub fn with_default(mut self, default: String) -> Self {
        self.default = Some(default);
        self
    }

    pub fn with_pipelines(mut self, pipelines: Vec<PipelineOp>) -> Self {
        self.pipelines = pipelines;
        self
    }
}

/// Pipeline operation types
#[derive(Debug, Clone, PartialEq)]
pub enum PipelineOp {
    Encode,
    NoEncode,
    Trim,
    Options {
        values: Vec<String>,
        strict: bool,
    },
    Map {
        mappings: Vec<(String, String)>,
    },
}

/// Metadata about all variables in a template (for database storage)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMetadata {
    pub variables: Vec<VariableMetadata>,
}

impl TemplateMetadata {
    pub fn from_template(template: &Template) -> Self {
        let variables = template
            .variables()
            .into_iter()
            .map(|var| VariableMetadata {
                name: var.name.clone(),
                is_optional: var.is_optional,
                default_value: var.default.clone(),
                options: None,
                strict: false,
            })
            .collect();

        Self { variables }
    }
}

/// Per-variable configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableMetadata {
    pub name: String,
    pub is_optional: bool,
    pub default_value: Option<String>,
    pub options: Option<Vec<String>>,
    pub strict: bool,
}
