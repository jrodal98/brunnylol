// Template parser using recursive descent

use anyhow::{bail, Result};

use super::ast::{PipelineOp, Template, TemplatePart, VariableExpr};

pub struct TemplateParser {
    input: String,
    pos: usize,
}

impl TemplateParser {
    pub fn parse(template: &str) -> Result<Template> {
        let mut parser = Self {
            input: template.to_string(),
            pos: 0,
        };
        parser.parse_template()
    }

    fn parse_template(&mut self) -> Result<Template> {
        let mut parts = Vec::new();
        let mut literal_buf = String::new();

        while self.pos < self.input.len() {
            if self.peek_char() == Some('{') {
                // Check for escaped braces {{
                if self.peek_ahead(1) == Some('{') {
                    // Escaped brace - add single { to literal
                    self.pos += 2;
                    literal_buf.push('{');
                } else {
                    // Start of variable - flush literal buffer first
                    if !literal_buf.is_empty() {
                        parts.push(TemplatePart::Literal(literal_buf.clone()));
                        literal_buf.clear();
                    }

                    // Parse variable
                    let var = self.parse_variable()?;
                    parts.push(TemplatePart::Variable(var));
                }
            } else if self.peek_char() == Some('}') {
                // Check for closing escaped brace }}
                if self.peek_ahead(1) == Some('}') {
                    self.pos += 2;
                    literal_buf.push('}');
                } else {
                    // Unexpected closing brace
                    bail!("Unexpected closing brace at position {}", self.pos);
                }
            } else {
                // Regular character
                literal_buf.push(self.consume_char()?);
            }
        }

        // Flush remaining literal
        if !literal_buf.is_empty() {
            parts.push(TemplatePart::Literal(literal_buf));
        }

        Ok(Template::new(parts))
    }

    fn parse_variable(&mut self) -> Result<VariableExpr> {
        // Consume opening {
        self.expect_char('{')?;

        // Skip whitespace
        self.skip_whitespace();

        // Parse variable name
        let name = self.parse_variable_name()?;

        // Skip whitespace
        self.skip_whitespace();

        // Check for modifiers (?, =)
        let mut is_optional = false;
        let mut default = None;

        if self.peek_char() == Some('?') {
            is_optional = true;
            self.consume_char()?;
            self.skip_whitespace();
        } else if self.peek_char() == Some('=') {
            self.consume_char()?;
            self.skip_whitespace();
            default = Some(self.parse_default_value()?);
            self.skip_whitespace();
        }

        // Parse pipelines
        let pipelines = self.parse_pipelines()?;

        // Skip whitespace
        self.skip_whitespace();

        // Expect closing }
        self.expect_char('}')?;

        Ok(VariableExpr {
            name,
            is_optional,
            default,
            pipelines,
        })
    }

    fn parse_variable_name(&mut self) -> Result<String> {
        let mut name = String::new();

        while let Some(ch) = self.peek_char() {
            if ch.is_alphanumeric() || ch == '_' {
                name.push(self.consume_char()?);
            } else if ch == '?' || ch == '=' || ch == '|' || ch == '}' || ch.is_whitespace() {
                break;
            } else {
                bail!(
                    "Invalid character '{}' in variable name at position {}",
                    ch,
                    self.pos
                );
            }
        }

        // Empty variable name maps to "query"
        if name.is_empty() {
            Ok("query".to_string())
        } else {
            Ok(name)
        }
    }

    fn parse_default_value(&mut self) -> Result<String> {
        let mut value = String::new();

        while let Some(ch) = self.peek_char() {
            if ch == '}' || ch == '|' {
                break;
            }
            value.push(self.consume_char()?);
        }

        Ok(value.trim().to_string())
    }

    fn parse_pipelines(&mut self) -> Result<Vec<PipelineOp>> {
        let mut pipelines = Vec::new();

        while self.peek_char() == Some('|') {
            self.consume_char()?; // consume |
            self.skip_whitespace();

            let op = self.parse_pipeline_op()?;
            pipelines.push(op);
            self.skip_whitespace();
        }

        Ok(pipelines)
    }

