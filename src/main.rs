#[macro_use]
extern crate rocket;
extern crate clap;
mod command;
pub mod commands;
pub mod yml_settings;
use command::Command;
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::Template;
use std::collections::HashMap;

use clap::Arg;

const DEFAULT_ALIAS: &str = "g";

#[get("/help")]
fn help(alias_to_bookmark_map: &State<HashMap<String, Box<dyn Command>>>) -> Template {
    let mut context = HashMap::new();
    let alias_to_description: HashMap<&String, String> = alias_to_bookmark_map
        .iter()
        .map(|(alias, bm)| (alias, bm.description()))
        .collect();
    context.insert("alias_to_description", alias_to_description);
    Template::render("help", context)
}

#[get("/")]
fn index() -> Template {
    let context: HashMap<String, String> = HashMap::new();
    Template::render("index", context)
}

#[get("/search?<q>&<default>")]
fn redirect(
    q: String,
    default: Option<String>,
    alias_to_bookmark_map: &State<HashMap<String, Box<dyn Command>>>,
) -> Redirect {
    let mut splitted = q.splitn(2, " ");
    let bookmark_alias = splitted.next().unwrap();
    let query = splitted.next().unwrap_or_default();

    let redirect_url = match alias_to_bookmark_map.get(bookmark_alias) {
        Some(bookmark) => bookmark.get_redirect_url(query),
        None => alias_to_bookmark_map
            .get(default.as_deref().unwrap_or(DEFAULT_ALIAS))
            .expect("Default search engine alias was not found!")
            .get_redirect_url(&q),
    };

    Redirect::to(redirect_url)
}

#[launch]
fn rocket() -> _ {
    let matches = clap::Command::new("Brunnylol")
        .arg(
            Arg::new("commands")
                .short('c')
                .long("commands")
                .value_name("COMMANDS")
                .help("Path to a YAML file containing commands"),
        )
        .get_matches();
    let yaml_path = matches.get_one("commands").map(|c: &String| c.as_str());
    let alias_to_bookmark_map = commands::AliasAndCommand::get_alias_to_bookmark_map(yaml_path);
    rocket::build()
        .manage(alias_to_bookmark_map)
        .attach(Template::fairing())
        .mount("/", routes![index, help, redirect])
}
