//! Handles the scraping/parsing/caching of the tournament list from pickleballtournaments.com

use std::collections::HashMap;
use std::time::Duration;

use chrono::prelude::*;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::{Error, Response};
use rocket::response::Responder;
use rocket::serde::json::Json;
use rocket::serde::Serialize;
use rocket::State;
use rocket_dyn_templates::Template;
use scraper::{ElementRef, Html, Selector};

use crate::client::Client;
use crate::util::Cache;

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct TournamentListing {
    id: usize,
    name: String,
    location: String,
    start_date: String,
    end_date: String,
    tag_urls: Vec<String>,
    logo_url: Option<String>,
    registration_status: RegistrationStatus,
}

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub enum RegistrationStatus {
    NotOpen,
    Closed,
    #[serde(rename_all = "camelCase")]
    OpenSoon {
        start_date: String,
        start_time: String,
    },
    #[serde(rename_all = "camelCase")]
    Open {
        deadline: String,
    },
    #[serde(rename_all = "camelCase")]
    ClosedToNew {
        payment_deadline: String,
    },
}

#[get("/tournaments")]
pub fn tournament_search() -> Template {
    let context: HashMap<String, String> = HashMap::new();
    Template::render("tournaments", &context)
}

// The following are single-variant enums so they serialize nicely as {"<tournaments|captcha|error>": value}.

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub enum TournamentListPayload {
    Tournaments(Vec<TournamentListing>),
}

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub enum CaptchaPayload {
    Captcha {
        url: String,
        script: String,
        sitekey: String,
    },
}

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub enum ErrorPayload {
    Error { reason: String },
}

#[derive(Debug, Responder)]
pub enum ScrapeError {
    #[response(status = 200, content_type = "json")]
    Captcha(Json<CaptchaPayload>),
    #[response(status = 500)]
    Error(Json<ErrorPayload>),
}

pub type ScrapeResult<T> = Result<T, ScrapeError>;