    fn parse_pipeline_op(&mut self) -> Result<PipelineOp> {
        // Check for negation
        let negated = if self.peek_char() == Some('!') {
            self.consume_char()?;
            true
        } else {
            false
        };

        // Parse operation name
        let op_name = self.parse_identifier()?;

        match (op_name.as_str(), negated) {
            ("encode", false) => Ok(PipelineOp::Encode),
            ("encode", true) => Ok(PipelineOp::NoEncode),
            ("trim", false) => Ok(PipelineOp::Trim),
            ("trim", true) => bail!("Cannot negate 'trim' operation"),
            ("options", false) => {
                // Parse options[val1,val2,val3] or options[val1,val2][strict]
                self.skip_whitespace();
                if self.peek_char() != Some('[') {
                    bail!("Expected '[' after 'options' at position {}", self.pos);
                }
                self.consume_char()?; // consume [

                // Parse comma-separated values
                let mut values = Vec::new();
                let mut current_value = String::new();

                loop {
                    self.skip_whitespace();
                    match self.peek_char() {
                        Some(']') => {
                            if !current_value.is_empty() {
                                values.push(current_value.trim().to_string());
                                current_value.clear();
                            }
                            self.consume_char()?; // consume ]
                            break;
                        }
                        Some(',') => {
                            if !current_value.is_empty() {
                                values.push(current_value.trim().to_string());
                                current_value.clear();
                            }
                            self.consume_char()?; // consume ,
                        }
                        Some(ch) => {
                            current_value.push(ch);
                            self.consume_char()?;
                        }
                        None => bail!("Unexpected end of input in options list"),
                    }
                }

                // Check for [strict] modifier
                self.skip_whitespace();
                let strict = if self.peek_char() == Some('[') {
                    self.consume_char()?; // consume [
                    let modifier = self.parse_identifier()?;
                    self.skip_whitespace();
                    self.expect_char(']')?;
                    modifier == "strict"
                } else {
                    false
                };

                Ok(PipelineOp::Options { values, strict })
            }
            ("options", true) => bail!("Cannot negate 'options' operation"),
            ("map", false) => {
                // Parse map[key1:value1,key2:value2,...]
                self.skip_whitespace();
                if self.peek_char() != Some('[') {
                    bail!("Expected '[' after 'map' at position {}", self.pos);
                }
                self.consume_char()?; // consume [

                // Parse comma-separated key:value pairs
                let mut mappings = Vec::new();
                let mut current_pair = String::new();

                loop {
                    match self.peek_char() {
                        Some(']') => {
                            if !current_pair.is_empty() {
                                // Parse the key:value pair
                                let parts: Vec<&str> = current_pair.splitn(2, ':').collect();
                                if parts.len() != 2 {
                                    bail!("Invalid map syntax: expected 'key:value' but got '{}'", current_pair);
                                }
                                mappings.push((
                                    parts[0].trim().to_string(),
                                    parts[1].trim().to_string(),
                                ));
                                current_pair.clear();
                            }
                            self.consume_char()?; // consume ]
                            break;
                        }
                        Some(',') => {
                            if !current_pair.is_empty() {
                                // Parse the key:value pair
                                let parts: Vec<&str> = current_pair.splitn(2, ':').collect();
                                if parts.len() != 2 {
                                    bail!("Invalid map syntax: expected 'key:value' but got '{}'", current_pair);
                                }
                                mappings.push((
                                    parts[0].trim().to_string(),
                                    parts[1].trim().to_string(),
                                ));
                                current_pair.clear();
                            }
                            self.consume_char()?; // consume ,
                        }
                        Some(ch) => {
                            current_pair.push(ch);
                            self.consume_char()?;
                        }
                        None => bail!("Unexpected end of input in map list"),
                    }
                }

                if mappings.is_empty() {
                    bail!("Map operation requires at least one mapping");
                }

                // Check for duplicate keys
                let mut seen_keys = std::collections::HashSet::new();
                for (key, _) in &mappings {
                    if !seen_keys.insert(key.as_str()) {
                        bail!("Duplicate key '{}' in map operation", key);
                    }
                }

                Ok(PipelineOp::Map { mappings })
            }
            ("map", true) => bail!("Cannot negate 'map' operation"),
            (name, _) => bail!("Unknown pipeline operation: {}", name),
        }
    }

