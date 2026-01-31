// Form generation for variable templates

use std::collections::HashMap;

use super::ast::{Template, TemplateMetadata};

/// Represents a variable for form rendering
#[derive(Debug, Clone)]
pub struct FormVariable {
    pub name: String,
    pub is_required: bool,
    pub default_value: Option<String>,
    pub current_value: Option<String>,
    pub options: Option<Vec<String>>,
    pub strict: bool,
}

/// Build form data from template and metadata
pub fn build_form_data(
    template: &Template,
    metadata: Option<&TemplateMetadata>,
    prefilled: &HashMap<String, String>,
) -> Vec<FormVariable> {
    let mut form_vars = Vec::new();

    for var_expr in template.variables() {
        // Skip built-in variables (url is auto-populated from base_url)
        if var_expr.name == "url" {
            continue;
        }

        // Find metadata for this variable if it exists
        let var_metadata = metadata.and_then(|m| {
            m.variables.iter().find(|v| v.name == var_expr.name)
        });

        // Extract options and strict from pipelines
        let (options, strict) = var_expr.pipelines.iter()
            .find_map(|p| {
                if let super::ast::PipelineOp::Options { values, strict } = p {
                    Some((Some(values.clone()), *strict))
                } else {
                    None
                }
            })
            .unwrap_or((None, false));

        let is_required = !var_expr.is_optional && var_expr.default.is_none();
        let default_value = var_expr.default.clone();
        let current_value = prefilled.get(&var_expr.name).cloned();

        form_vars.push(FormVariable {
            name: var_expr.name.clone(),
            is_required,
            default_value,
            current_value,
            options,
            strict,
        });
    }

    form_vars
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::template::parser::TemplateParser;

    #[test]
    fn test_build_form_data_simple() {
        let template = TemplateParser::parse("/{page}").unwrap();
        let prefilled = HashMap::new();

        let form_data = build_form_data(&template, None, &prefilled);

        assert_eq!(form_data.len(), 1);
        assert_eq!(form_data[0].name, "page");
        assert!(form_data[0].is_required);
    }

    #[test]
    fn test_build_form_data_with_optional() {
        let template = TemplateParser::parse("/{page}/{repo?}").unwrap();
        let prefilled = HashMap::new();

        let form_data = build_form_data(&template, None, &prefilled);

        assert_eq!(form_data.len(), 2);
        assert_eq!(form_data[0].name, "page");
        assert!(form_data[0].is_required);
        assert_eq!(form_data[1].name, "repo");
        assert!(!form_data[1].is_required);
    }

    #[test]
    fn test_build_form_data_with_default() {
        let template = TemplateParser::parse("/{author=default}").unwrap();
        let prefilled = HashMap::new();

        let form_data = build_form_data(&template, None, &prefilled);

        assert_eq!(form_data.len(), 1);
        assert_eq!(form_data[0].name, "author");
        assert!(!form_data[0].is_required); // Has default, so not required
        assert_eq!(form_data[0].default_value, Some("default".to_string()));
    }

    #[test]
    fn test_build_form_data_with_prefill() {
        let template = TemplateParser::parse("/{page}").unwrap();
        let mut prefilled = HashMap::new();
        prefilled.insert("page".to_string(), "test".to_string());

        let form_data = build_form_data(&template, None, &prefilled);

        assert_eq!(form_data.len(), 1);
        assert_eq!(form_data[0].current_value, Some("test".to_string()));
    }
}
