#[macro_use]
extern crate rocket;

use std::collections::HashMap;

use rocket::fs::{relative, FileServer};
use rocket_dyn_templates::Template;

use self::endpoints::tournaments;
use self::scrape::ScrapeCache;

mod client;
mod endpoints;
mod scrape;
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
        .mount("/", routes![tournaments::fetch, tournaments::search])
        .mount("/", FileServer::from(relative!("static")))
        .attach(Template::fairing())
        .manage(ScrapeCache::default())
}
