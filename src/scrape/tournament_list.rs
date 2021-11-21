use std::time::Duration;

use async_std::sync::RwLockReadGuard;
use chrono::prelude::*;
use once_cell::sync::Lazy;
use regex::Regex;
use rocket::serde::Serialize;
use scraper::{ElementRef, Html, Selector};

use crate::client::Client;
use crate::scrape::{ScrapeCache, ScrapeResult, TOURNAMENT_LIST_REFRESH};

pub type TournamentList = Vec<TournamentListing>;

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct TournamentListing {
    pub id: usize,
    pub name: String,
    pub location: String,
    pub start_date: String,
    pub end_date: String,
    pub tag_urls: Vec<String>,
    pub logo_url: Option<String>,
    pub registration_status: RegistrationStatus,
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

pub type TournamentListGuard<'a> = RwLockReadGuard<'a, TournamentList>;

pub async fn tournament_list<'a>(
    client: &'a Client<'a>,
    cache: &'a ScrapeCache,
) -> ScrapeResult<TournamentListGuard<'a>> {
    cache
        .tournament_list
        .retrieve_or_update(Duration::from_secs(TOURNAMENT_LIST_REFRESH), || async {
            let future_raw_html = cache
                .pages
                .retrieve_or_update(
                    Duration::from_secs(TOURNAMENT_LIST_REFRESH),
                    "https://www.pickleballtournaments.com/pbt_tlisting.pl?when=F",
                    |url| async { client.get(url).send().await },
                    "could not load future tournaments",
                )
                .await?
                .clone();

            let past_raw_html = cache
                .pages
                .retrieve_or_update(
                    Duration::from_secs(TOURNAMENT_LIST_REFRESH),
                    "https://www.pickleballtournaments.com/pbt_tlisting.pl?when=P",
                    |url| async {
                        client
                            .get(url)
                            .header(
                                "Referer",
                                "https://www.pickleballtournaments.com/pbt_tlisting.pl?when=F",
                            )
                            .header("Sec-Fetch-Site", "same-origin")
                            .send()
                            .await
                    },
                    "could not load past tournaments",
                )
                .await?
                .clone();

            let future_document = Html::parse_document(&future_raw_html);
            let past_document = Html::parse_document(&past_raw_html);

            let tournament_listings = future_document
                .select(&SELECTORS.tournament)
                .chain(past_document.select(&SELECTORS.tournament))
                .map(parse_tournament_listing)
                .collect::<Vec<_>>();

            Ok(tournament_listings)
        })
        .await
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
