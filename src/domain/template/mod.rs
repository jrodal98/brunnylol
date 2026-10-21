// Template module for RFC 6570-style variable templating
//
// This module provides parsing, validation, and expansion of URL templates
// with variable substitution and pipeline operations.

mod ast;
pub mod form_builder;
mod parser;
mod pipeline;
mod resolver;

pub use ast::{PipelineOp, Template, TemplateMetadata, TemplatePart, VariableExpr};
pub use parser::TemplateParser;
pub use pipeline::{PipelineOperation, PipelineRegistry};
pub use resolver::TemplateResolver;
