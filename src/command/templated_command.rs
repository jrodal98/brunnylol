use rocket::http::RawStr;

use super::command::Command;

struct TemplatedString {
    template: String,
    placeholder: String,
}

impl TemplatedString {
    fn replace(&self, query: &str) -> String {
        self.template.replace(&self.placeholder, query)
    }
}

pub struct TemplatedCommand {
    bookmark: String,
    template: TemplatedString,
    description: String,
}

impl TemplatedCommand {
    fn process_query(&self, query: &str) -> String {
        RawStr::new(query).percent_encode().to_string()
    }
}

impl Command for TemplatedCommand {
    fn description(&self) -> &str {
        &self.description
    }

    fn get_redirect_url(&self, query: &str) -> String {
        match self.process_query(query).as_str() {
            "" => self.bookmark.clone(),
            query => self.template.replace(query),
        }
    }
}

impl TemplatedCommand {
    pub fn new(bookmark: &str, template: &str, placeholder: &str, description: &str) -> Self {
        Self {
            bookmark: bookmark.to_string(),
            template: TemplatedString {
                template: template.to_string(),
                placeholder: placeholder.to_string(),
            },
            description: description.to_string(),
        }
    }
}

#[test]
fn test_description() {
    let command = TemplatedCommand::new(
        "www.example.com",
        "www.example.com/%s",
        "%s",
        "a test website",
    );
    assert_eq!(command.description(), "a test website".to_string());
}

#[test]
fn test_empty_query_redirect() {
    let command = TemplatedCommand::new(
        "www.example.com",
        "www.example.com/%s",
        "%s",
        "a test website",
    );
    assert_eq!(command.get_redirect_url(""), "www.example.com".to_string());
}

#[test]
fn test_non_empty_query_redirect() {
    let command = TemplatedCommand::new(
        "www.example.com",
        "www.example.com/%s",
        "%s",
        "a test website",
    );
    assert_eq!(
        command.get_redirect_url("hello world"),
        "www.example.com/hello%20world".to_string()
    );
}

#[test]
fn test_wacky_placeholder() {
    let command = TemplatedCommand::new(
        "www.example.com",
        "www.example.com/LMAO",
        "LMAO",
        "a test website",
    );
    assert_eq!(
        command.get_redirect_url("hello world"),
        "www.example.com/hello%20world".to_string()
    );
}

#[test]
fn test_wrong_placeholder() {
    let command = TemplatedCommand::new(
        "www.example.com",
        "www.example.com/{}",
        "%s",
        "a test website",
    );
    assert_eq!(
        command.get_redirect_url("hello world"),
        "www.example.com/{}".to_string()
    );
}
