use std::collections::HashMap;
use std::time::Duration;

use once_cell::sync::Lazy;
use regex::Regex;
use rocket::serde::Serialize;
use scraper::{ElementRef, Html, Selector};

use crate::client::Client;
use crate::scrape::{
    ScrapeCache, ScrapeResult, TOURNAMENT_EVENT_LIST_REFRESH,
    TOURNAMENT_EVENT_BRACKET_PAGE_REFRESH, TOURNAMENT_EVENT_PLAYER_LIST_PAGES_REFRESH,
    TOURNAMENT_PAGE_REFRESH,
};
use crate::util::cache::{CacheGuard, CacheMapGuard};
use crate::util::guard_stack::GuardStack;

pub type EventGroupList = Vec<EventGroup>;

#[derive(Clone, Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct EventGroup {
    pub name: String,
    pub events: Vec<Event>,
}

#[derive(Clone, Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub name: String,
    #[serde(skip_serializing)]
    pub content: EventContent,
}

#[derive(Clone)]
pub enum EventContent {
    BracketUrl(String),
    GroupListUrl(String),
    ListUrl(String),
}

impl EventContent {
    pub fn url(&self) -> &str {
        match self {
            EventContent::BracketUrl(url) => &url,
            EventContent::GroupListUrl(url) => &url,
            EventContent::ListUrl(url) => &url,
        }
    }
}

pub type EventGroupListGuard<'a> = GuardStack<
    'a,
    (
        CacheMapGuard<'a, usize, EventGroupList>,
        CacheGuard<'a, EventGroupList>,
    ),
    EventGroupList,
>;

