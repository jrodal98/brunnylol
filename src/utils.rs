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
        "jrodal" => Box::new(bookmarks::Jrodal),
    }
}
