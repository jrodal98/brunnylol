extern crate maplit;

use crate::bookmarks;
use maplit::hashmap;
use std::collections::HashMap;

pub fn get_alias_to_bookmark_map() -> HashMap<&'static str, Box<dyn bookmarks::Bookmark>> {
    hashmap! {
        "g" => Box::new(bookmarks::Google) as Box<dyn bookmarks::Bookmark>,
        "d" => Box::new(bookmarks::DuckDuckGo),
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
        "li" => Box::new(bookmarks::LinkedIn),
        "db" => Box::new(bookmarks::Dropbox),
        "nf" => Box::new(bookmarks::Netflix),
        "hulu" => Box::new(bookmarks::Hulu),
        "img" => Box::new(bookmarks::GoogleImage),
        "cal" => Box::new(bookmarks::GoogleCalendar),
        "bl" => Box::new(bookmarks::About),
        "~" => Box::new(bookmarks::Home),
        "dbl" => Box::new(bookmarks::BrunnylolDev),
        "eb" => Box::new(bookmarks::Ebay),
        "gm" => Box::new(bookmarks::GoogleMail),
        "go" => Box::new(bookmarks::GogoAnime),
        "tr" => Box::new(bookmarks::TypeRacer),
        "gd" => Box::new(bookmarks::GoogleDrive),
        "mega" => Box::new(bookmarks::MegaNz),
        "wap" => Box::new(bookmarks::WhatsApp),
        "ame" => Box::new(bookmarks::AndroidMessages),
        "gme" => Box::new(bookmarks::GroupMe),
        "meme" => Box::new(bookmarks::KnowYourMeme),
        "gmaps" => Box::new(bookmarks::GoogleMaps),
        "gp" => Box::new(bookmarks::GooglePhotos),
        "mc" => Box::new(bookmarks::MinecraftWiki),
        "so" => Box::new(bookmarks::StackOverflow),
        "pi" => Box::new(bookmarks::Pi),
        "box" => Box::new(bookmarks::Box),
        "pm" => Box::new(bookmarks::ProtonMail),
        "mt" => Box::new(bookmarks::MonkeyType),
        "lh" => Box::new(bookmarks::LocalHost),
        // END OF ALIAS IMPLEMENTATIONS (DO NOT DELETE THIS LINE)
    }
}
