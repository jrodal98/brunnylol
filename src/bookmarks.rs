extern crate rocket;
use rocket::http::uri::Uri;

pub trait Bookmark {
    fn new(urls: Vec<String>, description: String) -> Self
    where
        Self: Sized;
    fn urls(&self) -> &Vec<String>;
    fn description(&self) -> &String;

    fn get_redirect_url(&self, query: &str) -> String {
        if query.is_empty() || self.urls().len() == 1 {
            self.urls()[0].clone()
        } else {
            self.urls()[1]
                .clone()
                .replace("%s", &Uri::percent_encode(query))
        }
    }
}

pub struct GenericBookmark {
    urls: Vec<String>,
    description: String,
}

impl Bookmark for GenericBookmark {
    fn new(urls: Vec<String>, description: String) -> Self {
        Self { urls, description }
    }

    fn urls(&self) -> &Vec<String> {
        &self.urls
    }

    fn description(&self) -> &String {
        &self.description
    }
}

// doesnt' encode the query string
pub struct UnencodedBookmark {
    urls: Vec<String>,
    description: String,
}

impl Bookmark for UnencodedBookmark {
    fn new(urls: Vec<String>, description: String) -> Self {
        Self { urls, description }
    }

    fn urls(&self) -> &Vec<String> {
        &self.urls
    }

    fn description(&self) -> &String {
        &self.description
    }

    fn get_redirect_url(&self, query: &str) -> String {
        if query.is_empty() || self.urls().len() == 1 {
            self.urls()[0].clone()
        } else {
            self.urls()[1].clone().replace("%s", query)
        }
    }
}

pub fn alias_to_bookmark(alias: &str) -> Option<Box<dyn Bookmark>> {
    match alias {
        "g" => Some(Box::new(GenericBookmark::new(
            vec![
                "https://www.google.com".to_string(),
                "https://www.google.com/search?q=%s".to_string(),
            ],
            "Search google".to_string(),
        ))),
        "d" | "ddg" => Some(Box::new(GenericBookmark::new(
            vec![
                "https://www.duckduckgo.com".to_string(),
                "https://duckduckgo.com/?q=%s".to_string(),
            ],
            "Search duckduckgo".to_string(),
        ))),
        "b" => Some(Box::new(GenericBookmark::new(
            vec![
                "https://www.bing.com".to_string(),
                "https://www.bing.com/search?q=%s".to_string(),
            ],
            "Search bing".to_string(),
        ))),
        "yt" => Some(Box::new(GenericBookmark::new(
            vec![
                "https://www.youtube.com".to_string(),
                "https://www.youtube.com/results?search_query=%s".to_string(),
            ],
            "Search youtube".to_string(),
        ))),
        "time" => Some(Box::new(GenericBookmark::new(
            vec![
                "https://time.is/".to_string(),
                "https://time.is/%s".to_string(),
            ],
            "Get current time data for a city/country".to_string(),
        ))),
        "wiki" => Some(Box::new(GenericBookmark::new(
            vec![
                "https://www.wikipedia.org/".to_string(),
                "https://en.wikipedia.org/wiki/Special:Search/%s".to_string(),
            ],
            "Search wikipedia".to_string(),
        ))),
        "aw" => Some(Box::new(GenericBookmark::new(
            vec![
                "https://wiki.archlinux.org/".to_string(),
                "https://wiki.archlinux.org/index.php?title=Special%3ASearch&search=%s".to_string(),
            ],
            "Search the arch wiki".to_string(),
        ))),
        "gh" => Some(Box::new(UnencodedBookmark::new(
            vec![
                "https://github.com/jrodal98".to_string(),
                "https://github.com/%s".to_string(),
            ],
            "Go to brunnylol's developer's github or go to another repo (e.g. jrodal98/brunnylol)"
                .to_string(),
        ))),
        "def" => Some(Box::new(GenericBookmark::new(
            vec![
                "https://www.dictionary.com/".to_string(),
                "https://www.dictionary.com/browse/%s?s=t".to_string(),
            ],
            "Define a word with dictionary.com".to_string(),
        ))),
        "genius" => Some(Box::new(GenericBookmark::new(
            vec![
                "https://genius.com/".to_string(),
                "https://genius.com/search?q=%s".to_string(),
            ],
            "Search for a song with genius".to_string(),
        ))),
        "reddit" => Some(Box::new(GenericBookmark::new(
            vec![
                "https://www.reddit.com/".to_string(),
                "https://www.reddit.com/r/%s".to_string(),
            ],
            "Go to a subreddit".to_string(),
        ))),
        "speed" => Some(Box::new(GenericBookmark::new(
            vec!["https://www.speedtest.net/".to_string()],
            "Run an internet speedtest".to_string(),
        ))),
        "help" => Some(Box::new(GenericBookmark::new(
            vec!["https://www.brunnylol.xyz/".to_string()],
            "Go to brunnylol homepage".to_string(),
        ))),
        "jrodal" => Some(Box::new(GenericBookmark::new(
            vec!["https://jrodal.dev/".to_string()],
            "Go to brunnylol's developer's website".to_string(),
        ))),
        _ => None,
    }
}
