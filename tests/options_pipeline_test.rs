// Tests for options pipeline

use brunnylol::domain::template::{TemplateParser, TemplateResolver, PipelineOp};
use std::collections::HashMap;

#[test]
fn test_parse_options_pipeline() {
    let template = TemplateParser::parse("{product|options[mail,drive,sheets]}").unwrap();
    let vars = template.variables();

    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].name, "product");
    assert_eq!(vars[0].pipelines.len(), 1);

    // Check options are parsed
    match &vars[0].pipelines[0] {
        PipelineOp::Options { values, strict } => {
            assert_eq!(values.len(), 3);
            assert_eq!(values[0], "mail");
            assert_eq!(values[1], "drive");
            assert_eq!(values[2], "sheets");
            assert!(!strict); // Not strict by default
        }
        _ => panic!("Expected Options pipeline"),
    }
}

#[test]
fn test_parse_options_pipeline_strict() {
    let template = TemplateParser::parse("{product|options[mail,drive][strict]}").unwrap();
    let vars = template.variables();

    match &vars[0].pipelines[0] {
        PipelineOp::Options { values, strict } => {
            assert_eq!(values.len(), 2);
            assert!(*strict);
        }
        _ => panic!("Expected Options pipeline"),
    }
}

#[test]
fn test_resolve_with_valid_option_strict() {
    let template = TemplateParser::parse("https://{product|options[mail,drive][strict]}.proton.me").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("product".to_string(), "mail".to_string());

    let result = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(result, "https://mail.proton.me");
}

#[test]
fn test_resolve_with_invalid_option_strict() {
    let template = TemplateParser::parse("https://{product|options[mail,drive][strict]}.proton.me").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("product".to_string(), "calendar".to_string());

    let result = resolver.resolve(&template, &vars);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid value"));
}

#[test]
fn test_resolve_with_invalid_option_non_strict() {
    let template = TemplateParser::parse("https://{product|options[mail,drive]}.proton.me").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("product".to_string(), "calendar".to_string());

    // Non-strict allows any value
    let result = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(result, "https://calendar.proton.me");
}
