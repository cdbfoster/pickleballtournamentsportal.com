#[macro_use]
extern crate rocket;

use std::collections::HashMap;

use regex::Regex;
use reqwest::redirect::Policy;
use reqwest::StatusCode;
use rocket::fs::{relative, FileServer};
use rocket::http::{CookieJar, Status};
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket_dyn_templates::Template;

use self::client::ClientBuilder;
use self::tournaments::{fetch_tournaments, tournament_search, TournamentListing};
use self::util::Cache;

mod client;
mod tournaments;
mod util;

#[get("/")]
fn landing_page() -> Template {
    let context: HashMap<String, String> = HashMap::new();
    Template::render("index", &context)
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
struct UncaptchaRequest {
    url: String,
    challenge_response: String,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "kebab-case")]
struct CaptchaForm {
    g_recaptcha_response: String,
    h_recaptcha_response: String,
}

#[post("/uncaptcha", data = "<request>")]
async fn uncaptcha(request: Json<UncaptchaRequest>, cookies: &CookieJar<'_>) -> (Status, String) {
    let client = ClientBuilder::new(cookies)
        .default_header("Host", "validate.perfdrive.com")
        .default_header("Origin", "https://validate.perfdrive.com")
        .default_header("Cache-Control", "no-cache")
        .redirect_policy(Policy::none())
        .build();

    let response = client.post(&request.url)
        .header("Referer", &request.url)
        .header("Sec-Fetch-Site", "same-origin")
        .form(&CaptchaForm {
            g_recaptcha_response: request.challenge_response.clone(),
            h_recaptcha_response: request.challenge_response.clone(),
        })
        .send()
        .await;

    if let Ok(response) = response {
        if response.status() == StatusCode::FOUND {
            let location_pattern = Regex::new(r"pickleballtournaments\.com").unwrap();
            let location_header = response.headers().get("location").unwrap().to_str().unwrap();

            if location_pattern.is_match(location_header) {
                (Status::Ok, "Unblocked".to_owned())
            } else {
                (Status::Forbidden, format!("Unexpected redirect: {}", location_header))
            }
        } else {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap();
            (Status::Forbidden, format!("perfdrive.com responded with: {}\nBody: {}", status, body))
        }
    } else {
        (Status::InternalServerError, format!("Could not send request: {}", response.unwrap_err()))
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![landing_page])
        .mount("/", routes![fetch_tournaments, tournament_search])
        .mount("/", routes![uncaptcha])
        .mount("/", FileServer::from(relative!("static")))
        .attach(Template::fairing())
        .manage(Cache::<Vec<TournamentListing>>::new())
}
