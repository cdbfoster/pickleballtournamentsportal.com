#[macro_use]
extern crate rocket;

use std::collections::HashMap;

use rocket::fs::{relative, FileServer};
use rocket_dyn_templates::Template;

use self::endpoints::{tournament, tournaments};
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

#[catch(404)]
fn not_found() -> Template {
    let context: HashMap<String, String> = HashMap::new();
    Template::render("404", &context)
}

#[get("/not-found")]
fn not_found_page() -> Template {
    not_found()
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![landing_page, not_found_page])
        .mount("/", routes![tournaments::data, tournaments::search])
        .mount("/", routes![tournament::data, tournament::page])
        .mount("/", FileServer::from(relative!("static")))
        .register("/", catchers![not_found])
        .attach(Template::fairing())
        .manage(ScrapeCache::default())
}
