use super::Command;

struct TemplatedString {
    template: String,
    placeholder: String,
}

impl TemplatedString {
    fn new(template: &str, placeholder: &str) -> Self {
        if !template.contains(placeholder) {
            panic!(
                "Invalid TemplateString - {} does not contain {}",
                template, placeholder
            );
        } else {
            Self {
                template: template.to_string(),
                placeholder: placeholder.to_string(),
            }
        }
    }
    fn replace(&self, query: &str) -> String {
        self.template.replace(&self.placeholder, query)
    }
}

pub struct TemplatedCommand {
    bookmark: String,
    template: TemplatedString,
    description: String,
    encode_query: bool,
}

impl TemplatedCommand {
    fn process_query(&self, query: &str) -> String {
        if self.encode_query {
            urlencoding::encode(query).to_string()
        } else {
            query.to_string()
        }
    }
}

impl Command for TemplatedCommand {
    fn description(&self) -> String {
        self.description.clone()
    }

    fn get_redirect_url(&self, query: &str) -> String {
        match self.process_query(query).as_str() {
            "" => self.bookmark.clone(),
            query => self.template.replace(query),
        }
    }
}

impl TemplatedCommand {
    pub fn new(bookmark: &str, template: &str, description: &str) -> Self {
        Self {
            bookmark: bookmark.to_string(),
            template: TemplatedString::new(template, "{}"),
            description: description.to_string(),
            encode_query: true,
        }
    }

    pub fn with_no_query_encode(mut self) -> Self {
        self.encode_query = false;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_description() {
        let command =
            TemplatedCommand::new("www.example.com", "www.example.com/{}", "a test website");
        assert_eq!(command.description(), "a test website".to_string());
    }

    #[test]
    fn test_empty_query_redirect() {
        let command =
            TemplatedCommand::new("www.example.com", "www.example.com/{}", "a test website");
        assert_eq!(command.get_redirect_url(""), "www.example.com".to_string());
    }

    #[test]
    fn test_non_empty_query_redirect() {
        let command =
            TemplatedCommand::new("www.example.com", "www.example.com/{}", "a test website");
        assert_eq!(
            command.get_redirect_url("hello world"),
            "www.example.com/hello%20world".to_string()
        );
    }

    #[test]
    fn test_no_encode() {
        let command =
            TemplatedCommand::new("www.example.com", "www.example.com/{}", "a test website")
                .with_no_query_encode();
        assert_eq!(
            command.get_redirect_url("hello/world"),
            "www.example.com/hello/world".to_string()
        );
    }

    #[test]
    #[should_panic(expected = "Invalid TemplateString - www.example.com/%s does not contain {}")]
    fn test_wrong_placeholder() {
        let command =
            TemplatedCommand::new("www.example.com", "www.example.com/%s", "a test website");
        assert_eq!(
            command.get_redirect_url("hello world"),
            "www.example.com/%s".to_string()
        );
    }
}
