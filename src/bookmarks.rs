extern crate rocket;
use rocket::http::uri::Uri;

pub trait Bookmark: Send + Sync {
    fn urls(&self) -> Vec<String>;
    fn description(&self) -> String;

    fn get_redirect_url(&self, query: &str) -> String {
        let urls = self.urls();
        if query.is_empty() || urls.len() == 1 {
            urls[0].clone()
        } else {
            urls[1].clone().replace("%s", &Uri::percent_encode(query))
        }
    }
}

pub struct Google;
pub struct DuckDuckGo;
pub struct Bing;
pub struct Youtube;
pub struct Github;
pub struct Dictionary;
pub struct Timeis;
pub struct Wikipedia;
pub struct ArchWiki;
pub struct Reddit;
pub struct UrbanDictionary;
pub struct Genius;
pub struct Speed;
pub struct Help;
pub struct Jrodal;

impl Bookmark for Google {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://www.google.com".to_string(),
            "https://www.google.com/search?q=%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Search google".to_string()
    }
}

impl Bookmark for DuckDuckGo {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://www.duckduckgo.com".to_string(),
            "https://duckduckgo.com/?q=%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Search duckduckgo".to_string()
    }
}

impl Bookmark for Bing {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://www.bing.com".to_string(),
            "https://www.bing.com/search?q=%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Search bing".to_string()
    }
}

impl Bookmark for Youtube {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://www.youtube.com".to_string(),
            "https://www.youtube.com/results?search_query=%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Search youtube".to_string()
    }
}

impl Bookmark for Timeis {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://time.is/".to_string(),
            "https://time.is/%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Get current time data for a city/country".to_string()
    }
}

impl Bookmark for Wikipedia {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://www.wikipedia.org/".to_string(),
            "https://en.wikipedia.org/wiki/Special:Search/%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Search wikipedia".to_string()
    }
}

impl Bookmark for ArchWiki {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://wiki.archlinux.org/".to_string(),
            "https://wiki.archlinux.org/index.php?title=Special%3ASearch&search=%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Search the arch wiki".to_string()
    }
}

impl Bookmark for Dictionary {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://www.dictionary.com/".to_string(),
            "https://www.dictionary.com/browse/%s?s=t".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Define a word with dictionary.com".to_string()
    }
}

impl Bookmark for Reddit {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://www.reddit.com/".to_string(),
            "https://www.reddit.com/r/%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Go to a subreddit".to_string()
    }
}

impl Bookmark for UrbanDictionary {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://www.urbandictionary.com/".to_string(),
            "https://www.urbandictionary.com/define.php?term=%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Searches for a phrase on urban dictionary".to_string()
    }
}

impl Bookmark for Genius {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://genius.com/".to_string(),
            "https://genius.com/search?q=%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Search for a song with genius".to_string()
    }
}

impl Bookmark for Github {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://github.com/jrodal98".to_string(),
            "https://github.com/%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Go to brunnylol's developer's github or go to another repo (e.g. jrodal98/brunnylol)"
            .to_string()
    }

    // don't encode the query
    fn get_redirect_url(&self, query: &str) -> String {
        if query.is_empty() || self.urls().len() == 1 {
            self.urls()[0].clone()
        } else {
            self.urls()[1].clone().replace("%s", query)
        }
    }
}

impl Bookmark for Speed {
    fn urls(&self) -> Vec<String> {
        vec!["https://www.speedtest.net/".to_string()]
    }

    fn description(&self) -> String {
        "Run an internet speedtest".to_string()
    }
}

impl Bookmark for Help {
    fn urls(&self) -> Vec<String> {
        vec!["/".to_string()]
    }

    fn description(&self) -> String {
        "Go to brunnylol homepage".to_string()
    }
}

impl Bookmark for Jrodal {
    fn urls(&self) -> Vec<String> {
        vec!["https://jrodal.dev/".to_string()]
    }

    fn description(&self) -> String {
        "Go to brunnylol's developer's website".to_string()
    }
}

