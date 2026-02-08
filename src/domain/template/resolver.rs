// Template variable resolution

use anyhow::{Context, Result};
use std::collections::HashMap;

use super::ast::{PipelineOp, Template, TemplatePart};
use super::pipeline::PipelineRegistry;

/// Resolves variables in a template to produce final URL
pub struct TemplateResolver {
    pipeline_registry: PipelineRegistry,
}

impl TemplateResolver {
    pub fn new() -> Self {
        Self {
            pipeline_registry: PipelineRegistry::new(),
        }
    }

    /// Resolve template with provided variable values
    pub fn resolve(
        &self,
        template: &Template,
        variables: &HashMap<String, String>,
    ) -> Result<String> {
        let mut result = String::new();

        for part in &template.parts {
            match part {
                TemplatePart::Literal(s) => result.push_str(s),
                TemplatePart::Variable(var_expr) => {
                    // Get value from variables map, default, or skip if optional
                    // Treat empty strings as missing for optional variables
                    let value = variables
                        .get(&var_expr.name)
                        .filter(|v| !v.is_empty())
                        .cloned()
                        .or_else(|| var_expr.default.clone());

                    match value {
                        Some(mut val) => {
                            // Apply pipeline operations
                            for pipeline_op in &var_expr.pipelines {
                                val = self.apply_pipeline(&val, pipeline_op)?;
                            }

                            // Apply default encoding ONLY if no encoding-related pipeline exists
                            // EXCEPT for built-in variables like {url} which should never be encoded by default
                            let has_encoding_pipeline = var_expr.pipelines.iter()
                                .any(|p| matches!(p, PipelineOp::Encode | PipelineOp::NoEncode));

                            let is_builtin_noenc = var_expr.name == "url"; // {url} built-in should not be encoded

                            if !has_encoding_pipeline && !is_builtin_noenc {
                                val = urlencoding::encode(&val).to_string();
                            }

                            result.push_str(&val);
                        }
                        None => {
                            if !var_expr.is_optional {
                                anyhow::bail!("Missing required variable: {}", var_expr.name);
                            }
                            // Optional variable with no value - omit segment
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    fn apply_pipeline(&self, value: &str, op: &PipelineOp) -> Result<String> {
        match op {
            PipelineOp::Encode => {
                let pipeline_op = self
                    .pipeline_registry
                    .get("encode")
                    .context("encode operation not registered")?;
                pipeline_op.apply(value)
            }
            PipelineOp::NoEncode => Ok(value.to_string()),
            PipelineOp::Trim => {
                let pipeline_op = self
                    .pipeline_registry
                    .get("trim")
                    .context("trim operation not registered")?;
                pipeline_op.apply(value)
            }
            PipelineOp::Options { values, strict } => {
                // Validate value against options if strict
                if *strict && !values.contains(&value.to_string()) {
                    anyhow::bail!(
                        "Invalid value '{}'. Must be one of: {}",
                        value,
                        values.join(", ")
                    );
                }
                // Options pipeline doesn't transform the value, just validates
                Ok(value.to_string())
            }
        }
    }

    /// Check if all required variables are provided
    pub fn validate_variables(
        &self,
        template: &Template,
        variables: &HashMap<String, String>,
    ) -> Result<Vec<String>> {
        let mut missing = Vec::new();

        for var in template.variables() {
            if !var.is_optional
                && var.default.is_none()
                && !variables.contains_key(&var.name)
            {
                missing.push(var.name.clone());
            }
        }

        Ok(missing)
    }
}

impl Default for TemplateResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::template::parser::TemplateParser;

    #[test]
    fn test_resolve_simple_variable() {
        let template = TemplateParser::parse("https://example.com/{query}").unwrap();
        let resolver = TemplateResolver::new();

        let mut vars = HashMap::new();
        vars.insert("query".to_string(), "rust templates".to_string());

        let result = resolver.resolve(&template, &vars).unwrap();
        assert_eq!(result, "https://example.com/rust%20templates");
    }

    #[test]
    fn test_resolve_optional_variable_missing() {
        let template = TemplateParser::parse("/path/{repo?}/file").unwrap();
        let resolver = TemplateResolver::new();

        let vars = HashMap::new();
        let result = resolver.resolve(&template, &vars).unwrap();
        // TODO: Implement smart segment omission to get "/path/file" instead of "/path//file"
        assert_eq!(result, "/path//file");
    }

    #[test]
    fn test_resolve_optional_variable_empty_string() {
        // Test that empty strings for optional variables are treated as missing
        let template = TemplateParser::parse("/path/{repo?}/file").unwrap();
        let resolver = TemplateResolver::new();

        let mut vars = HashMap::new();
        vars.insert("repo".to_string(), "".to_string());
        let result = resolver.resolve(&template, &vars).unwrap();
        // Empty string should be treated as missing, same as above test
        assert_eq!(result, "/path//file");
    }

    #[test]
    fn test_resolve_variable_with_default() {
        let template = TemplateParser::parse("/{author=default}/repo").unwrap();
        let resolver = TemplateResolver::new();

        let vars = HashMap::new();
        let result = resolver.resolve(&template, &vars).unwrap();
        assert_eq!(result, "/default/repo");
    }

    #[test]
    fn test_resolve_with_trim_pipeline() {
        let template = TemplateParser::parse("{query|trim}").unwrap();
        let resolver = TemplateResolver::new();

        let mut vars = HashMap::new();
        vars.insert("query".to_string(), "  hello  ".to_string());

        let result = resolver.resolve(&template, &vars).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_resolve_with_explicit_encode() {
        let template = TemplateParser::parse("{query|encode}").unwrap();
        let resolver = TemplateResolver::new();

        let mut vars = HashMap::new();
        vars.insert("query".to_string(), "hello world".to_string());

        let result = resolver.resolve(&template, &vars).unwrap();
        assert_eq!(result, "hello%20world");
    }

    #[test]
    fn test_resolve_with_noencode() {
        let template = TemplateParser::parse("{path|!encode}").unwrap();
        let resolver = TemplateResolver::new();

        let mut vars = HashMap::new();
        vars.insert("path".to_string(), "foo/bar".to_string());

        let result = resolver.resolve(&template, &vars).unwrap();
        assert_eq!(result, "foo/bar");
    }

    #[test]
    fn test_resolve_multiple_pipelines() {
        let template = TemplateParser::parse("{query|trim|encode}").unwrap();
        let resolver = TemplateResolver::new();

        let mut vars = HashMap::new();
        vars.insert("query".to_string(), "  hello world  ".to_string());

        let result = resolver.resolve(&template, &vars).unwrap();
        assert_eq!(result, "hello%20world");
    }

    #[test]
    fn test_validate_variables_missing() {
        let template = TemplateParser::parse("/{page}/{repo}").unwrap();
        let resolver = TemplateResolver::new();

        let mut vars = HashMap::new();
        vars.insert("page".to_string(), "test".to_string());

        let missing = resolver.validate_variables(&template, &vars).unwrap();
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], "repo");
    }

    #[test]
    fn test_validate_variables_all_present() {
        let template = TemplateParser::parse("/{page}/{repo}").unwrap();
        let resolver = TemplateResolver::new();

        let mut vars = HashMap::new();
        vars.insert("page".to_string(), "test".to_string());
        vars.insert("repo".to_string(), "rust".to_string());

        let missing = resolver.validate_variables(&template, &vars).unwrap();
        assert!(missing.is_empty());
    }
}
