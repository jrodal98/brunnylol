use crate::{
    domain::{
        bookmark_command::BookmarkCommand, nested_command::NestedCommand,
        templated_command::TemplatedCommand, Command,
    },
    config::yml_settings::YmlSettings,
};
use std::collections::HashMap;

const DEFAULT_CONFIG_FILE: &'static str = "commands.yml";

/// AliasAndCommand is an object that holds a command that the user can execute and an alias
/// that the user can use to reference that command.
pub struct AliasAndCommand {
    alias: String,
    command: Box<dyn Command>,
}

impl From<YmlSettings> for AliasAndCommand {
    fn from(value: YmlSettings) -> Self {
        let command_box = match (value.command, value.encode, value.nested) {
            (None, None, None) => {
                Box::new(BookmarkCommand::new(&value.url, &value.description)) as Box<dyn Command>
            }
            (Some(command), maybe_encode, None) => {
                let tc = TemplatedCommand::new(&value.url, &command, &value.description);
                Box::new(if !maybe_encode.unwrap_or(true) {
                    tc.with_no_query_encode()
                } else {
                    tc
                })
            }
            (None, None, Some(nested)) => {
                let alias_and_commands =
                    nested.into_iter().map(|settings| settings.into()).collect();
                let commands = AliasAndCommand::create_alias_to_bookmark_map(alias_and_commands);
                Box::new(NestedCommand::new(&value.url, commands, &value.description))
            }
            _ => panic!("Invalid yaml configuration"),
        };
        Self {
            alias: value.alias.clone(),
            command: command_box,
        }
    }
}

impl AliasAndCommand {
    fn create_alias_to_bookmark_map(
        alias_and_commands: Vec<AliasAndCommand>,
    ) -> HashMap<String, Box<dyn Command>> {
        let mut map = HashMap::new();
        for alias_and_command in alias_and_commands.into_iter() {
            if map
                .insert(alias_and_command.alias.clone(), alias_and_command.command)
                .is_some()
            {
                panic!("Duplicate alias: {}", alias_and_command.alias);
            }
        }
        map
    }

    pub fn get_alias_to_bookmark_map(maybe_yml: Option<&str>) -> HashMap<String, Box<dyn Command>> {
        let yml = std::fs::read_to_string(maybe_yml.unwrap_or(DEFAULT_CONFIG_FILE))
            .expect("Could not read file");
        let settings: Vec<YmlSettings> =
            serde_yaml::from_str(&yml).expect("Invalid yaml configuration");
        let alias_and_commands = settings.into_iter().map(AliasAndCommand::from).collect();
        Self::create_alias_to_bookmark_map(alias_and_commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_map() {
        // ensure that the map can be constructed
        let _ = AliasAndCommand::get_alias_to_bookmark_map(None);
    }

    #[test]
    #[should_panic(expected = "Duplicate alias: a")]
    fn test_duplicate_map_panics() {
        let aliases_and_commands = vec![
            AliasAndCommand {
                alias: "a".to_string(),
                command: Box::new(BookmarkCommand::new("www.example.com", "test website")),
            },
            AliasAndCommand {
                alias: "a".to_string(),
                command: Box::new(BookmarkCommand::new("www.example2.com", "test2 website")),
            },
        ];
        let _ = AliasAndCommand::create_alias_to_bookmark_map(aliases_and_commands);
    }
}
