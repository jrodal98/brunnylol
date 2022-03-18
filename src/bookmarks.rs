extern crate rocket;
use rocket::http::RawStr;

pub trait Bookmark: Send + Sync {
    fn urls(&self) -> Vec<String>;
    fn description(&self) -> String;

    fn override_query<'a>(&self, query: &'a str) -> &'a str {
        query
    }

    fn process_query(&self, query: &str) -> String {
        let query = self.override_query(query);
        RawStr::new(query).percent_encode().to_string()
    }

    fn get_redirect_url(&self, query: &str) -> String {
        let query = &self.process_query(query);
        let urls = self.urls();
        if query.is_empty() || urls.len() == 1 {
            urls[0].clone()
        } else {
            urls[1].clone().replace("%s", query)
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

// START OF STRUCT IMPLEMENTATIONS (DO NOT DELETE THIS LINE)

impl Bookmark for ProtonMail {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://beta.protonmail.com".to_string(),
            "https://beta.protonmail.com/u/%s/".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Go to Protonmail - ALIAS X to go to mail for protonmail account X.".to_string()
    }
}

impl Bookmark for Box {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://app.box.com/".to_string(),
            "https://app.box.com/folder/%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Go to box cloud storage - ALIAS X to go to folder X".to_string()
    }
}

impl Bookmark for Pi {
    fn urls(&self) -> Vec<String> {
        vec![
            "http://192.168.0.104/".to_string(),
            "http://192.168.0.104:%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Go to raspberry pi pages".to_string()
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
    fn urls(&self) -> Vec<String> {
        vec![
            "https://stackoverflow.com".to_string(),
            "https://stackoverflow.com/search?q=%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Search questions on stackoverflow".to_string()
    }
}

impl Bookmark for MinecraftWiki {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://minecraft.gamepedia.com/".to_string(),
            "https://minecraft.gamepedia.com/index.php?search=%s&title=Special%3ASearch&go=Go"
                .to_string(),
        ]
    }

    fn description(&self) -> String {
        "Search minecraft.gamepedia.com".to_string()
    }
}

impl Bookmark for GooglePhotos {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://photos.google.com/".to_string(),
            "https://photos.google.com/u/%s/".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Go to google photos - ALIAS X to go to photos for google account X.".to_string()
    }
}

impl Bookmark for GoogleMaps {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://www.google.com/maps".to_string(),
            "https://www.google.com/maps/search/%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Search Google Maps".to_string()
    }
}

impl Bookmark for KnowYourMeme {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://knowyourmeme.com".to_string(),
            "https://knowyourmeme.com/search?q=%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Search the know your meme database".to_string()
    }
}

impl Bookmark for GroupMe {
    fn urls(&self) -> Vec<String> {
        vec!["https://web.groupme.com/chats".to_string()]
    }

    fn description(&self) -> String {
        "Go to groupme".to_string()
    }
}

impl Bookmark for AndroidMessages {
    fn urls(&self) -> Vec<String> {
        vec!["https://messages.google.com/".to_string()]
    }

    fn description(&self) -> String {
        "Goes to android messages web client".to_string()
    }
}

impl Bookmark for WhatsApp {
    fn urls(&self) -> Vec<String> {
        vec!["https://web.whatsapp.com/".to_string()]
    }

    fn description(&self) -> String {
        "Go to whatsapp web messenger".to_string()
    }
}

impl Bookmark for MegaNz {
    fn urls(&self) -> Vec<String> {
        vec!["https://mega.nz".to_string()]
    }

    fn description(&self) -> String {
        "Go to mega.nz".to_string()
    }
}

impl Bookmark for TypeRacer {
    fn urls(&self) -> Vec<String> {
        vec!["https://play.typeracer.com/".to_string()]
    }

    fn description(&self) -> String {
        "Go to typeracer".to_string()
    }
}

impl Bookmark for GoogleDrive {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://drive.google.com/".to_string(),
            "https://drive.google.com/drive/u/%s/my-drive".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Go to google drive - ALIAS X to go to drive for google account X.".to_string()
    }
}

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

    // don't encode the string
    fn process_query(&self, query: &str) -> String {
        query.to_string()
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
        vec!["/help".to_string()]
    }

    fn description(&self) -> String {
        "Go to brunnylol's help page".to_string()
    }
}

