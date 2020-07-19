pub trait Bookmark {
    fn new(urls: Vec<String>, description: String) -> Self;
    fn urls(&self) -> &Vec<String>;
    fn description(&self) -> &String;

    fn get_redirect_url(&self, query: &str) -> String {
        if query.is_empty() {
            self.urls()[0].clone()
        } else {
            self.urls()[1].clone().replace("%s", query)
        }
    }
}

pub struct SimpleBookmark {
    urls: Vec<String>,
    description: String,
}

impl Bookmark for SimpleBookmark {
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

pub fn alias_to_bookmark(alias: &str) -> Option<impl Bookmark> {
    match alias {
        "g" => Some(SimpleBookmark::new(
            vec![
                "https://www.google.com".to_string(),
                "https://www.google.com/search?q=%s".to_string(),
            ],
            "Search google".to_string(),
        )),
        "d" => Some(SimpleBookmark::new(
            vec![
                "https://www.duckduckgo.com".to_string(),
                "https://duckduckgo.com/?q=%s".to_string(),
            ],
            "Search duckduckgo".to_string(),
        )),
        "b" => Some(SimpleBookmark::new(
            vec![
                "https://www.bing.com".to_string(),
                "https://www.bing.com/search?q=%s".to_string(),
            ],
            "Search bing".to_string(),
        )),
        "yt" => Some(SimpleBookmark::new(
            vec![
                "https://www.youtube.com".to_string(),
                "https://www.youtube.com/results?search_query=%s".to_string(),
            ],
            "Search youtube".to_string(),
        )),
        "time" => Some(SimpleBookmark::new(
            vec!["https://time.is/".to_string()],
            "Get current time data for various timezones".to_string(),
        )),
        "wp" => Some(SimpleBookmark::new(
            vec![
                "https://www.wikipedia.org/".to_string(),
                "https://en.wikipedia.org/wiki/Special:Search/%s".to_string(),
            ],
            "Search wikipedia".to_string(),
        )),
        "aw" => Some(SimpleBookmark::new(
            vec![
                "https://wiki.archlinux.org/".to_string(),
                "https://wiki.archlinux.org/index.php?title=Special%3ASearch&search=%s".to_string(),
            ],
            "Search the arch wiki".to_string(),
        )),
        "gh" => Some(SimpleBookmark::new(
            vec![
                "https://github.com/".to_string(),
                "https://github.com/search?q=%s".to_string(),
            ],
            "Search github".to_string(),
        )),
        "def" => Some(SimpleBookmark::new(
            vec![
                "https://www.dictionary.com/".to_string(),
                "https://www.dictionary.com/browse/%s?s=t".to_string(),
            ],
            "Define a word with dictionary.com".to_string(),
        )),
        _ => None,
    }
}
