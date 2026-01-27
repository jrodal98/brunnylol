use std::collections::HashMap;

// Domain model for bookmark commands
// Uses enum instead of trait for better performance (no heap allocation, no vtable dispatch)

#[derive(Debug, Clone)]
pub enum Command {
    Simple {
        url: String,
        description: String,
    },
    Templated {
        base_url: String,  // Fallback URL when query is empty
        template: String,
        description: String,
        encode_query: bool,
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
            Command::Templated { description, .. } => description,
            Command::Nested { description, .. } => description,
        }
    }

    pub fn get_redirect_url(&self, query: &str) -> String {
        match self {
            Command::Simple { url, .. } => url.clone(),
            Command::Templated { base_url, template, encode_query, .. } => {
                // If query is empty, return base URL (not template with empty query)
                if query.trim().is_empty() {
                    base_url.clone()
                } else {
                    let query_encoded = if *encode_query {
                        urlencoding::encode(query).to_string()
                    } else {
                        query.to_string()
                    };
                    template.replace("{}", &query_encoded)
                }
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
