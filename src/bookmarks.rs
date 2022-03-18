extern crate rocket;
use rocket::http::RawStr;

pub trait Bookmark: Send + Sync {
    fn urls(&self) -> Vec<&'static str>;
    fn description(&self) -> &'static str;

    fn override_query<'a>(&self, query: &'a str) -> &'a str {
        query
    }

    fn encode_query(&self) -> bool {
        true
    }

    fn process_query(&self, query: &str) -> String {
        let query = self.override_query(query);
        if self.encode_query() {
            RawStr::new(query).percent_encode().to_string()
        } else {
            query.to_string()
        }
    }

    fn get_redirect_url(&self, query: &str) -> String {
        let query = &self.process_query(query);
        let urls = self.urls();
        if query.is_empty() || urls.len() == 1 {
            urls[0].to_string()
        } else {
            urls[1].clone().replace("%s", query).to_string()
        }
    }
}

// START OF STRUCT DECLARATIONS

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
pub struct Amazon;
pub struct LeetX;
pub struct Facebook;
pub struct Instagram;
pub struct LinkedIn;
pub struct Dropbox;
pub struct Netflix;
pub struct Hulu;
pub struct GoogleImage;
pub struct GoogleCalendar;
pub struct About;
pub struct Home;
pub struct BrunnylolDev;
pub struct Ebay;
pub struct GoogleMail;
pub struct GogoAnime;
pub struct GoogleDrive;
pub struct TypeRacer;
pub struct MegaNz;
pub struct WhatsApp;
pub struct AndroidMessages;
pub struct GroupMe;
pub struct KnowYourMeme;
pub struct GoogleMaps;
pub struct GooglePhotos;
pub struct MinecraftWiki;
pub struct StackOverflow;
pub struct Pi;
pub struct Box;
pub struct ProtonMail;
pub struct MonkeyType;

// START OF STRUCT IMPLEMENTATIONS (DO NOT DELETE THIS LINE)

impl Bookmark for MonkeyType {
    fn urls(&self) -> Vec<&'static str> {
        vec!["https://monkeytype.com"]
    }

    fn description(&self) -> &'static str {
        "Go to monkeytype, a minimalistic typing test"
    }
}

impl Bookmark for ProtonMail {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://beta.protonmail.com",
            "https://beta.protonmail.com/u/%s/",
        ]
    }

    fn description(&self) -> &'static str {
        "Go to Protonmail - ALIAS X to go to mail for protonmail account X."
    }
}

impl Bookmark for Box {
    fn urls(&self) -> Vec<&'static str> {
        vec!["https://app.box.com/", "https://app.box.com/folder/%s"]
    }

    fn description(&self) -> &'static str {
        "Go to box cloud storage - ALIAS X to go to folder X"
    }
}

impl Bookmark for Pi {
    fn urls(&self) -> Vec<&'static str> {
        vec!["http://192.168.0.104/", "http://192.168.0.104:%s"]
    }

    fn description(&self) -> &'static str {
        "Go to raspberry pi pages"
    }

    fn override_query<'a>(&self, query: &'a str) -> &'a str {
        match query {
            "j" => "8096",
            "t" => "9091",
            "h" => "8384",
            _ => query,
        }
    }
}

impl Bookmark for StackOverflow {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://stackoverflow.com",
            "https://stackoverflow.com/search?q=%s",
        ]
    }

    fn description(&self) -> &'static str {
        "Search questions on stackoverflow"
    }
}

impl Bookmark for MinecraftWiki {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://minecraft.gamepedia.com/",
            "https://minecraft.gamepedia.com/index.php?search=%s&title=Special%3ASearch&go=Go",
        ]
    }

    fn description(&self) -> &'static str {
        "Search minecraft.gamepedia.com"
    }
}

impl Bookmark for GooglePhotos {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://photos.google.com/",
            "https://photos.google.com/u/%s/",
        ]
    }

    fn description(&self) -> &'static str {
        "Go to google photos - ALIAS X to go to photos for google account X."
    }
}

impl Bookmark for GoogleMaps {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://www.google.com/maps",
            "https://www.google.com/maps/search/%s",
        ]
    }

    fn description(&self) -> &'static str {
        "Search Google Maps"
    }
}

impl Bookmark for KnowYourMeme {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://knowyourmeme.com",
            "https://knowyourmeme.com/search?q=%s",
        ]
    }

    fn description(&self) -> &'static str {
        "Search the know your meme database"
    }
}

impl Bookmark for GroupMe {
    fn urls(&self) -> Vec<&'static str> {
        vec!["https://web.groupme.com/chats"]
    }

    fn description(&self) -> &'static str {
        "Go to groupme"
    }
}

