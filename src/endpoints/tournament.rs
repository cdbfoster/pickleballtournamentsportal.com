use std::collections::HashMap;

use rocket::serde::json::Json;
use rocket::serde::Serialize;
use rocket::State;
use rocket_dyn_templates::Template;

use crate::client::Client;
use crate::scrape::tournament_event_group_list::{tournament_event_group_list, EventGroupList};
use crate::scrape::tournament_info::{tournament_info, Info};
use crate::scrape::tournament_list::{tournament_list, TournamentListing};
use crate::scrape::tournament_player_list::{tournament_player_list, PlayerList};
use crate::scrape::tournament_schedule::{tournament_schedule, Schedule};
use crate::scrape::{ScrapeCache, ScrapeError, ScrapeResult};

#[get("/tournament/<id>")]
pub fn page(id: usize) -> Template {
    let context =
        std::array::IntoIter::new([("id".to_owned(), id.to_string())]).collect::<HashMap<_, _>>();
    Template::render("tournament", &context)
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub enum TournamentDataPayload {
    TournamentData(TournamentData),
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct TournamentData {
    listing: TournamentListing,
    players: PlayerList,
    event_groups: EventGroupList,
    schedule: Schedule,
    info: Info,
}

#[get("/tournament/<id>/data")]
pub async fn data(
    id: usize,
    client: Client<'_>,
    cache: &State<ScrapeCache>,
) -> ScrapeResult<Json<TournamentDataPayload>> {
    let listing = tournament_list(&client, cache)
        .await?
        .iter()
        .find(|t| t.id == id)
        .map(|l| l.clone())
        .ok_or_else(|| ScrapeError::from_str("tournament not found"))?;

    let player_list = tournament_player_list(id, &client, cache).await?;
    let event_group_list = tournament_event_group_list(id, &client, cache).await?;
    let schedule = tournament_schedule(id, &client, cache).await?;
    let info = tournament_info(id, &client, cache).await?;

    Ok(Json(TournamentDataPayload::TournamentData(
        TournamentData {
            listing,
            players: player_list.clone(),
            event_groups: event_group_list.clone(),
            schedule: schedule.clone(),
            info: info.clone(),
        },
    )))
}
