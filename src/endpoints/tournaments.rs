//! Handles the scraping/parsing/caching of the tournament list from pickleballtournaments.com

use std::collections::HashMap;

use rocket::serde::json::Json;
use rocket::serde::Serialize;
use rocket::State;
use rocket_dyn_templates::Template;

use crate::client::Client;
use crate::scrape::tournament_list::{tournament_list, TournamentList};
use crate::scrape::{ScrapeCache, ScrapeResult};

#[get("/tournaments")]
pub fn search() -> Template {
    let context: HashMap<String, String> = HashMap::new();
    Template::render("tournaments", &context)
}

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub enum TournamentListPayload {
    Tournaments(TournamentList),
}

#[get("/tournaments/fetch")]
pub async fn data(
    client: Client<'_>,
    cache: &State<ScrapeCache>,
) -> ScrapeResult<Json<TournamentListPayload>> {
    let tournament_list = tournament_list(&client, cache).await?;

    Ok(Json(TournamentListPayload::Tournaments(
        tournament_list.clone(),
    )))
}
