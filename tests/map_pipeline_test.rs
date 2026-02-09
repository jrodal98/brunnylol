// Tests for map pipeline

use brunnylol::domain::template::{TemplateParser, TemplateResolver, PipelineOp};
use std::collections::HashMap;

#[test]
fn test_parse_map_pipeline() {
    let template = TemplateParser::parse("{app|map[cal:calendar,sh:sheets]}").unwrap();
    let vars = template.variables();

    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].name, "app");
    assert_eq!(vars[0].pipelines.len(), 1);

    // Check mappings are parsed
    match &vars[0].pipelines[0] {
        PipelineOp::Map { mappings } => {
            assert_eq!(mappings.len(), 2);
            assert_eq!(mappings[0], ("cal".to_string(), "calendar".to_string()));
            assert_eq!(mappings[1], ("sh".to_string(), "sheets".to_string()));
        }
        _ => panic!("Expected Map pipeline"),
    }
}

#[test]
fn test_resolve_with_map_matched() {
    let template = TemplateParser::parse("https://google.com/{app|map[cal:calendar]|!encode}").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("app".to_string(), "cal".to_string());

    let result = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(result, "https://google.com/calendar");
}

#[test]
fn test_resolve_with_map_unmapped_passthrough() {
    let template = TemplateParser::parse("https://google.com/{app|map[cal:calendar]|!encode}").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("app".to_string(), "sheets".to_string());

    // sheets is not in the map, so it should pass through unchanged
    let result = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(result, "https://google.com/sheets");
}

#[test]
fn test_resolve_with_map_multiple_mappings() {
    let template = TemplateParser::parse("{app|map[cal:calendar,sh:sheets,dc:docs]|!encode}").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("app".to_string(), "sh".to_string());

    let result = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(result, "sheets");
}

#[test]
fn test_resolve_with_map_chained_with_options() {
    let template = TemplateParser::parse("{app|options[calendar,sheets,docs]|map[cal:calendar]|!encode}").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("app".to_string(), "cal".to_string());

    let result = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(result, "calendar");
}

#[test]
fn test_resolve_with_map_and_auto_encode() {
    let template = TemplateParser::parse("{app|map[my app:my application]}").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("app".to_string(), "my app".to_string());

    // Map transforms "my app" to "my application", then auto-encoding happens
    let result = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(result, "my%20application");
}

#[test]
fn test_resolve_with_map_no_encode() {
    let template = TemplateParser::parse("{app|map[cal:calendar]|!encode}").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("app".to_string(), "cal".to_string());

    let result = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(result, "calendar");
}