async fn map_response(response: Result<Response, Error>, error: &str) -> ScrapeResult<Response> {
    match response {
        Ok(r) => {
            let captcha_url = Regex::new(r"validate\.perfdrive\.com").unwrap();
            if captcha_url.is_match(r.url().as_str()) {
                let url = r.url().to_string();

                let html_raw = r.text().await.unwrap();
                let html = Html::parse_document(&html_raw);

                let script_selector = Selector::parse("script").unwrap();
                let script = html.select(&script_selector).next().unwrap().html();

                let captcha_selector = Selector::parse(".h-captcha").unwrap();
                let captcha_html = html.select(&captcha_selector).next().unwrap().html();

                let sitekey_pattern = Regex::new(r#"sitekey="([^"]+)""#).unwrap();
                let sitekey = sitekey_pattern.captures(&captcha_html).unwrap()[1].to_owned();

                Err(ScrapeError::Captcha(Json(CaptchaPayload::Captcha {
                    url,
                    script,
                    sitekey,
                })))
            } else {
                Ok(r)
            }
        }
        Err(e) => Err(ScrapeError::Error(Json(ErrorPayload::Error {
            reason: if let Some(status) = e.status() {
                format!("{}:\n  status: {}\n  error: {}", error, status.as_u16(), e)
            } else {
                format!("{}:\n  error: {}", error, e)
            },
        }))),
    }
}

#[get("/tournaments/fetch")]
pub async fn fetch_tournaments(
    client: Client<'_>,
    tournament_listings_cache: &State<Cache<Vec<TournamentListing>>>,
) -> ScrapeResult<Json<TournamentListPayload>> {
    let tournament_listings = tournament_listings_cache
        .retrieve_or_update(Duration::from_secs(60 * 60), || async {
            let response = map_response(
                client
                    .get("https://www.pickleballtournaments.com/pbt_tlisting.pl?when=F")
                    .send()
                    .await,
                "could not load future tournaments",
            )
            .await?;

            let future_raw_html = response.text().await.unwrap();

            let response = map_response(
                client
                    .get("https://www.pickleballtournaments.com/pbt_tlisting.pl?when=P")
                    .header(
                        "Referer",
                        "https://www.pickleballtournaments.com/pbt_tlisting.pl?when=F",
                    )
                    .header("Sec-Fetch-Site", "same-origin")
                    .send()
                    .await,
                "could not load past tournaments",
            )
            .await?;

            let past_raw_html = response.text().await.unwrap();

            let future_document = Html::parse_document(&future_raw_html);
            let past_document = Html::parse_document(&past_raw_html);

            let tournament_listings = future_document
                .select(&SELECTORS.tournament)
                .chain(past_document.select(&SELECTORS.tournament))
                .map(parse_tournament_listing)
                .collect::<Vec<_>>();

            Ok(tournament_listings)
        })
        .await?
        .unwrap();

    Ok(Json(TournamentListPayload::Tournaments(
        tournament_listings.clone(),
    )))
}

struct Selectors {
    tournament: Selector,
    title: Selector,
    location: Selector,
    date: Selector,
    tag: Selector,
    logo: Selector,
    registration: Selector,
    is_adonly: Selector,
    soon: Selector,
}

static SELECTORS: Lazy<Selectors> = Lazy::new(|| Selectors {
    tournament: Selector::parse(".tourneylist > .row").unwrap(),
    title: Selector::parse("h3 > a").unwrap(),
    location: Selector::parse(".infocenter > p").unwrap(),
    date: Selector::parse(".tourney-date").unwrap(),
    tag: Selector::parse(".logos > span > p:not(.tourney-date)").unwrap(),
    logo: Selector::parse(".tagscenter").unwrap(),
    registration: Selector::parse(".registration").unwrap(),
    is_adonly: Selector::parse(".adonly").unwrap(),
    soon: Selector::parse(".soon-date").unwrap(),
});

struct Patterns {
    id: Regex,
    date: Regex,
    img_url: Regex,
    registration: Regex,
    soon_time: Regex,
}

static PATTERNS: Lazy<Patterns> = Lazy::new(|| Patterns {
    id: Regex::new(r"\?tid=(\d+)").unwrap(),
    date: Regex::new(r"(\d{1, 2})/(\d{1, 2})/(\d{2})").unwrap(),
    img_url: Regex::new(r#"src="([^"]+)""#).unwrap(),
    registration: Regex::new(r#"registration ([^ "]+)"#).unwrap(),
    soon_time: Regex::new(r"\d{1, 2}/\d{1, 2}/\d{2} (.+)").unwrap(),
});

fn parse_tournament_listing(tournament_element: ElementRef) -> TournamentListing {
    let title_element = tournament_element.select(&SELECTORS.title).next().unwrap();
    let title_element_html = title_element.html();

    let id = PATTERNS.id.captures(&title_element_html).unwrap()[1]
        .parse::<usize>()
        .unwrap();
    let name = title_element.inner_html();

    let location = tournament_element
        .select(&SELECTORS.location)
        .next()
        .unwrap()
        .inner_html();

    let date_element = tournament_element.select(&SELECTORS.date).next().unwrap();
    let date_element_inner_html = date_element.inner_html();
    let mut dates_iter = PATTERNS.date.captures_iter(&date_element_inner_html);

    let date_match = dates_iter.next().unwrap();
    let start_date = NaiveDate::from_ymd(
        date_match[3].parse::<i32>().unwrap() + 2000,
        date_match[1].parse().unwrap(),
        date_match[2].parse().unwrap(),
    )
    .format("%Y-%m-%d")
    .to_string();

    let date_match = dates_iter.next().unwrap();
    let end_date = NaiveDate::from_ymd(
        date_match[3].parse::<i32>().unwrap() + 2000,
        date_match[1].parse().unwrap(),
        date_match[2].parse().unwrap(),
    )
    .format("%Y-%m-%d")
    .to_string();

    let tag_urls = tournament_element
        .select(&SELECTORS.tag)
        .map(|e| {
            let inner_html = e.inner_html();
            PATTERNS.img_url.captures(&inner_html).unwrap()[1].to_owned()
        })
        .collect::<Vec<String>>();

    let logo_url = {
        let inner_html = tournament_element
            .select(&SELECTORS.logo)
            .next()
            .unwrap()
            .inner_html();
        PATTERNS.img_url.captures(&inner_html).map(|c| {
            if !c[1].starts_with("http") {
                format!("https://www.pickleballtournaments.com{}", &c[1])
            } else {
                c[1].to_owned()
            }
        })
    };

    let registration_status = match tournament_element.select(&SELECTORS.registration).next() {
        Some(registration_element) => {
            let registration_html = registration_element.html();
            match PATTERNS.registration.captures(&registration_html) {
                Some(c) => match &c[1] {
                    "closednow" => match registration_element.select(&SELECTORS.is_adonly).next() {
                        Some(_) => RegistrationStatus::NotOpen,
                        None => RegistrationStatus::Closed,
                    },
                    "closedpayonlynow" => {
                        let date_match = PATTERNS.date.captures(&registration_html).unwrap();
                        RegistrationStatus::ClosedToNew {
                            payment_deadline: NaiveDate::from_ymd(
                                date_match[3].parse::<i32>().unwrap() + 2000,
                                date_match[1].parse().unwrap(),
                                date_match[2].parse().unwrap(),
                            )
                            .format("%Y-%m-%d")
                            .to_string(),
                        }
                    }
                    "opennow" => {
                        let date_match = PATTERNS.date.captures(&registration_html).unwrap();
                        RegistrationStatus::Open {
                            deadline: NaiveDate::from_ymd(
                                date_match[3].parse::<i32>().unwrap() + 2000,
                                date_match[1].parse().unwrap(),
                                date_match[2].parse().unwrap(),
                            )
                            .format("%Y-%m-%d")
                            .to_string(),
                        }
                    }
                    other => panic!("unknown registration status: {}", other),
                },
                None => {
                    let soon_element = registration_element.select(&SELECTORS.soon).next().unwrap();
                    let inner_html = soon_element.inner_html();
                    let date_match = PATTERNS.date.captures(&inner_html).unwrap();
                    RegistrationStatus::OpenSoon {
                        start_date: NaiveDate::from_ymd(
                            date_match[3].parse::<i32>().unwrap() + 2000,
                            date_match[1].parse().unwrap(),
                            date_match[2].parse().unwrap(),
                        )
                        .format("%Y-%m-%d")
                        .to_string(),
                        start_time: PATTERNS.soon_time.captures(&inner_html).unwrap()[1].to_owned(),
                    }
                }
            }
        }
        None => RegistrationStatus::Closed,
    };

    TournamentListing {
        id,
        name,
        location,
        start_date,
        end_date,
        tag_urls,
        logo_url,
        registration_status,
    }
}
