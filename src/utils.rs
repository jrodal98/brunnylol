use crate::bookmarks;


pub fn alias_to_bookmark(alias: &str) -> Option<Box<dyn bookmarks::Bookmark>> {
    match alias {
        "g" => Some(Box::new(bookmarks::Google)),
        "ddg" => Some(Box::new(bookmarks::DuckDuckGo)),
        "yt" => Some(Box::new(bookmarks::Youtube)),
        "b" => Some(Box::new(bookmarks::Bing)),
        "time" => Some(Box::new(bookmarks::Timeis)),
        "wiki" => Some(Box::new(bookmarks::Wikipedia)),
        "aw" => Some(Box::new(bookmarks::ArchWiki)),
        "gh" => Some(Box::new(bookmarks::Github)),
        "def" => Some(Box::new(bookmarks::Dictionary)),
        "red" => Some(Box::new(bookmarks::Reddit)),
        "wut" => Some(Box::new(bookmarks::UrbanDictionary)),
        "gen" | "lyrics" => Some(Box::new(bookmarks::Genius)),
        "speed" => Some(Box::new(bookmarks::Speed)),
        "help" => Some(Box::new(bookmarks::Help)),
        "jrodal" => Some(Box::new(bookmarks::Jrodal)),
        _ => None,
    }
}
