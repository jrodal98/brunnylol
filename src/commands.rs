use crate::command::{
    command::Command, nested_command::NestedCommand, simple_bookmark::SimpleBookmark,
    templated_command::TemplatedCommand,
};
use std::collections::HashMap;

pub struct AliasAndCommand<'a> {
    alias: &'a str,
    command: Box<dyn Command>,
}

impl<'a> AliasAndCommand<'static> {
    fn create_alias_to_bookmark_map(
        alias_and_commands: Vec<AliasAndCommand<'static>>,
    ) -> HashMap<&'static str, Box<dyn Command>> {
        let mut map = HashMap::new();
        for alias_and_command in alias_and_commands.into_iter() {
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

    fn github() -> Self {
        Self {
            alias: "gh",
            command: Box::new(
                TemplatedCommand::new(
                    "https://github.com/jrodal98",
                    "https://github.com/{}",
                    "Go to jrodal98's github page or go to another repo (e.g. jrodal98/brunnylol)",
                )
                .with_no_query_encode(),
            ),
        }
    }

    fn dictionary() -> Self {
        Self {
            alias: "def",
            command: Box::new(TemplatedCommand::new(
                "https://www.dictionary.com/",
                "https://www.dictionary.com/browse/{}?s=t",
                "Define a word with dictionary.com",
            )),
        }
    }

    fn reddit() -> Self {
        Self {
            alias: "red",
            command: Box::new(TemplatedCommand::new(
                "https://www.reddit.com/",
                "https://www.reddit.com/r/{}",
                "Go to a subreddit",
            )),
        }
    }

    fn urbandictionary() -> Self {
        Self {
            alias: "wut",
            command: Box::new(TemplatedCommand::new(
                "https://www.urbandictionary.com/",
                "https://www.urbandictionary.com/define.php?term={}",
                "Searches for a phrase on urban dictionary",
            )),
        }
    }

    fn genius() -> Self {
        Self {
            alias: "gen",
            command: Box::new(TemplatedCommand::new(
                "https://genius.com/",
                "https://genius.com/search?q={}",
                "Search youtube",
            )),
        }
    }

    fn speed() -> Self {
        Self {
            alias: "speed",
            command: Box::new(SimpleBookmark::new(
                "https://www.speedtest.net/",
                "Run an internet speedtest",
            )),
        }
    }

    fn help() -> Self {
        Self {
            alias: "help",
            command: Box::new(SimpleBookmark::new("/help", "Go to brunnylol's help page")),
        }
    }

    fn jrodal() -> Self {
        Self {
            alias: "jr",
            command: Box::new(SimpleBookmark::new(
                "https://jrodal.com/",
                "Go to jrodal.com",
            )),
        }
    }

    fn amazon() -> Self {
        Self {
                        alias: "am",
                        command: Box::new(TemplatedCommand::new(
            "https://smile.amazon.com/",
            "https://smile.amazon.com/s?k={}&ref=nb_sb_noss_2",
        "Search amazon through smile.amazon (donates .5% of whatever you spend to a charity of your choosing)."
            )),
                    }
    }

    fn leet_x() -> Self {
        Self {
            alias: "1337x",
            command: Box::new(TemplatedCommand::new(
                "https://1337x.to/",
                "https://1337x.to/search/{}/1/",
                "Search 1337x.to",
            )),
        }
    }

    fn facebook() -> Self {
        Self {
            alias: "fb",
            command: Box::new(TemplatedCommand::new(
                "https://www.facebook.com/",
                "https://www.facebook.com/search/top?q={}",
                "Search Facebook",
            )),
        }
    }

    fn instagram() -> Self {
        Self {
            alias: "ig",
            command: Box::new(TemplatedCommand::new(
                "https://www.instagram.com/",
                "https://www.instagram.com/{}/",
                "Search instagram",
            )),
        }
    }

    fn linkedin() -> Self {
        Self {
            alias: "li",
            command: Box::new(TemplatedCommand::new(
            "https://www.linkedin.com/",
            "https://www.linkedin.com/search/results/all/?keywords={}&origin=GLOBAL_SEARCH_HEADER",
                "Search LinkedIn",
            )),
        }
    }

    fn dropbox() -> Self {
        Self {
            alias: "db",
            command: Box::new(SimpleBookmark::new(
                "https://www.dropbox.com/home",
                "Go to dropbox",
            )),
        }
    }

    fn netflix() -> Self {
        Self {
            alias: "nf",
            command: Box::new(TemplatedCommand::new(
                "https://www.netflix.com/",
                "https://www.netflix.com/search?q={}",
                "Search Netflix",
            )),
        }
    }

    fn hulu() -> Self {
        Self {
            alias: "hulu",
            command: Box::new(SimpleBookmark::new("https://www.hulu.com/", "Go to hulu")),
        }
    }

    fn google_image() -> Self {
        Self {
            alias: "img",
            command: Box::new(TemplatedCommand::new(
                "https://images.google.com/",
                "https://images.google.com/images?um=1&hl=en&safe=active&nfpr=1&q={}",
                "Search google images",
            )),
        }
    }

    fn google_calendar() -> Self {
        Self {
            alias: "cal",
            command: Box::new(TemplatedCommand::new(
                "https://calendar.google.com/",
                "https://calendar.google.com/calendar/b/{}/r",
                "Go to google calendar - ALIAS X to go to calendar for google account X.",
            )),
        }
    }

    fn about() -> Self {
        Self {
            alias: "bl",
            command: Box::new(SimpleBookmark::new("/", "Go to brunnylol's home page")),
        }
    }

    fn brunnylol_dev() -> Self {
        Self {
            alias: "dbl",
            command: Box::new(TemplatedCommand::new(
                "http://localhost:8000/",
                "http://localhost:8000/search?q={}",
                "Forward the query to your local version of brunnylol (port 8000)",
            )),
        }
    }

    fn ebay() -> Self {
        Self {
            alias: "eb",
            command: Box::new(TemplatedCommand::new(
            "https://www.ebay.com/",
            "https://www.ebay.com/sch/i.html?_from=R40&_trksid=p2380057.m570.l1313&_nkw={}&_sacat=0",
                        "Search ebay",
            )
        ),
        }
    }

    fn gmail() -> Self {
        Self {
            alias: "gm",
            command: Box::new(TemplatedCommand::new(
                "https://mail.google.com/",
                "https://mail.google.com/mail/u/{}/",
                "Go to gmail - ALIAS X to go to mail for google account X.",
            )),
        }
    }

    fn type_racer() -> Self {
        Self {
            alias: "tr",
            command: Box::new(SimpleBookmark::new(
                "https://play.typeracer.com/",
                "Play typeracer",
            )),
        }
    }

    fn google_drive() -> Self {
        Self {
            alias: "gd",
            command: Box::new(TemplatedCommand::new(
                "https://drive.google.com/",
                "https://drive.google.com/drive/u/{}/my-drive",
                "Go to google drive - ALIAS X to go to drive for google account X.",
            )),
        }
    }

    fn whatsapp() -> Self {
        Self {
            alias: "wap",
            command: Box::new(SimpleBookmark::new(
                "https://web.whatsapp.com/",
                "Go to whatsapp web messenger",
            )),
        }
    }

    fn android_messages() -> Self {
        Self {
            alias: "ame",
            command: Box::new(SimpleBookmark::new(
                "https://messages.google.com/",
                "Go to android messages web client",
            )),
        }
    }

    fn know_your_meme() -> Self {
        Self {
            alias: "meme",
            command: Box::new(TemplatedCommand::new(
                "https://knowyourmeme.com",
                "https://knowyourmeme.com/search?q={}",
                "Search the 'know your meme' database",
            )),
        }
    }

    fn google_maps() -> Self {
        Self {
            alias: "gmap",
            command: Box::new(TemplatedCommand::new(
                "https://www.google.com/maps",
                "https://www.google.com/maps/search/{}",
                "Search Google maps",
            )),
        }
    }

    fn google_photos() -> Self {
        Self {
            alias: "gp",
            command: Box::new(TemplatedCommand::new(
                "https://photos.google.com/",
                "https://photos.google.com/u/{}/",
                "Go to google photos - ALIAS X to go to photos for google account X.",
            )),
        }
    }

    fn minecraft_wiki() -> Self {
        Self {
            alias: "mc",
            command: Box::new(TemplatedCommand::new(
                "https://minecraft.gamepedia.com/",
                "https://minecraft.gamepedia.com/index.php?search={}&title=Special%3ASearch&go=Go",
                "Search minecraft.gamepedia.com",
            )),
        }
    }

    fn stack_overflow() -> Self {
        Self {
            alias: "so",
            command: Box::new(TemplatedCommand::new(
                "https://stackoverflow.com",
                "https://stackoverflow.com/search?q={}",
                "Search questions on stackoverflow",
            )),
        }
    }

    fn jellyfin(base_url: &str) -> Self {
        Self {
            alias: "j",
            command: Box::new(SimpleBookmark::new(
                &format!("{}:8096", base_url),
                "Go to jellyfin",
            )),
        }
    }

    fn transmission(base_url: &str) -> Self {
        Self {
            alias: "t",
            command: Box::new(SimpleBookmark::new(
                &format!("{}:9091", base_url),
                "Go to transmission",
            )),
        }
    }

    fn pi() -> Self {
        let base_url = "http://192.168.0.104";
        let alias_and_commands = vec![Self::jellyfin(base_url), Self::transmission(base_url)];

        Self {
            alias: "pi",
            command: Box::new(NestedCommand::new(
                base_url,
                Self::create_alias_to_bookmark_map(alias_and_commands),
                "Go to raspberry pi pages",
            )),
        }
    }

    fn protonmail() -> Self {
        Self {
            alias: "pm",
            command: Box::new(TemplatedCommand::new(
                "https://beta.protonmail.com",
                "https://beta.protonmail.com/u/{}/",
                "Go to Protonmail - ALIAS X to go to mail for protonmail account X.",
            )),
        }
    }

    fn monkeytype() -> Self {
        Self {
            alias: "mt",
            command: Box::new(SimpleBookmark::new(
                "https://monkeytype.com",
                "Go to monkeytype, a minimalistic typing test",
            )),
        }
    }

    fn hugo(base_url: &str) -> Self {
        Self {
            alias: "h",
            command: Box::new(SimpleBookmark::new(
                &format!("{}:1313", base_url),
                "Go to hugo page",
            )),
        }
    }

    fn rocket(base_url: &str) -> Self {
        Self {
            alias: "r",
            command: Box::new(SimpleBookmark::new(
                &format!("{}:8000", base_url),
                "Go to rocket",
            )),
        }
    }

    fn localhost() -> Self {
        let base_url = "http://localhost";
        let alias_and_commands = vec![
            Self::jellyfin(base_url),
            Self::transmission(base_url),
            Self::hugo(base_url),
            Self::rocket(base_url),
        ];

        Self {
            alias: "lh",
            command: Box::new(NestedCommand::new(
                base_url,
                Self::create_alias_to_bookmark_map(alias_and_commands),
                "Go to raspberry pi pages",
            )),
        }
    }

    fn advent_of_code_repo(alias: &'static str, repo: &str) -> Self {
        Self {
            alias,
            command: Box::new(SimpleBookmark::new(
                &format!("https://github.com/{}", repo),
                &format!("Go to {}", repo),
            )),
        }
    }

    fn advent_of_code() -> Self {
        let alias_and_commands = vec![
            Self::advent_of_code_repo("j", "jrodal98/advent-of-code-2022"),
            Self::advent_of_code_repo("l", "gorel/advent-2022"),
            Self::advent_of_code_repo("e", "mozilla2012/adventOfCode"),
        ];

        Self {
            alias: "aoc",
            command: Box::new(NestedCommand::new(
                "https://adventofcode.com/2022/",
                Self::create_alias_to_bookmark_map(alias_and_commands),
                "Advent of code",
            )),
        }
    }

    pub fn get_alias_to_bookmark_map() -> HashMap<&'static str, Box<dyn Command>> {
        let alias_and_commands = vec![
            Self::google(),
            Self::duckduckgo(),
            Self::youtube(),
            Self::bing(),
            Self::time(),
            Self::wikipedia(),
            Self::archwiki(),
            Self::github(),
            Self::dictionary(),
            Self::reddit(),
            Self::urbandictionary(),
            Self::genius(),
            Self::speed(),
            Self::help(),
            Self::jrodal(),
            Self::amazon(),
            Self::leet_x(),
            Self::facebook(),
            Self::instagram(),
            Self::dropbox(),
            Self::linkedin(),
            Self::netflix(),
            Self::hulu(),
            Self::google_image(),
            Self::google_calendar(),
            Self::about(),
            Self::brunnylol_dev(),
            Self::ebay(),
            Self::gmail(),
            Self::type_racer(),
            Self::google_drive(),
            Self::whatsapp(),
            Self::android_messages(),
            Self::know_your_meme(),
            Self::google_maps(),
            Self::google_photos(),
            Self::minecraft_wiki(),
            Self::stack_overflow(),
            Self::pi(),
            Self::localhost(),
            Self::protonmail(),
            Self::monkeytype(),
            Self::advent_of_code(),
        ];
        Self::create_alias_to_bookmark_map(alias_and_commands)
    }
}

#[test]
fn test_valid_map() {
    // ensure that the map can be constructed
    let _ = AliasAndCommand::get_alias_to_bookmark_map();
}

#[test]
#[should_panic(expected = "Duplicate alias: a")]
fn test_duplicate_map_panics() {
    let aliases_and_commands = vec![
        AliasAndCommand {
            alias: "a",
            command: Box::new(SimpleBookmark::new("www.example.com", "test website")),
        },
        AliasAndCommand {
            alias: "a",
            command: Box::new(SimpleBookmark::new("www.example2.com", "test2 website")),
        },
    ];
    let _ = AliasAndCommand::create_alias_to_bookmark_map(aliases_and_commands);
}