impl Bookmark for AndroidMessages {
    fn urls(&self) -> Vec<&'static str> {
        vec!["https://messages.google.com/"]
    }

    fn description(&self) -> &'static str {
        "Goes to android messages web client"
    }
}

impl Bookmark for WhatsApp {
    fn urls(&self) -> Vec<&'static str> {
        vec!["https://web.whatsapp.com/"]
    }

    fn description(&self) -> &'static str {
        "Go to whatsapp web messenger"
    }
}

impl Bookmark for MegaNz {
    fn urls(&self) -> Vec<&'static str> {
        vec!["https://mega.nz"]
    }

    fn description(&self) -> &'static str {
        "Go to mega.nz"
    }
}

impl Bookmark for TypeRacer {
    fn urls(&self) -> Vec<&'static str> {
        vec!["https://play.typeracer.com/"]
    }

    fn description(&self) -> &'static str {
        "Go to typeracer"
    }
}

impl Bookmark for GoogleDrive {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://drive.google.com/",
            "https://drive.google.com/drive/u/%s/my-drive",
        ]
    }

    fn description(&self) -> &'static str {
        "Go to google drive - ALIAS X to go to drive for google account X."
    }
}

impl Bookmark for Google {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://www.google.com",
            "https://www.google.com/search?q=%s",
        ]
    }

    fn description(&self) -> &'static str {
        "Search google"
    }
}

impl Bookmark for DuckDuckGo {
    fn urls(&self) -> Vec<&'static str> {
        vec!["https://www.duckduckgo.com", "https://duckduckgo.com/?q=%s"]
    }

    fn description(&self) -> &'static str {
        "Search duckduckgo"
    }
}

impl Bookmark for Bing {
    fn urls(&self) -> Vec<&'static str> {
        vec!["https://www.bing.com", "https://www.bing.com/search?q=%s"]
    }

    fn description(&self) -> &'static str {
        "Search bing"
    }
}

impl Bookmark for Youtube {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://www.youtube.com",
            "https://www.youtube.com/results?search_query=%s",
        ]
    }

    fn description(&self) -> &'static str {
        "Search youtube"
    }
}

impl Bookmark for Timeis {
    fn urls(&self) -> Vec<&'static str> {
        vec!["https://time.is/", "https://time.is/%s"]
    }

    fn description(&self) -> &'static str {
        "Get current time data for a city/country"
    }
}

impl Bookmark for Wikipedia {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://www.wikipedia.org/",
            "https://en.wikipedia.org/wiki/Special:Search/%s",
        ]
    }

    fn description(&self) -> &'static str {
        "Search wikipedia"
    }
}

impl Bookmark for ArchWiki {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://wiki.archlinux.org/",
            "https://wiki.archlinux.org/index.php?title=Special%3ASearch&search=%s",
        ]
    }

    fn description(&self) -> &'static str {
        "Search the arch wiki"
    }
}

impl Bookmark for Dictionary {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://www.dictionary.com/",
            "https://www.dictionary.com/browse/%s?s=t",
        ]
    }

    fn description(&self) -> &'static str {
        "Define a word with dictionary.com"
    }
}

impl Bookmark for Reddit {
    fn urls(&self) -> Vec<&'static str> {
        vec!["https://www.reddit.com/", "https://www.reddit.com/r/%s"]
    }

    fn description(&self) -> &'static str {
        "Go to a subreddit"
    }
}

impl Bookmark for UrbanDictionary {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://www.urbandictionary.com/",
            "https://www.urbandictionary.com/define.php?term=%s",
        ]
    }

    fn description(&self) -> &'static str {
        "Searches for a phrase on urban dictionary"
    }
}

impl Bookmark for Genius {
    fn urls(&self) -> Vec<&'static str> {
        vec!["https://genius.com/", "https://genius.com/search?q=%s"]
    }

    fn description(&self) -> &'static str {
        "Search for a song with genius"
    }
}

impl Bookmark for Github {
    fn urls(&self) -> Vec<&'static str> {
        vec!["https://github.com/jrodal98", "https://github.com/%s"]
    }

    fn description(&self) -> &'static str {
        "Go to brunnylol's developer's github or go to another repo (e.g. jrodal98/brunnylol)"
    }

    fn encode_query(&self) -> bool {
        false
    }
}

impl Bookmark for Speed {
    fn urls(&self) -> Vec<&'static str> {
        vec!["https://www.speedtest.net/"]
    }

    fn description(&self) -> &'static str {
        "Run an internet speedtest"
    }
}

