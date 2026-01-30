// Template module for RFC 6570-style variable templating
//
// This module provides parsing, validation, and expansion of URL templates
// with variable substitution and pipeline operations.

mod ast;
mod parser;
mod pipeline;
mod resolver;
mod variable;

pub use ast::{PipelineOp, Template, TemplatePart, VariableExpr};
pub use parser::TemplateParser;
pub use pipeline::{PipelineOperation, PipelineRegistry};
pub use resolver::TemplateResolver;
pub use variable::TemplateMetadata;

#[cfg(test)]
mod tests;
