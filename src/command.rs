extern crate rocket;
use std::collections::HashMap;

use rocket::http::RawStr;

pub trait Command: Send + Sync {
    fn description(&self) -> &str;
    fn get_redirect_url(&self, query: &str) -> String;
}

struct SimpleBookmark {
    bookmark: String,
    description: String,
}

impl Command for SimpleBookmark {
    fn description(&self) -> &str {
        &self.description
    }

    fn get_redirect_url(&self, _query: &str) -> String {
        self.bookmark.clone()
    }
}

struct TemplatedCommand {
    bookmark: String,
    templated_url: String,
    description: String,
}

impl TemplatedCommand {
    fn process_query(&self, query: &str) -> String {
        RawStr::new(query).percent_encode().to_string()
    }
}

impl Command for TemplatedCommand {
    fn description(&self) -> &str {
        &self.description
    }

    fn get_redirect_url(&self, query: &str) -> String {
        match self.process_query(query).as_str() {
            "" => self.bookmark.clone(),
            query => self.templated_url.replace("%s", query),
        }
    }
}

struct NestedCommand<'a> {
    bookmark: String,
    commands: HashMap<&'a str, Box<dyn Command>>,
    description: &'a str,
}

impl<'a> Command for NestedCommand<'a> {
    fn description(&self) -> &str {
        &self.description
    }

    fn get_redirect_url(&self, query: &str) -> String {
        let mut splitted = query.splitn(2, " ");
        if let Some(bookmark_alias) = splitted.next() {
            self.commands
                .get(bookmark_alias)
                .expect(&format!("{} is not a valid command", bookmark_alias))
                .get_redirect_url(splitted.next().unwrap_or_default())
        } else {
            self.bookmark.clone()
        }
    }
}