pub async fn tournament_event_group_list<'a>(
    tournament_id: usize,
    client: &'a Client<'a>,
    cache: &'a ScrapeCache,
) -> ScrapeResult<EventGroupListGuard<'a>> {
    cache
        .tournament_event_list
        .get(tournament_id)
        .await
        .try_push_guard_async(|event_list_cache| async move {
            event_list_cache
                .retrieve_or_update(
                    Duration::from_secs(TOURNAMENT_EVENT_LIST_REFRESH),
                    || async {
                        let tournament_page_url = format!(
                            "https://www.pickleballtournaments.com/tournamentinfo.pl?tid={}",
                            tournament_id
                        );

                        let event_bracket_page_url = format!(
                            "https://www.pickleballtournaments.com/cinfo.pl?tid={}",
                            tournament_id,
                        );

                        // First, try to get the event groups from the Events/Brackets page.
                        let mut event_groups = {
                            let event_bracket_page_raw_html = cache
                                .pages
                                .retrieve_or_update(
                                    Duration::from_secs(TOURNAMENT_EVENT_BRACKET_PAGE_REFRESH),
                                    &event_bracket_page_url,
                                    |url| async {
                                        client
                                            .get(url)
                                            .header("Referer", &tournament_page_url)
                                            .header("Sec-Fetch-Site", "same-origin")
                                            .send()
                                            .await
                                    },
                                    "could not load event tournament bracket list",
                                )
                                .await?
                                .clone();

                            let event_bracket_page =
                                Html::parse_document(&event_bracket_page_raw_html);

                            event_bracket_page
                                .select(&SELECTORS.section)
                                .map(|n| {
                                    let event_group_name = n.inner_html();

                                    let mut url_to_event_name = HashMap::new();

                                    n.next_siblings()
                                        .filter_map(|s| ElementRef::wrap(s))
                                        .skip(1)
                                        .take_while(|e| &e.value().name.local != "h2")
                                        .for_each(|e| {
                                            let event_element =
                                                e.select(&SELECTORS.event).next().unwrap();
                                            let event_name = event_element.inner_html();
                                            let event_html = event_element.html();
                                            let event_url =
                                                PATTERNS.url.captures(&event_html).unwrap()[1]
                                                    .to_owned();

                                            url_to_event_name
                                                .entry(event_url)
                                                .or_insert(Vec::new())
                                                .push(event_name);
                                        });

                                    EventGroup {
                                        name: event_group_name,
                                        events: url_to_event_name
                                            .into_iter()
                                            .map(|(url, names)| Event {
                                                name: common_name(&names),
                                                content: if url.contains("rptbrackets.pl") {
                                                    EventContent::ListUrl(format!(
                                                        "https://www.pickleballtournaments.com/{}",
                                                        url
                                                    ))
                                                } else if url.contains("show.pl") {
                                                    let bracket_filename_captures = PATTERNS.bracket_filename.captures(&url).unwrap();
                                                    EventContent::BracketUrl(format!(
                                                        "https://www.pickleballtournaments.com/Tournaments/{}/{}",
                                                        bracket_filename_captures[1].replace("%2F", "/"),
                                                        &bracket_filename_captures[2],
                                                    ))
                                                } else {
                                                    panic!("Unknown event url: {:?}", url)
                                                },
                                            })
                                            .collect(),
                                    }
                                })
                                .collect::<Vec<_>>()
                        };

                        // If we don't get any event groups from that, scrape each of the Event Player List pages
                        if event_groups.is_empty() {
                            let tournament_page_raw_html = cache
                                .pages
                                .retrieve_or_update(
                                    Duration::from_secs(TOURNAMENT_PAGE_REFRESH),
                                    &tournament_page_url,
                                    |url| async { client.get(url).send().await },
                                    "could not load tournament info",
                                )
                                .await?
                                .clone();

                            let event_list_urls = {
                                let tournament_page =
                                    Html::parse_document(&tournament_page_raw_html);

                                tournament_page
                                    .select(&SELECTORS.event_list)
                                    .filter(|e| {
                                        !sanitize_name(&e.inner_html()).contains("waitlist")
                                    })
                                    .map(|e| {
                                        let html = e.html();
                                        (
                                            e.inner_html(),
                                            format!(
                                                "https://www.pickleballtournaments.com/{}",
                                                &PATTERNS.url.captures(&html).unwrap()[1]
                                            ),
                                        )
                                    })
                                    .collect::<Vec<_>>()
                            };

                            let mut event_pages = Vec::with_capacity(event_list_urls.len());
                            for (name, url) in event_list_urls.iter() {
                                event_pages.push(
                                    cache
                                        .pages
                                        .retrieve_or_update(
                                            Duration::from_secs(
                                                TOURNAMENT_EVENT_PLAYER_LIST_PAGES_REFRESH,
                                            ),
                                            url,
                                            |url| async {
                                                client
                                                    .get(url)
                                                    .header("Referer", &tournament_page_url)
                                                    .header("Sec-Fetch-Site", "same-origin")
                                                    .send()
                                                    .await
                                            },
                                            &format!(
                                                "could not load event player page for {:?}",
                                                name
                                            ),
                                        )
                                        .await?
                                        .clone(),
                                );
                            }

                            event_groups = event_list_urls
                                .into_iter()
                                .zip(event_pages.into_iter())
                                .map(|((name, url), page_raw_html)| {
                                    let page = Html::parse_document(&page_raw_html);

                                    EventGroup {
                                        name,
                                        events: page
                                            .select(&SELECTORS.section)
                                            .map(|s| s.inner_html())
                                            .filter(|s| !s.trim().is_empty())
                                            .map(|s| Event {
                                                name: s,
                                                content: EventContent::GroupListUrl(url.clone()),
                                            })
                                            .collect(),
                                    }
                                })
                                .collect();
                        }

                        Ok(event_groups)
                    },
                )
                .await
        })
        .await
}

struct Selectors {
    section: Selector,
    event: Selector,
    event_list: Selector,
}

static SELECTORS: Lazy<Selectors> = Lazy::new(|| Selectors {
    section: Selector::parse("h2.section-title").unwrap(),
    event: Selector::parse("tr a").unwrap(),
    event_list: Selector::parse("#menuEventList ul li a").unwrap(),
});

struct Patterns {
    url: Regex,
    bracket_filename: Regex,
}

static PATTERNS: Lazy<Patterns> = Lazy::new(|| Patterns {
    url: Regex::new(r#"href="([^"]+)""#).unwrap(),
    bracket_filename: Regex::new("&amp;dir=([^&]+)&amp;filename=(.+)$").unwrap(),
});

/// Finds the common prefix in a list of names
fn common_name(names: &[String]) -> String {
    let name_parts = names
        .iter()
        .map(|n| n.trim_end().split(" ").collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let minimum_parts = name_parts.iter().map(|n| n.len()).min().unwrap();

    for i in 0..minimum_parts {
        if !name_parts.iter().skip(1).all(|n| n[i] == name_parts[0][i]) {
            return name_parts[0][..i].join(" ");
        }
    }

    name_parts[0][..minimum_parts].join(" ")
}

fn sanitize_name(name: &str) -> String {
    name.to_lowercase().replace(" ", "").replace("-", "")
}