    fn parse_identifier(&mut self) -> Result<String> {
        let mut ident = String::new();

        while let Some(ch) = self.peek_char() {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(self.consume_char()?);
            } else {
                break;
            }
        }

        if ident.is_empty() {
            bail!("Expected identifier at position {}", self.pos);
        }

        Ok(ident)
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn peek_ahead(&self, offset: usize) -> Option<char> {
        self.input[self.pos..].chars().nth(offset)
    }

    fn consume_char(&mut self) -> Result<char> {
        let ch = self.peek_char()
            .ok_or_else(|| anyhow::anyhow!("Unexpected end of input at position {}", self.pos))?;
        self.pos += ch.len_utf8();
        Ok(ch)
    }

    fn expect_char(&mut self, expected: char) -> Result<()> {
        match self.peek_char() {
            Some(ch) if ch == expected => {
                self.consume_char()?;
                Ok(())
            }
            Some(ch) => bail!(
                "Expected '{}' but found '{}' at position {}",
                expected,
                ch,
                self.pos
            ),
            None => bail!("Expected '{}' but found end of input", expected),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_variable() {
        let template = TemplateParser::parse("https://example.com/{query}").unwrap();
        assert_eq!(template.parts.len(), 2);

        match &template.parts[0] {
            TemplatePart::Literal(s) => assert_eq!(s, "https://example.com/"),
            _ => panic!("Expected literal"),
        }

        match &template.parts[1] {
            TemplatePart::Variable(var) => {
                assert_eq!(var.name, "query");
                assert!(!var.is_optional);
                assert!(var.default.is_none());
            }
            _ => panic!("Expected variable"),
        }
    }

    #[test]
    fn test_parse_optional_variable() {
        let template = TemplateParser::parse("/path/{repo?}").unwrap();
        let vars = template.variables();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name, "repo");
        assert!(vars[0].is_optional);
    }

    #[test]
    fn test_parse_variable_with_default() {
        let template = TemplateParser::parse("/path/{author=default}").unwrap();
        let vars = template.variables();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name, "author");
        assert_eq!(vars[0].default, Some("default".to_string()));
    }

