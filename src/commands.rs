extern crate maplit;

use crate::command::{command::Command, templated_command::TemplatedCommand};
use maplit::hashmap;
use std::collections::HashMap;

pub struct AliasAndCommand<'a> {
    alias: &'a str,
    command: Box<dyn Command>,
}

impl<'a> AliasAndCommand<'static> {
    pub fn get_alias_to_bookmark_map() -> HashMap<&'static str, Box<dyn Command>> {
        let mut map = HashMap::new();
        for alias_and_command in vec![
            Self::google(),
            Self::duckduckgo(),
            Self::youtube(),
            Self::bing(),
            Self::time(),
            Self::wikipedia(),
            Self::archwiki(),
        ]
        .into_iter()
        {
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
}
//         // "gh" => Box::new(bookmarks::Github),
//         // "def" => Box::new(bookmarks::Dictionary),
//         // "red" => Box::new(bookmarks::Reddit),
//         // "wut" => Box::new(bookmarks::UrbanDictionary),
//         // "gen" => Box::new(bookmarks::Genius),
//         // "speed" => Box::new(bookmarks::Speed),
//         // "help" => Box::new(bookmarks::Help),
//         // "jr" => Box::new(bookmarks::Jrodal),
//         // "am" => Box::new(bookmarks::Amazon),
//         // "1337x" => Box::new(bookmarks::LeetX),
//         // "fb" => Box::new(bookmarks::Facebook),
//         // "ig" => Box::new(bookmarks::Instagram),
//         // "li" => Box::new(bookmarks::LinkedIn),
//         // "db" => Box::new(bookmarks::Dropbox),
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
