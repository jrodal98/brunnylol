use std::collections::HashMap;

pub mod template;

// Domain model for bookmark commands
// Uses enum instead of trait for better performance (no heap allocation, no vtable dispatch)

#[derive(Debug, Clone)]
pub enum Command {
    Simple {
        url: String,
        description: String,
    },
    Variable {
        base_url: String,
        template: template::Template,  // Parsed AST
        description: String,
        metadata: Option<template::TemplateMetadata>,
    },
    Nested {
        children: HashMap<String, Command>,
        description: String,
    },
}

impl Command {
    pub fn description(&self) -> &str {
        match self {
            Command::Simple { description, .. } => description,
            Command::Variable { description, .. } => description,
            Command::Nested { description, .. } => description,
        }
    }

    pub fn get_redirect_url(&self, query: &str) -> String {
        match self {
            Command::Simple { url, .. } => url.clone(),
            Command::Variable { base_url, template, .. } => {
                // For variable templates, map query to variables
                if query.trim().is_empty() {
                    return base_url.clone();
                }

                let mut vars = HashMap::new();

                // Check if template has a "query" variable
                let has_query_var = template.variables().iter().any(|v| v.name == "query");

                if has_query_var {
                    vars.insert("query".to_string(), query.to_string());
                } else if let Some(first_var) = template.variables().first() {
                    // Map to first variable
                    vars.insert(first_var.name.clone(), query.to_string());
                }

                // Use resolver to expand template
                let resolver = template::TemplateResolver::new();
                resolver.resolve(template, &vars).unwrap_or_else(|_| base_url.clone())
            }
            Command::Nested { children, .. } => {
                let parts: Vec<&str> = query.splitn(2, ' ').collect();
                if let Some(child) = children.get(parts[0]) {
                    child.get_redirect_url(parts.get(1).unwrap_or(&""))
                } else {
                    String::new() // Empty string for invalid nested command
                }
            }
        }
    }
}
