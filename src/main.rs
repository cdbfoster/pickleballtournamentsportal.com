#[macro_use]
extern crate rocket;

use std::collections::HashMap;

use rocket::fs::{relative, FileServer};
use rocket_dyn_templates::Template;

use self::tournaments::{fetch_tournaments, tournament_search, TournamentListing};
use self::util::cache::{Cache, PageCache};

mod client;
mod tournaments;
mod util;

#[get("/")]
fn landing_page() -> Template {
    let context: HashMap<String, String> = HashMap::new();
    Template::render("index", &context)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![landing_page])
        .mount("/", routes![fetch_tournaments, tournament_search])
        .mount("/", FileServer::from(relative!("static")))
        .attach(Template::fairing())
        .manage(Cache::<Vec<TournamentListing>>::new())
        .manage(PageCache::new())
}
