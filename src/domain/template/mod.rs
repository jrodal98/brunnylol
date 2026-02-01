// Template module for RFC 6570-style variable templating
//
// This module provides parsing, validation, and expansion of URL templates
// with variable substitution and pipeline operations.

mod ast;
mod parser;
mod pipeline;
mod resolver;
pub mod form_builder;

pub use ast::{PipelineOp, Template, TemplatePart, VariableExpr, TemplateMetadata};
pub use parser::TemplateParser;
pub use pipeline::{PipelineOperation, PipelineRegistry};
pub use resolver::TemplateResolver;
