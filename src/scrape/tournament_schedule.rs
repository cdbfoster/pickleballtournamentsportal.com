use std::time::Duration;

use once_cell::sync::Lazy;
use regex::Regex;
use rocket::serde::Serialize;
use scraper::{Html, Selector};

use crate::client::Client;
use crate::scrape::tournament_event_group_list::tournament_event_group_list;
use crate::scrape::{
    ScrapeCache, ScrapeResult, TOURNAMENT_PAGE_REFRESH, TOURNAMENT_SCHEDULE_REFRESH,
};
use crate::util::cache::{CacheGuard, CacheMapGuard};
use crate::util::guard_stack::GuardStack;

pub type Schedule = Vec<ScheduleItem>;

#[derive(Clone, Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct ScheduleItem {
    pub date: String,
    pub time: String,
    pub venue: String,
    pub event: String,
    pub link: bool,
}

pub type ScheduleGuard<'a> =
    GuardStack<'a, (CacheMapGuard<'a, usize, Schedule>, CacheGuard<'a, Schedule>), Schedule>;

pub async fn tournament_schedule<'a>(
    tournament_id: usize,
    client: &'a Client<'a>,
    cache: &'a ScrapeCache,
) -> ScrapeResult<ScheduleGuard<'a>> {
    cache
        .tournament_schedule
        .get(tournament_id)
        .await
        .try_push_guard_async(|schedule_cache| async move {
            schedule_cache
                .retrieve_or_update(Duration::from_secs(TOURNAMENT_SCHEDULE_REFRESH), || async {
                    let event_groups =
                        tournament_event_group_list(tournament_id, client, cache).await?;

                    let tournament_page_url = format!(
                        "https://www.pickleballtournaments.com/tournamentinfo.pl?tid={}",
                        tournament_id
                    );

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

                    let tournament_page = Html::parse_document(&tournament_page_raw_html);

                    let mut schedule = Vec::new();

                    for day_element in tournament_page.select(&SELECTORS.day) {
                        let mut headers = day_element.select(&SELECTORS.header);
                        let date = headers.next().unwrap().inner_html();
                        let venues = {
                            let mut values = headers.map(|h| h.inner_html()).collect::<Vec<_>>();
                            values.pop(); // The last one isn't a venue.
                            values
                        };

                        for row in day_element.select(&SELECTORS.row).skip(2) {
                            // Sometimes there are blank rows at the end of a day?
                            let time =
                                if let Some(time_element) = row.select(&SELECTORS.time).next() {
                                    time_element.inner_html()
                                } else {
                                    continue;
                                };

                            for (venue, events_block) in
                                venues.iter().zip(row.select(&SELECTORS.events))
                            {
                                let event_list = events_block.inner_html();

                                // They bold schedule items that are bad, like wait lists.
                                if event_list.starts_with("<b>") {
                                    continue;
                                }

                                event_list
                                    .split("<br>")
                                    .filter(|e| e != &"&nbsp;")
                                    .for_each(|e| {
                                        let url = PATTERNS.url.captures(e).map(|c| {
                                            format!(
                                                "https://www.pickleballtournaments.com/{}",
                                                &c[1]
                                            )
                                        });

                                        let name = PATTERNS.name.captures(e).unwrap()[2].to_owned();

                                        schedule.push(ScheduleItem {
                                            date: date.clone(),
                                            time: time.clone(),
                                            venue: venue.clone(),
                                            event: name,
                                            link: url
                                                .and_then(|url| {
                                                    event_groups
                                                        .iter()
                                                        .flat_map(|g| g.events.iter())
                                                        .find(|e| e.content.url() == url)
                                                })
                                                .is_some(),
                                        })
                                    });
                            }
                        }
                    }

                    Ok(schedule)
                })
                .await
        })
        .await
}

struct Selectors {
    day: Selector,
    row: Selector,
    header: Selector,
    time: Selector,
    events: Selector,
}

static SELECTORS: Lazy<Selectors> = Lazy::new(|| Selectors {
    day: Selector::parse("#menuSchedule table").unwrap(),
    row: Selector::parse("tr").unwrap(),
    header: Selector::parse("tr th").unwrap(),
    time: Selector::parse("td b").unwrap(),
    events: Selector::parse("td:not(:first-child)").unwrap(),
});

struct Patterns {
    url: Regex,
    name: Regex,
}

static PATTERNS: Lazy<Patterns> = Lazy::new(|| Patterns {
    url: Regex::new(r#"href="([^"]+)""#).unwrap(),
    name: Regex::new(r"^(:?<a[^>]+>)?([^<]+)(:?</a>)?$").unwrap(),
});
