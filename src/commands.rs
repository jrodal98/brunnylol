use crate::{
    command::{
        bookmark_command::BookmarkCommand, nested_command::NestedCommand,
        templated_command::TemplatedCommand, Command,
    },
    yml_settings::YmlSettings,
};
use std::collections::HashMap;

/// AliasAndCommand is an object that holds a command that the user can execute and an alias
/// that the user can use to reference that command.
pub struct AliasAndCommand {
    alias: String,
    command: Box<dyn Command>,
}

/// The bl macro is a flexible macro that can be used to define different kinds of AliasAndCommand
/// objects. AliasAndCommand is an object that holds a command that the user can execute and an alias
/// that the user can use to reference that command.
///
/// The bl macro takes different arguments depending on the type of AliasAndCommand object that it
/// should create. The first argument is always an alias for the command, which is a string that the
/// user can use to reference the command.
///
/// If the macro is called with three arguments ($alias, $url, $command), it creates an AliasAndCommand
/// object with a TemplatedCommand inside. TemplatedCommand is a type of command that can be executed
/// by substituting placeholders in a URL with values specified by the user. The $url and $command
/// arguments are used to create the TemplatedCommand object.
///
/// If the macro is called with four arguments and the string "no_encode" as the fourth argument
/// ($alias, $url, $command, "no_encode"), it creates an AliasAndCommand object with a TemplatedCommand
///     inside. The TemplatedCommand is created in the same way as before, but the with_no_query_encode
///     method is called on the TemplatedCommand before it is used to create the AliasAndCommand
///     object. This method disables query encoding for the TemplatedCommand, which affects how the URL
///     is constructed when the command is executed.
///
///     If the macro is called with three arguments ($alias, $url, $description), it creates an
///     AliasAndCommand object with a BookmarkCommand inside. BookmarkCommand is a type of command that
///     simply navigates to a pre-defined URL when executed. The $url and $description arguments are
///     used to create the BookmarkCommand object.
///
///     If the macro is called with four or more arguments ($alias, $base_url, $description,
///         $($alias_and_command),*), it creates an AliasAndCommand object with a NestedCommand inside.
///     NestedCommand is a type of command that holds a map of other AliasAndCommand objects, allowing
///     the user to execute those commands by referencing their aliases. The $base_url argument is used
///     to create the NestedCommand, and the other arguments are used to create the map of
///     AliasAndCommand objects that the NestedCommand contains. The create_alias_to_bookmark_map
///     method is called on the AliasAndCommand object to create the map from the provided arguments.
macro_rules! bl {
    // Create an AliasAndCommand with a TemplatedCommand inside
    ($alias:expr, $url:expr, $command:expr, $description:expr) => {
        AliasAndCommand {
            alias: $alias.to_string(),
            command: Box::new(TemplatedCommand::new($url, $command, $description)),
        }
    };

    // Create an AliasAndCommand with a TemplatedCommand inside, with query encoding disabled
    ($alias:expr, $url:expr, $command:expr, $description:expr, "no_encode") => {
        AliasAndCommand {
            alias: $alias.to_string(),
            command: Box::new(TemplatedCommand::new($url, $command, $description).with_no_query_encode()),
        }
    };

    // Create an AliasAndCommand with a BookmarkCommand inside
    ($alias:expr, $url:expr, $description:expr) => {
        AliasAndCommand {
            alias: $alias.to_string(),
            command: Box::new(BookmarkCommand::new($url, $description)),
        }
    };

    // Create an AliasAndCommand with a NestedCommand inside
    ($alias:expr, $base_url:expr, $description:expr, $($alias_and_command:expr),*) => {
        AliasAndCommand {
            alias: $alias.to_string(),
            command: Box::new(NestedCommand::new(
                $base_url,
                AliasAndCommand::create_alias_to_bookmark_map(vec![$($alias_and_command),*]),
                $description,
            )),
        }
    };
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
            // valid
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

    pub fn get_aliases_from_yml() -> Vec<AliasAndCommand> {
        let yml = include_str!("../commands.yml");
        let settings: Vec<YmlSettings> =
            serde_yaml::from_str(yml).expect("Invalid yaml configuration");
        settings.into_iter().map(AliasAndCommand::from).collect()
    }

    pub fn get_alias_to_bookmark_map() -> HashMap<String, Box<dyn Command>> {
        let mut alias_and_commands = vec![
            bl! {
                "aoc",
                "https://adventofcode.com/2022/",
                "Advent of code",
                Self::advent_of_code_repo("j", "jrodal98/advent-of-code-2022"),
                Self::advent_of_code_repo("l", "gorel/advent-2022"),
                Self::advent_of_code_repo("e", "mozilla2012/adventOfCode")
            },
            Self::pi(),
            Self::localhost(),
        ];
        alias_and_commands.extend(Self::get_aliases_from_yml());
        Self::create_alias_to_bookmark_map(alias_and_commands)
    }

    fn jellyfin(base_url: &str) -> Self {
        bl! {
            "j",
            &format!("{}:8096", base_url),
            "Go to jellyfin"
        }
    }

    fn transmission(base_url: &str) -> Self {
        bl! {
            "t",
            &format!("{}:9091", base_url),
            "Go to transmission"
        }
    }

    fn pi() -> Self {
        let base_url = "http://192.168.0.104";
        bl! {
            "pi",
            base_url,
            "Go to raspberry pi pages",
            Self::jellyfin(base_url),
            Self::transmission(base_url)
        }
    }

    fn hugo(base_url: &str) -> Self {
        bl! {
            "h",
            &format!("{}:1313", base_url),
            "Go to hugo page"
        }
    }

    fn rocket(base_url: &str) -> Self {
        bl! {
            "r",
            &format!("{}:8000", base_url),
            "Go to rocket"
        }
    }

    fn localhost() -> Self {
        let base_url = "http://localhost";
        bl! {
            "lh",
            base_url,
            "Go to raspberry pi pages",
            Self::jellyfin(base_url),
            Self::transmission(base_url),
            Self::hugo(base_url),
            Self::rocket(base_url)
        }
    }

    fn advent_of_code_repo(alias: &'static str, repo: &str) -> Self {
        bl! {
            alias,
            &format!("https://github.com/{}", repo),
            &format!("Go to {}", repo)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_map() {
        // ensure that the map can be constructed
        let _ = AliasAndCommand::get_alias_to_bookmark_map();
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
