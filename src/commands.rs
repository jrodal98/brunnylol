extern crate maplit;

use crate::command::{
    command::Command, simple_bookmark::SimpleBookmark, templated_command::TemplatedCommand,
};
use std::collections::HashMap;

pub struct AliasAndCommand<'a> {
    alias: &'a str,
    command: Box<dyn Command>,
}

impl<'a> AliasAndCommand<'static> {
    fn create_alias_to_bookmark_map(
        alias_and_commands: Vec<AliasAndCommand<'static>>,
    ) -> HashMap<&'static str, Box<dyn Command>> {
        let mut map = HashMap::new();
        for alias_and_command in alias_and_commands.into_iter() {
            if map
                .insert(alias_and_command.alias, alias_and_command.command)
                .is_some()
            {
                panic!("Duplicate alias: {}", alias_and_command.alias);
            }
        }
        map
    }

    fn google() -> Self {
        Self {
            alias: "g",
            command: Box::new(TemplatedCommand::new(
                "https://www.google.com",
                "https://www.google.com/search?q={}",
                "Search google",
            )),
        }
    }

    fn duckduckgo() -> Self {
        Self {
            alias: "d",
            command: Box::new(TemplatedCommand::new(
                "https://www.duckduckgo.com",
                "https://duckduckgo.com/?q={}",
                "Search duckduckgo",
            )),
        }
    }

    fn youtube() -> Self {
        Self {
            alias: "yt",
            command: Box::new(TemplatedCommand::new(
                "https://www.youtube.com",
                "https://www.youtube.com/results?search_query={}",
                "Search youtube",
            )),
        }
    }

    fn bing() -> Self {
        Self {
            alias: "b",
            command: Box::new(TemplatedCommand::new(
                "https://www.bing.com",
                "https://www.bing.com/search?q={}",
                "Search bing",
            )),
        }
    }

    fn time() -> Self {
        Self {
            alias: "time",
            command: Box::new(TemplatedCommand::new(
                "https://time.is/",
                "https://time.is/{}",
                "Get current time data for a city/country",
            )),
        }
    }

    fn wikipedia() -> Self {
        Self {
            alias: "wiki",
            command: Box::new(TemplatedCommand::new(
                "https://www.wikipedia.org/",
                "https://en.wikipedia.org/wiki/Special:Search/{}",
                "Search wikipedia",
            )),
        }
    }

    fn archwiki() -> Self {
        Self {
            alias: "aw",
            command: Box::new(TemplatedCommand::new(
                "https://wiki.archlinux.org/",
                "https://wiki.archlinux.org/index.php?title=Special%3ASearch&search={}",
                "Search the arch wiki",
            )),
        }
    }

    fn github() -> Self {
        Self {
            alias: "gh",
            command: Box::new(
                TemplatedCommand::new(
                    "https://github.com/jrodal98",
                    "https://github.com/{}",
                    "Go to jrodal98's github page or go to another repo (e.g. jrodal98/brunnylol)",
                )
                .with_no_query_encode(),
            ),
        }
    }

    fn dictionary() -> Self {
        Self {
            alias: "def",
            command: Box::new(TemplatedCommand::new(
                "https://www.dictionary.com/",
                "https://www.dictionary.com/browse/{}?s=t",
                "Define a word with dictionary.com",
            )),
        }
    }

    fn reddit() -> Self {
        Self {
            alias: "red",
            command: Box::new(TemplatedCommand::new(
                "https://www.reddit.com/",
                "https://www.reddit.com/r/{}",
                "Go to a subreddit",
            )),
        }
    }

    fn urbandictionary() -> Self {
        Self {
            alias: "wut",
            command: Box::new(TemplatedCommand::new(
                "https://www.urbandictionary.com/",
                "https://www.urbandictionary.com/define.php?term={}",
                "Searches for a phrase on urban dictionary",
            )),
        }
    }

    fn genius() -> Self {
        Self {
            alias: "gen",
            command: Box::new(TemplatedCommand::new(
                "https://genius.com/",
                "https://genius.com/search?q={}",
                "Search youtube",
            )),
        }
    }

    fn speed() -> Self {
        Self {
            alias: "speed",
            command: Box::new(SimpleBookmark::new(
                "https://www.speedtest.net/",
                "Run an internet speedtest",
            )),
        }
    }

    fn help() -> Self {
        Self {
            alias: "help",
            command: Box::new(SimpleBookmark::new("/help", "Go to brunnylol's help page")),
        }
    }

    fn jrodal() -> Self {
        Self {
            alias: "jr",
            command: Box::new(SimpleBookmark::new(
                "https://jrodal.com/",
                "Go to jrodal.com",
            )),
        }
    }

    fn amazon() -> Self {
        Self {
                        alias: "am",
                        command: Box::new(TemplatedCommand::new(
            "https://smile.amazon.com/",
            "https://smile.amazon.com/s?k={}&ref=nb_sb_noss_2",
        "Search amazon through smile.amazon (donates .5% of whatever you spend to a charity of your choosing)."
            )),
                    }
    }

    fn leet_x() -> Self {
        Self {
            alias: "1337x",
            command: Box::new(TemplatedCommand::new(
                "https://1337x.to/",
                "https://1337x.to/search/{}/1/",
                "Search 1337x.to",
            )),
        }
    }

    fn facebook() -> Self {
        Self {
            alias: "fb",
            command: Box::new(TemplatedCommand::new(
                "https://www.facebook.com/",
                "https://www.facebook.com/search/top?q={}",
                "Search Facebook",
            )),
        }
    }

    fn instagram() -> Self {
        Self {
            alias: "ig",
            command: Box::new(TemplatedCommand::new(
                "https://www.instagram.com/",
                "https://www.instagram.com/{}/",
                "Search instagram",
            )),
        }
    }

    fn linkedin() -> Self {
        Self {
            alias: "li",
            command: Box::new(TemplatedCommand::new(
            "https://www.linkedin.com/",
            "https://www.linkedin.com/search/results/all/?keywords={}&origin=GLOBAL_SEARCH_HEADER",
                "Search LinkedIn",
            )),
        }
    }

    fn dropbox() -> Self {
        Self {
            alias: "db",
            command: Box::new(SimpleBookmark::new(
                "https://www.dropbox.com/home",
                "Go to dropbox",
            )),
        }
    }

    pub fn get_alias_to_bookmark_map() -> HashMap<&'static str, Box<dyn Command>> {
        let alias_and_commands = vec![
            Self::google(),
            Self::duckduckgo(),
            Self::youtube(),
            Self::bing(),
            Self::time(),
            Self::wikipedia(),
            Self::archwiki(),
            Self::github(),
            Self::dictionary(),
            Self::reddit(),
            Self::urbandictionary(),
            Self::genius(),
            Self::speed(),
            Self::help(),
            Self::jrodal(),
            Self::amazon(),
            Self::leet_x(),
            Self::facebook(),
            Self::instagram(),
            Self::dropbox(),
            Self::linkedin(),
        ];
        Self::create_alias_to_bookmark_map(alias_and_commands)
    }
}
//         // "nf" => Box::new(bookmarks::Netflix),
//         // "hulu" => Box::new(bookmarks::Hulu),
//         // "img" => Box::new(bookmarks::GoogleImage),
//         // "cal" => Box::new(bookmarks::GoogleCalendar),
//         // "bl" => Box::new(bookmarks::About),
//         // "~" => Box::new(bookmarks::Home),
//         // "dbl" => Box::new(bookmarks::BrunnylolDev),
//         // "eb" => Box::new(bookmarks::Ebay),
//         // "gm" => Box::new(bookmarks::GoogleMail),
//         // "go" => Box::new(bookmarks::GogoAnime),
//         // "tr" => Box::new(bookmarks::TypeRacer),
//         // "gd" => Box::new(bookmarks::GoogleDrive),
//         // "mega" => Box::new(bookmarks::MegaNz),
//         // "wap" => Box::new(bookmarks::WhatsApp),
//         // "ame" => Box::new(bookmarks::AndroidMessages),
//         // "gme" => Box::new(bookmarks::GroupMe),
//         // "meme" => Box::new(bookmarks::KnowYourMeme),
//         // "gmaps" => Box::new(bookmarks::GoogleMaps),
//         // "gp" => Box::new(bookmarks::GooglePhotos),
//         // "mc" => Box::new(bookmarks::MinecraftWiki),
//         // "so" => Box::new(bookmarks::StackOverflow),
//         // "pi" => Box::new(bookmarks::Pi),
//         // "box" => Box::new(bookmarks::Box),
//         // "pm" => Box::new(bookmarks::ProtonMail),
//         // "mt" => Box::new(bookmarks::MonkeyType),
//         // "lh" => Box::new(bookmarks::LocalHost),
//     }
// }

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
            alias: "a",
            command: Box::new(SimpleBookmark::new("www.example.com", "test website")),
        },
        AliasAndCommand {
            alias: "a",
            command: Box::new(SimpleBookmark::new("www.example2.com", "test2 website")),
        },
    ];
    let _ = AliasAndCommand::create_alias_to_bookmark_map(aliases_and_commands);
}