impl Bookmark for Jrodal {
    fn urls(&self) -> Vec<String> {
        vec!["https://jrodal.com/".to_string()]
    }

    fn description(&self) -> String {
        "Go to brunnylol's developer's website".to_string()
    }
}

impl Bookmark for Amazon {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://smile.amazon.com/".to_string(),
            "https://smile.amazon.com/s?k=%s&ref=nb_sb_noss_2".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Search amazon through smile.amazon (donates .5% of whatever you spend to a charity of your choosing).".to_string()
    }
}

impl Bookmark for LeetX {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://1337x.to/".to_string(),
            "https://1337x.to/search/%s/1/".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Search 1337x.to".to_string()
    }
}

impl Bookmark for Facebook {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://www.facebook.com/".to_string(),
            "https://www.facebook.com/search/top?q=%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Search Facebook".to_string()
    }
}

impl Bookmark for Instagram {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://www.instagram.com/".to_string(),
            "https://www.instagram.com/%s/".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Search Instagram".to_string()
    }
}

impl Bookmark for LinkedIn {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://www.linkedin.com/".to_string(),
            "https://www.linkedin.com/search/results/all/?keywords=%s&origin=GLOBAL_SEARCH_HEADER"
                .to_string(),
        ]
    }

    fn description(&self) -> String {
        "Go to LinkedIn".to_string()
    }
}

impl Bookmark for Dropbox {
    fn urls(&self) -> Vec<String> {
        vec!["https://www.dropbox.com/home".to_string()]
    }

    fn description(&self) -> String {
        "Go to Dropbox".to_string()
    }
}

impl Bookmark for Netflix {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://www.netflix.com/".to_string(),
            "https://www.netflix.com/search?q=%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Search Netflix".to_string()
    }
}

impl Bookmark for Hulu {
    fn urls(&self) -> Vec<String> {
        vec!["https://www.hulu.com/".to_string()]
    }

    fn description(&self) -> String {
        "Go to hulu".to_string()
    }
}

impl Bookmark for GoogleImage {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://images.google.com/".to_string(),
            "https://images.google.com/images?um=1&hl=en&safe=active&nfpr=1&q=%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Search google images".to_string()
    }
}

impl Bookmark for GoogleCalendar {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://calendar.google.com/".to_string(),
            "https://calendar.google.com/calendar/b/%s/r".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Go to google calendar - ALIAS X to go to calendar for google account X.".to_string()
    }
}

impl Bookmark for About {
    fn urls(&self) -> Vec<String> {
        vec!["/".to_string()]
    }

    fn description(&self) -> String {
        "Go to brunnylol home page".to_string()
    }
}

impl Bookmark for Home {
    fn urls(&self) -> Vec<String> {
        vec!["https://jrodal98.github.io/startpage/".to_string()]
    }

    fn description(&self) -> String {
        "Go to Jacob Rodal's browser start page".to_string()
    }
}

impl Bookmark for BrunnylolDev {
    fn urls(&self) -> Vec<String> {
        vec![
            "http://localhost:8000/".to_string(),
            "http://localhost:8000/search?q=%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Forward the query to your local version of brunnylol".to_string()
    }
}

impl Bookmark for Ebay {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://www.ebay.com/".to_string(),
            "https://www.ebay.com/sch/i.html?_from=R40&_trksid=p2380057.m570.l1313&_nkw=%s&_sacat=0".to_string()
        ]
    }

    fn description(&self) -> String {
        "Search ebay".to_string()
    }
}

impl Bookmark for GoogleMail {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://mail.google.com/".to_string(),
            "https://mail.google.com/mail/u/%s/".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Go to gmail - ALIAS X to go to mail for google account X.".to_string()
    }
}

impl Bookmark for GogoAnime {
    fn urls(&self) -> Vec<String> {
        vec![
            "https://www25.gogoanimes.tv/".to_string(),
            "https://www25.gogoanimes.tv//search.html?keyword=%s".to_string(),
        ]
    }

    fn description(&self) -> String {
        "Search gogoanimes.tv".to_string()
    }
}
