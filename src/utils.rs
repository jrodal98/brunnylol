extern crate maplit;

use crate::bookmarks;
use maplit::hashmap;
use std::collections::HashMap;

pub fn get_alias_to_bookmark_map() -> HashMap<&'static str, Box<dyn bookmarks::Bookmark>> {
    hashmap! {
        "g" => Box::new(bookmarks::Google) as Box<dyn bookmarks::Bookmark>,
        "ddg" => Box::new(bookmarks::DuckDuckGo),
        "yt" => Box::new(bookmarks::Youtube),
        "b" => Box::new(bookmarks::Bing),
        "time" => Box::new(bookmarks::Timeis),
        "wiki" => Box::new(bookmarks::Wikipedia),
        "aw" => Box::new(bookmarks::ArchWiki),
        "gh" => Box::new(bookmarks::Github),
        "def" => Box::new(bookmarks::Dictionary),
        "red" => Box::new(bookmarks::Reddit),
        "wut" => Box::new(bookmarks::UrbanDictionary),
        "gen" => Box::new(bookmarks::Genius),
        "speed" => Box::new(bookmarks::Speed),
        "help" => Box::new(bookmarks::Help),
        "jr" => Box::new(bookmarks::Jrodal),
        "am" => Box::new(bookmarks::Amazon),
        "1337x" => Box::new(bookmarks::LeetX),
        "fb" => Box::new(bookmarks::Facebook),
        "ig" => Box::new(bookmarks::Instagram),
        "co" => Box::new(bookmarks::UVACollab),
        "sis" => Box::new(bookmarks::UVASis),
        "li" => Box::new(bookmarks::LinkedIn),
        "db" => Box::new(bookmarks::Dropbox),
        "net" => Box::new(bookmarks::Netflix),
        "hulu" => Box::new(bookmarks::Hulu),
        "im" => Box::new(bookmarks::GoogleImage),
        "cal" => Box::new(bookmarks::GoogleCalendar),
        "about" => Box::new(bookmarks::About),
        "home" => Box::new(bookmarks::Home),
        "dbl" => Box::new(bookmarks::BrunnylolDev),
        "eb" => Box::new(bookmarks::Ebay),
        "bb" => Box::new(bookmarks::UVACommBlackboard),
        "gm" => Box::new(bookmarks::GoogleMail),
        "go" => Box::new(bookmarks::GogoAnime),
        "hs" => Box::new(bookmarks::Handshake),
        "pz" => Box::new(bookmarks::Piazza),
        "cw" => Box::new(bookmarks::Campuswire),
        "gs" => Box::new(bookmarks::Gradescope),
        "vm" => Box::new(bookmarks::Vimeo),
        "pi" => Box::new(bookmarks::Pi4),
        "tr" => Box::new(bookmarks::TypeRacer),
        "gd" => Box::new(bookmarks::GoogleDrive),
        "mega" => Box::new(bookmarks::MegaNz),
    }
}
