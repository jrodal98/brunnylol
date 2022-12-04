use std::collections::HashMap;

use crate::command::templated_command::TemplatedCommand;

use super::{command::Command, simple_bookmark::SimpleBookmark};

pub struct NestedCommand<'a> {
    bookmark: String,
    commands: HashMap<&'a str, Box<dyn Command>>,
    description: String,
}

impl<'a> Command for NestedCommand<'a> {
    // TODO: iterate over commands to form description
    fn description(&self) -> &str {
        &self.description
    }

    fn get_redirect_url(&self, query: &str) -> String {
        let mut splitted = query.splitn(2, " ");
        let alias = splitted.next().expect("Expected alias");

        if alias.is_empty() {
            return self.bookmark.clone();
        }

        let nested_query = splitted.next().unwrap_or_default();

        self.commands
            .get(alias)
            .expect(&format!("{} is not a valid command alias", alias))
            .get_redirect_url(nested_query)
    }
}

impl<'a> NestedCommand<'a> {
    pub fn new(
        bookmark: &str,
        commands: HashMap<&'a str, Box<dyn Command>>,
        description: &str,
    ) -> Self {
        Self {
            bookmark: bookmark.to_string(),
            commands,
            description: description.to_string(),
        }
    }
}

fn create_nested_command(should_recurse: bool) -> NestedCommand<'static> {
    let mut commands: HashMap<&str, Box<dyn Command>> = HashMap::new();
    // a single character should work
    commands.insert(
        "t",
        Box::new(TemplatedCommand::new(
            "www.template.com",
            "www.template.com/{}",
            "templated command",
        )),
    );
    // an entire word should work as well
    commands.insert(
        "bookmark",
        Box::new(SimpleBookmark::new("www.bookmark.com", "bookmark command")),
    );

    if should_recurse {
        // arbitrary nesting should be possible
        commands.insert("nested", Box::new(create_nested_command(false)));
    }

    NestedCommand::new("www.example.com", commands, "a test website")
}

#[test]
fn test_description() {
    let command = create_nested_command(true);
    assert_eq!(command.description(), "a test website".to_string());
}

#[test]
fn test_empty_query_redirect() {
    let command = create_nested_command(true);
    assert_eq!(command.get_redirect_url(""), "www.example.com".to_string());
}

#[test]
fn test_bookmark_command() {
    let command = create_nested_command(true);
    assert_eq!(
        command.get_redirect_url("bookmark"),
        "www.bookmark.com".to_string()
    );
}

#[test]
fn test_templated_command_alias_only() {
    let command = create_nested_command(true);
    assert_eq!(
        command.get_redirect_url("t"),
        "www.template.com".to_string()
    );
}

#[test]
fn test_templated_command_alias_and_query() {
    let command = create_nested_command(true);
    assert_eq!(
        command.get_redirect_url("t hello world"),
        "www.template.com/hello%20world".to_string()
    );
}

#[test]
fn test_nested_bookmark_command() {
    let command = create_nested_command(true);
    assert_eq!(
        command.get_redirect_url("nested bookmark"),
        "www.bookmark.com".to_string()
    );
}

#[test]
fn test_nested_templated_command_alias_only() {
    let command = create_nested_command(true);
    assert_eq!(
        command.get_redirect_url("nested t"),
        "www.template.com".to_string()
    );
}

#[test]
fn test_nested_templated_command_alias_and_query() {
    let command = create_nested_command(true);
    assert_eq!(
        command.get_redirect_url("nested t hello world"),
        "www.template.com/hello%20world".to_string()
    );
}
