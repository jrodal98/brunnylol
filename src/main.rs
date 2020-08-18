#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
mod bookmarks;
mod utils;
use crate::utils::get_alias_to_bookmark_map;
use rocket::response::Redirect;
use rocket::State;
use std::collections::HashMap;

const DEFAULT_ALIAS: &str = "g";

#[get("/")]
fn index() -> &'static str {
    "See https://github.com/jrodal98/brunnylol for commands."
}

#[get("/search?<q>")]
fn redirect(
    q: String,
    alias_to_bookmark_map: State<HashMap<&'static str, Box<dyn bookmarks::Bookmark>>>,
) -> Redirect {
    let mut splitted = q.splitn(2, " ");
    let bookmark_alias = splitted.next().unwrap();
    let query = splitted.next().unwrap_or_default();

    let redirect_url = match alias_to_bookmark_map.get(bookmark_alias) {
        Some(bookmark) => bookmark.get_redirect_url(query),
        None => alias_to_bookmark_map
            .get(bookmark_alias)
            .expect("Default search engine alias was not found!")
            .get_redirect_url(&q),
    };

    Redirect::to(redirect_url)
}

fn main() {
    let alias_to_bookmark_map = get_alias_to_bookmark_map();
    rocket::ignite()
        .manage(alias_to_bookmark_map)
        .mount("/", routes![index, redirect])
        .launch();
}
