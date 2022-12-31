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

    pub fn get_alias_to_bookmark_map() -> HashMap<String, Box<dyn Command>> {
        let alias_and_commands = vec![
            bl! {
                "g",
                "https://www.google.com",
                "https://www.google.com/search?q={}",
                "Search google"
            },
            bl! {
                "d",
                "https://www.duckduckgo.com",
                "https://duckduckgo.com/?q={}",
                "Search duckduckgo"
            },
            bl! {
                "yt",
                "https://www.youtube.com",
                "https://www.youtube.com/results?search_query={}",
                "Search youtube"
            },
            bl! {
                "b",
                "https://www.bing.com",
                "https://www.bing.com/search?q={}",
                "Search bing"
            },
            bl! {
                "time",
                "https://time.is/",
                "https://time.is/{}",
                "Get current time data for a city/country"
            },
            bl! {
                "wiki",
                "https://www.wikipedia.org/",
                "https://en.wikipedia.org/wiki/Special:Search/{}",
                "Search wikipedia"
            },
            bl! {
                "aw",
                "https://wiki.archlinux.org/",
                "https://wiki.archlinux.org/index.php?title=Special%3ASearch&search={}",
                "Search the arch wiki"
            },
            bl! {
                "gh",
                "https://github.com/jrodal98",
                "https://github.com/{}",
                "Go to jrodal98's github page or go to another repo (e.g. jrodal98/brunnylol)",
                "no_encode"
            },
            bl! {
                "def",
                "https://www.dictionary.com/",
                "https://www.dictionary.com/browse/{}?s=t",
                "Define a word with dictionary.com"
            },
            bl! {
                "red",
                "https://www.reddit.com/",
                "https://www.reddit.com/r/{}",
                "Go to a subreddit"
            },
            bl! {
                "wut",
                "https://www.urbandictionary.com/",
                "https://www.urbandictionary.com/define.php?term={}",
                "Searches for a phrase on urban dictionary"
            },
            bl! {
                "gen",
                "https://genius.com/",
                "https://genius.com/search?q={}",
                "Search youtube"
            },
            bl! {
                "speed",
                "https://www.speedtest.net/",
                "Run an internet speedtest"
            },
            bl! {
                "help",
                "/help",
                "Go to brunnylol's help page"
            },
            bl! {
                "jr",
                "https://jrodal.com/",
                "Go to jrodal.com"
            },
            bl! {
                "am",
                "https://smile.amazon.com/",
                "https://smile.amazon.com/s?k={}&ref=nb_sb_noss_2",
                "Search amazon through smile.amazon (donates .5% of whatever you spend to a charity of your choosing)."
            },
            bl! {
                "1337x",
                "https://1337x.to/",
                "https://1337x.to/search/{}/1/",
                "Search 1337x.to"
            },
            bl! {
                "fb",
                "https://www.facebook.com/",
                "https://www.facebook.com/search/top?q={}",
                "Search Facebook"
            },
            bl! {
                "ig",
                "https://www.instagram.com/",
                "https://www.instagram.com/{}/",
                "Search instagram"
            },
            bl! {
                "db",
                "https://www.dropbox.com/home",
                "Go to dropbox"
            },
            bl! {
                "li",
                "https://www.linkedin.com/",
                "https://www.linkedin.com/search/results/all/?keywords={}&origin=GLOBAL_SEARCH_HEADER",
                "Search LinkedIn"
            },
            bl! {
                "nf",
                "https://www.netflix.com/",
                "https://www.netflix.com/search?q={}",
                "Search Netflix"
            },
            bl! {
                "hulu",
                "https://www.hulu.com/",
                "Go to hulu"
            },
            bl! {
                "img",
                "https://images.google.com/",
                "https://images.google.com/images?um=1&hl=en&safe=active&nfpr=1&q={}",
                "Search google images"
            },
            bl! {
                "cal",
                "https://calendar.google.com/",
                "https://calendar.google.com/calendar/b/{}/r",
                "Go to google calendar - ALIAS X to go to calendar for google account X."
            },
            bl! {
                "bl",
                "/",
                "Go to brunnylol's home page"
            },
            bl! {
                "dbl",
                "http://localhost:8000/",
                "http://localhost:8000/search?q={}",
                "Forward the query to your local version of brunnylol (port 8000)"
            },
            bl! {
                "eb",
                "https://www.ebay.com/",
                "https://www.ebay.com/sch/i.html?_from=R40&_trksid=p2380057.m570.l1313&_nkw={}&_sacat=0",
                "Search ebay"
            },
            bl! {
                "gm",
                "https://mail.google.com/",
                "https://mail.google.com/mail/u/{}/",
                "Go to gmail - ALIAS X to go to mail for google account X."
            },
            bl! {
                "tr",
                "https://play.typeracer.com/",
                "Play typeracer"
            },
            bl! {
                "gd",
                "https://drive.google.com/",
                "https://drive.google.com/drive/u/{}/my-drive",
                "Go to google drive - ALIAS X to go to drive for google account X."
            },
            bl! {
                "wap",
                "https://web.whatsapp.com/",
                "Go to whatsapp web messenger"
            },
            bl! {
                "ame",
                "https://messages.google.com/",
                "Go to android messages web client"
            },
            bl! {
                "meme",
                "https://knowyourmeme.com",
                "https://knowyourmeme.com/search?q={}",
                "Search the 'know your meme' database"
            },
            bl! {
                "gmap",
                "https://www.google.com/maps",
                "https://www.google.com/maps/search/{}",
                "Search Google maps"
            },
            bl! {
                "gp",
                "https://photos.google.com/",
                "https://photos.google.com/u/{}/",
                "Go to google photos - ALIAS X to go to photos for google account X."
            },
            bl! {
                "mc",
                "https://minecraft.gamepedia.com/",
                "https://minecraft.gamepedia.com/index.php?search={}&title=Special%3ASearch&go=Go",
                "Search minecraft.gamepedia.com"
            },
            bl! {
                "so",
                "https://stackoverflow.com",
                "https://stackoverflow.com/search?q={}",
                "Search questions on stackoverflow"
            },
            bl! {
                "pm",
                "https://beta.protonmail.com",
                "https://beta.protonmail.com/u/{}/",
                "Go to Protonmail - ALIAS X to go to mail for protonmail account X."
            },
            bl! {
                "mt",
                "https://monkeytype.com",
                "Go to monkeytype, a minimalistic typing test"
            },
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
