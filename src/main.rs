#[macro_use]
extern crate rocket;

use rocket::fs::{relative, FileServer};
use rocket_dyn_templates::Template;

use self::tournaments::{get_tournaments, TournamentListing};
use self::util::Cache;

mod client;
mod tournaments;
mod util;

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![get_tournaments])
        .mount("/", FileServer::from(relative!("static")))
        .attach(Template::fairing())
        .manage(Cache::<Vec<TournamentListing>>::new())
}
