use std::collections::HashMap;

use super::Command;

pub struct NestedCommand {
    bookmark: String,
    commands: HashMap<String, Box<dyn Command>>,
    description: String,
}

impl Command for NestedCommand {
    fn description(&self) -> String {
        let mut description = self.description.clone();
        for (alias, command) in self.commands.iter() {
            description.push_str(&format!("|{}: {}", alias, command.description()));
        }
        description
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

impl NestedCommand {
    pub fn new(
        bookmark: &str,
        commands: HashMap<String, Box<dyn Command>>,
        description: &str,
    ) -> Self {
        Self {
            bookmark: bookmark.to_string(),
            commands,
            description: description.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::command::{bookmark_command::BookmarkCommand, templated_command::TemplatedCommand};

    use super::*;

    #[allow(dead_code)]
    fn create_nested_command(should_recurse: bool) -> NestedCommand {
        let mut commands: HashMap<String, Box<dyn Command>> = HashMap::new();
        // a single character should work
        commands.insert(
            "t".to_string(),
            Box::new(TemplatedCommand::new(
                "www.template.com",
                "www.template.com/{}",
                "templated command",
            )),
        );
        // an entire word should work as well
        commands.insert(
            "bookmark".to_string(),
            Box::new(BookmarkCommand::new("www.bookmark.com", "bookmark command")),
        );

        if should_recurse {
            // arbitrary nesting should be possible
            commands.insert("nested".to_string(), Box::new(create_nested_command(false)));
        }

        NestedCommand::new("www.example.com", commands, "a test website")
    }

    #[test]
    fn test_description() {
        let command = create_nested_command(true);
        let description = command.description();
        assert_eq!(description.contains("a test website"), true);
        assert_eq!(description.contains("|nested: a test website"), true);
        assert_eq!(description.contains("|bookmark: bookmark command"), true);
        assert_eq!(description.contains("|t: templated command"), true);
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
}
