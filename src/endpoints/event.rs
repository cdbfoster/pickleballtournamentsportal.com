use std::collections::HashMap;

use rocket::serde::json::Json;
use rocket::serde::Serialize;
use rocket::State;
use rocket_dyn_templates::Template;

use crate::client::Client;
use crate::scrape::event::{event_bracket, event_team_list, Bracket, TeamList};
use crate::scrape::tournament_event_group_list::{tournament_event_group_list, EventUrl};
use crate::scrape::tournament_list::{tournament_list, TournamentListing};
use crate::scrape::tournament_schedule::{tournament_schedule, ScheduleItem};
use crate::scrape::{ScrapeCache, ScrapeError, ScrapeResult};

#[get("/tournament/<id>/event/<event>")]
pub fn page(id: usize, event: &str) -> Template {
    let context = std::array::IntoIter::new([
        ("id".to_owned(), id.to_string()),
        ("event".to_owned(), event.to_owned()),
    ])
    .collect::<HashMap<_, _>>();
    Template::render("event", &context)
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub enum EventDataPayload {
    EventData(EventData),
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct EventData {
    name: String,
    schedule_item: Option<ScheduleItem>,
    teams: TeamList,
    bracket: Option<Bracket>,
    tournament: TournamentListing,
}

#[get("/tournament/<id>/event/<event_name>/data")]
pub async fn data(
    id: usize,
    event_name: &str,
    client: Client<'_>,
    cache: &State<ScrapeCache>,
) -> ScrapeResult<Json<EventDataPayload>> {
    let listing = tournament_list(&client, cache)
        .await?
        .iter()
        .find(|t| t.id == id)
        .cloned()
        .ok_or_else(|| ScrapeError::from_str("tournament not found"))?;

    let event = tournament_event_group_list(id, &client, cache)
        .await?
        .iter()
        .flat_map(|g| g.events.iter())
        .find(|e| e.name == event_name)
        .cloned()
        .ok_or_else(|| ScrapeError::from_str("event not found"))?;

    let schedule_item = tournament_schedule(id, &client, cache)
        .await?
        .iter()
        .find(|e| e.event == event.name)
        .cloned();

    let teams = event_team_list(id, &event, &client, cache).await?.clone();

    let bracket = match event.url {
        EventUrl::Bracket(_) => Some(event_bracket(id, &event, &client, cache).await?.clone()),
        _ => None,
    };

    Ok(Json(EventDataPayload::EventData(EventData {
        name: event.name.to_owned(),
        schedule_item,
        teams,
        bracket,
        tournament: listing,
    })))
}