    #[test]
    fn test_parse_empty_variable_maps_to_query() {
        let template = TemplateParser::parse("https://example.com/search?q={}").unwrap();
        let vars = template.variables();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name, "query");
    }

    #[test]
    fn test_parse_escaped_braces() {
        let template = TemplateParser::parse("https://example.com/{{escaped}}").unwrap();
        assert_eq!(template.parts.len(), 1);

        match &template.parts[0] {
            TemplatePart::Literal(s) => assert_eq!(s, "https://example.com/{escaped}"),
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn test_parse_pipeline_encode() {
        let template = TemplateParser::parse("{query|encode}").unwrap();
        let vars = template.variables();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].pipelines.len(), 1);
        assert_eq!(vars[0].pipelines[0], PipelineOp::Encode);
    }

    #[test]
    fn test_parse_pipeline_noencode() {
        let template = TemplateParser::parse("{query|!encode}").unwrap();
        let vars = template.variables();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].pipelines.len(), 1);
        assert_eq!(vars[0].pipelines[0], PipelineOp::NoEncode);
    }

    #[test]
    fn test_parse_pipeline_trim() {
        let template = TemplateParser::parse("{query|trim}").unwrap();
        let vars = template.variables();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].pipelines.len(), 1);
        assert_eq!(vars[0].pipelines[0], PipelineOp::Trim);
    }

    #[test]
    fn test_parse_multiple_pipelines() {
        let template = TemplateParser::parse("{query|trim|encode}").unwrap();
        let vars = template.variables();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].pipelines.len(), 2);
        assert_eq!(vars[0].pipelines[0], PipelineOp::Trim);
        assert_eq!(vars[0].pipelines[1], PipelineOp::Encode);
    }

    #[test]
    fn test_parse_whitespace_handling() {
        let template = TemplateParser::parse("{ query | trim | encode }").unwrap();
        let vars = template.variables();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name, "query");
        assert_eq!(vars[0].pipelines.len(), 2);
    }

    #[test]
    fn test_parse_multiple_variables() {
        let template = TemplateParser::parse("/{page}/{author}/{repo}").unwrap();
        let vars = template.variables();
        assert_eq!(vars.len(), 3);
        assert_eq!(vars[0].name, "page");
        assert_eq!(vars[1].name, "author");
        assert_eq!(vars[2].name, "repo");
    }

    #[test]
    fn test_parse_map_single() {
        let template = TemplateParser::parse("{var|map[cal:calendar]}").unwrap();
        let vars = template.variables();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].pipelines.len(), 1);
        match &vars[0].pipelines[0] {
            PipelineOp::Map { mappings } => {
                assert_eq!(mappings.len(), 1);
                assert_eq!(mappings[0].0, "cal");
                assert_eq!(mappings[0].1, "calendar");
            }
            _ => panic!("Expected Map pipeline"),
        }
    }

    #[test]
    fn test_parse_map_multiple() {
        let template = TemplateParser::parse("{var|map[cal:calendar,sh:sheets,dc:docs]}").unwrap();
        let vars = template.variables();
        assert_eq!(vars.len(), 1);
        match &vars[0].pipelines[0] {
            PipelineOp::Map { mappings } => {
                assert_eq!(mappings.len(), 3);
                assert_eq!(mappings[0], ("cal".to_string(), "calendar".to_string()));
                assert_eq!(mappings[1], ("sh".to_string(), "sheets".to_string()));
                assert_eq!(mappings[2], ("dc".to_string(), "docs".to_string()));
            }
            _ => panic!("Expected Map pipeline"),
        }
    }

    #[test]
    fn test_parse_map_with_whitespace() {
        let template = TemplateParser::parse("{var|map[ cal : calendar , sh : sheets ]}").unwrap();
        let vars = template.variables();
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
    fn test_parse_map_chained_with_options() {
        let template = TemplateParser::parse("{var|options[calendar,sheets,docs]|map[cal:calendar]}").unwrap();
        let vars = template.variables();
        assert_eq!(vars[0].pipelines.len(), 2);
        match &vars[0].pipelines[0] {
            PipelineOp::Options { values, strict } => {
                assert_eq!(values.len(), 3);
                assert!(!strict);
            }
            _ => panic!("Expected Options pipeline"),
        }
        match &vars[0].pipelines[1] {
            PipelineOp::Map { mappings } => {
                assert_eq!(mappings.len(), 1);
                assert_eq!(mappings[0], ("cal".to_string(), "calendar".to_string()));
            }
            _ => panic!("Expected Map pipeline"),
        }
    }

    #[test]
    fn test_parse_map_with_colon_in_value() {
        let template = TemplateParser::parse("{var|map[g:https://google.com,gh:https://github.com]}").unwrap();
        let vars = template.variables();
        match &vars[0].pipelines[0] {
            PipelineOp::Map { mappings } => {
                assert_eq!(mappings.len(), 2);
                assert_eq!(mappings[0], ("g".to_string(), "https://google.com".to_string()));
                assert_eq!(mappings[1], ("gh".to_string(), "https://github.com".to_string()));
            }
            _ => panic!("Expected Map pipeline"),
        }
    }

    #[test]
    fn test_parse_map_empty_fails() {
        let result = TemplateParser::parse("{var|map[]}");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires at least one mapping"));
    }

    #[test]
    fn test_parse_map_missing_colon_fails() {
        let result = TemplateParser::parse("{var|map[nocolon]}");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid map syntax"));
    }

    #[test]
    fn test_parse_map_negated_fails() {
        let result = TemplateParser::parse("{var|!map[a:b]}");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot negate 'map'"));
    }

    #[test]
    fn test_parse_map_duplicate_keys_fails() {
        let result = TemplateParser::parse("{var|map[cal:calendar,cal:contacts]}");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Duplicate key 'cal'"));
    }
}
