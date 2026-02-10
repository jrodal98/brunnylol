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
fn test_proton_product_with_shortcut() {
    // Real-world example: Proton bookmark with product shortcut
    let template = TemplateParser::parse(
        "https://{product|map[m:mail,cal:calendar,dr:drive]|options[mail,drive,sheets,docs,vpn,calendar,pass,wallet,authenticator,meet,lumo][strict]}.proton.me"
    ).unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("product".to_string(), "dr".to_string());

    let result = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(result, "https://drive.proton.me");
}

#[test]
fn test_proton_product_with_full_name() {
    let template = TemplateParser::parse(
        "https://{product|map[m:mail,cal:calendar,dr:drive]|options[mail,drive,sheets,docs,vpn,calendar,pass,wallet,authenticator,meet,lumo][strict]}.proton.me"
    ).unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("product".to_string(), "vpn".to_string());

    let result = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(result, "https://vpn.proton.me");
}

#[test]
fn test_proton_with_subproduct_shortcut() {
    // Test with both product and subproduct using shortcuts
    let template = TemplateParser::parse(
        "https://{product|map[m:mail,cal:calendar,dr:drive]|options[mail,drive,sheets,docs,vpn,calendar,pass,wallet,authenticator,meet,lumo][strict]}.proton.me/{subproduct?|map[p:photos,d:docs,s:sheets]|options[photos,docs,sheets][strict]}"
    ).unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("product".to_string(), "dr".to_string());
    vars.insert("subproduct".to_string(), "p".to_string());

    let result = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(result, "https://drive.proton.me/photos");
}

#[test]
fn test_proton_subproduct_optional_omitted() {
    // Test that optional subproduct can be omitted
    let template = TemplateParser::parse(
        "https://{product|map[m:mail,cal:calendar,dr:drive]|options[mail,drive,sheets,docs,vpn,calendar,pass,wallet,authenticator,meet,lumo][strict]}.proton.me/{subproduct?|map[p:photos,d:docs,s:sheets]|options[photos,docs,sheets][strict]}"
    ).unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("product".to_string(), "m".to_string());
    // subproduct not provided

    let result = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(result, "https://mail.proton.me/");
}

#[test]
fn test_proton_invalid_product_fails() {
    let template = TemplateParser::parse(
        "https://{product|map[m:mail,cal:calendar,dr:drive]|options[mail,drive,sheets,docs,vpn,calendar,pass,wallet,authenticator,meet,lumo][strict]}.proton.me"
    ).unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("product".to_string(), "invalid".to_string());

    let result = resolver.resolve(&template, &vars);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid value"));
}

#[test]
fn test_proton_invalid_subproduct_fails() {
    let template = TemplateParser::parse(
        "https://{product|map[m:mail,cal:calendar,dr:drive]|options[mail,drive,sheets,docs,vpn,calendar,pass,wallet,authenticator,meet,lumo][strict]}.proton.me/{subproduct?|map[p:photos,d:docs,s:sheets]|options[photos,docs,sheets][strict]}"
    ).unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("product".to_string(), "dr".to_string());
    vars.insert("subproduct".to_string(), "invalid".to_string());

    let result = resolver.resolve(&template, &vars);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid value"));
}

#[test]
fn test_map_before_strict_options_correct_order() {
    // Demonstrates the CORRECT pipeline order: map THEN options[strict]
    let template = TemplateParser::parse(
        "{var|map[short:longvalue]|options[longvalue,other][strict]}"
    ).unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("var".to_string(), "short".to_string());

    // With correct order: "short" -> map -> "longvalue" -> validate -> SUCCESS
    let result = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(result, "longvalue");
}

#[test]
fn test_options_before_map_wrong_order_fails() {
    // Demonstrates the WRONG pipeline order: options[strict] THEN map
    let template = TemplateParser::parse(
        "{var|options[longvalue,other][strict]|map[short:longvalue]}"
    ).unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("var".to_string(), "short".to_string());

    // With wrong order: "short" -> validate (fails!) -> never gets to map
    let result = resolver.resolve(&template, &vars);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid value 'short'"));
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