impl Bookmark for Help {
    fn urls(&self) -> Vec<&'static str> {
        vec!["/help"]
    }

    fn description(&self) -> &'static str {
        "Go to brunnylol's help page"
    }
}

impl Bookmark for Jrodal {
    fn urls(&self) -> Vec<&'static str> {
        vec!["https://jrodal.com/"]
    }

    fn description(&self) -> &'static str {
        "Go to brunnylol's developer's website"
    }
}

impl Bookmark for Amazon {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://smile.amazon.com/",
            "https://smile.amazon.com/s?k=%s&ref=nb_sb_noss_2",
        ]
    }

    fn description(&self) -> &'static str {
        "Search amazon through smile.amazon (donates .5% of whatever you spend to a charity of your choosing)."
    }
}

impl Bookmark for LeetX {
    fn urls(&self) -> Vec<&'static str> {
        vec!["https://1337x.to/", "https://1337x.to/search/%s/1/"]
    }

    fn description(&self) -> &'static str {
        "Search 1337x.to"
    }
}

impl Bookmark for Facebook {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://www.facebook.com/",
            "https://www.facebook.com/search/top?q=%s",
        ]
    }

    fn description(&self) -> &'static str {
        "Search Facebook"
    }
}

impl Bookmark for Instagram {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://www.instagram.com/",
            "https://www.instagram.com/%s/",
        ]
    }

    fn description(&self) -> &'static str {
        "Search Instagram"
    }
}

impl Bookmark for LinkedIn {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://www.linkedin.com/",
            "https://www.linkedin.com/search/results/all/?keywords=%s&origin=GLOBAL_SEARCH_HEADER",
        ]
    }

    fn description(&self) -> &'static str {
        "Go to LinkedIn"
    }
}

impl Bookmark for Dropbox {
    fn urls(&self) -> Vec<&'static str> {
        vec!["https://www.dropbox.com/home"]
    }

    fn description(&self) -> &'static str {
        "Go to Dropbox"
    }
}

impl Bookmark for Netflix {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://www.netflix.com/",
            "https://www.netflix.com/search?q=%s",
        ]
    }

    fn description(&self) -> &'static str {
        "Search Netflix"
    }
}

impl Bookmark for Hulu {
    fn urls(&self) -> Vec<&'static str> {
        vec!["https://www.hulu.com/"]
    }

    fn description(&self) -> &'static str {
        "Go to hulu"
    }
}

impl Bookmark for GoogleImage {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://images.google.com/",
            "https://images.google.com/images?um=1&hl=en&safe=active&nfpr=1&q=%s",
        ]
    }

    fn description(&self) -> &'static str {
        "Search google images"
    }
}

impl Bookmark for GoogleCalendar {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://calendar.google.com/",
            "https://calendar.google.com/calendar/b/%s/r",
        ]
    }

    fn description(&self) -> &'static str {
        "Go to google calendar - ALIAS X to go to calendar for google account X."
    }
}

impl Bookmark for About {
    fn urls(&self) -> Vec<&'static str> {
        vec!["/"]
    }

    fn description(&self) -> &'static str {
        "Go to brunnylol home page"
    }
}

impl Bookmark for Home {
    fn urls(&self) -> Vec<&'static str> {
        vec!["https://jrodal98.github.io/startpage/"]
    }

    fn description(&self) -> &'static str {
        "Go to Jacob Rodal's browser start page"
    }
}

impl Bookmark for BrunnylolDev {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "http://localhost:8000/",
            "http://localhost:8000/search?q=%s",
        ]
    }

    fn description(&self) -> &'static str {
        "Forward the query to your local version of brunnylol"
    }
}

impl Bookmark for Ebay {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://www.ebay.com/",
            "https://www.ebay.com/sch/i.html?_from=R40&_trksid=p2380057.m570.l1313&_nkw=%s&_sacat=0"
        ]
    }

    fn description(&self) -> &'static str {
        "Search ebay"
    }
}

impl Bookmark for GoogleMail {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://mail.google.com/",
            "https://mail.google.com/mail/u/%s/",
        ]
    }

    fn description(&self) -> &'static str {
        "Go to gmail - ALIAS X to go to mail for google account X."
    }
}

impl Bookmark for GogoAnime {
    fn urls(&self) -> Vec<&'static str> {
        vec![
            "https://www25.gogoanimes.tv/",
            "https://www25.gogoanimes.tv//search.html?keyword=%s",
        ]
    }

    fn description(&self) -> &'static str {
        "Search gogoanimes.tv"
    }
}
