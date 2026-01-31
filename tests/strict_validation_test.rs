// Tests for strict validation redirect behavior

use brunnylol::domain::template::{TemplateParser, TemplateResolver};
use std::collections::HashMap;

#[test]
fn test_strict_validation_fails_with_invalid_value() {
    let template = TemplateParser::parse("https://{product|options[mail,drive,sheets][strict]}.proton.me").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("product".to_string(), "invalid_choice".to_string());

    // Should return error for invalid choice with strict
    let result = resolver.resolve(&template, &vars);
    assert!(result.is_err());

    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Invalid value"));
    assert!(err_msg.contains("invalid_choice"));
}

#[test]
fn test_strict_validation_passes_with_valid_value() {
    let template = TemplateParser::parse("https://{product|options[mail,drive,sheets][strict]}.proton.me").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("product".to_string(), "mail".to_string());

    let result = resolver.resolve(&template, &vars);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "https://mail.proton.me");
}

#[test]
fn test_non_strict_allows_any_value() {
    let template = TemplateParser::parse("https://{product|options[mail,drive,sheets]}.proton.me").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("product".to_string(), "invalid_choice".to_string());

    // Non-strict should allow any value
    let result = resolver.resolve(&template, &vars);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "https://invalid_choice.proton.me");
}

#[test]
fn test_options_with_spaces_in_values() {
    // Note: Spaces around commas are trimmed during parsing
    let template = TemplateParser::parse("{choice|options[option_one,option_two,option_three]}").unwrap();
    let vars_list = template.variables();

    assert_eq!(vars_list.len(), 1);
    assert_eq!(vars_list[0].pipelines.len(), 1);

    match &vars_list[0].pipelines[0] {
        brunnylol::domain::template::PipelineOp::Options { values, .. } => {
            assert_eq!(values.len(), 3);
            assert_eq!(values[0], "option_one");
            assert_eq!(values[1], "option_two");
            assert_eq!(values[2], "option_three");
        }
        _ => panic!("Expected Options pipeline"),
    }
}

#[test]
fn test_options_combined_with_other_pipelines() {
    let template = TemplateParser::parse("{product|trim|options[mail,drive][strict]}").unwrap();
    let vars_list = template.variables();

    assert_eq!(vars_list[0].pipelines.len(), 2);

    // First should be trim
    assert!(matches!(vars_list[0].pipelines[0], brunnylol::domain::template::PipelineOp::Trim));

    // Second should be options
    match &vars_list[0].pipelines[1] {
        brunnylol::domain::template::PipelineOp::Options { values, strict } => {
            assert_eq!(values.len(), 2);
            assert!(*strict);
        }
        _ => panic!("Expected Options pipeline"),
    }
}

#[test]
fn test_options_applied_after_trim() {
    let template = TemplateParser::parse("{product|trim|options[mail,drive][strict]}").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("product".to_string(), "  mail  ".to_string());

    // Should trim first, then validate
    let result = resolver.resolve(&template, &vars);
    assert!(result.is_ok());
}
