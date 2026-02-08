use std::collections::HashMap;

pub mod template;

// Domain model for bookmark commands
// Uses enum instead of trait for better performance (no heap allocation, no vtable dispatch)

#[derive(Debug, Clone)]
pub enum Command {
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
            Command::Variable { description, .. } => description,
            Command::Nested { description, .. } => description,
        }
    }

    pub fn base_url(&self) -> &str {
        match self {
            Command::Variable { base_url, .. } => base_url,
            Command::Nested { .. } => "",
        }
    }

    pub fn get_redirect_url(&self, query: &str) -> String {
        match self {
            Command::Variable { base_url, template, .. } => {
                // For variable templates, map query to variables
                if query.trim().is_empty() {
                    return base_url.clone();
                }

                let mut vars = HashMap::new();
                let template_vars = template.variables();

                // Add {url} as built-in variable mapped to base_url
                vars.insert("url".to_string(), base_url.clone());

                // Check if template has a "query" variable
                let has_query_var = template_vars.iter().any(|v| v.name == "query");

                if has_query_var && template_vars.len() == 1 {
                    // Single query variable - map entire query
                    vars.insert("query".to_string(), query.to_string());
                } else if template_vars.is_empty() {
                    // No variables in template, just return base URL
                    return base_url.clone();
                } else {
                    // Multiple variables - split query by whitespace and map positionally
                    let query_parts: Vec<&str> = query.split_whitespace().collect();

                    // Filter out built-in variables like {url} for positional mapping
                    let user_vars: Vec<_> = template_vars.iter()
                        .filter(|v| v.name != "url")
                        .collect();

                    for (i, var) in user_vars.iter().enumerate() {
                        if i < query_parts.len() {
                            vars.insert(var.name.clone(), query_parts[i].to_string());
                        }
                        // If not enough args provided, variables will use defaults or be empty
                    }

                    // If there are extra args beyond the variables, join them as "query" if it exists
                    if query_parts.len() > user_vars.len() && has_query_var {
                        let remaining = query_parts[user_vars.len()..].join(" ");
                        vars.insert("query".to_string(), remaining);
                    }
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
