use std::time::Duration;

use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{ElementRef, Html, Selector};

use crate::client::Client;
use crate::scrape::tournament_event_group_list::{Event, EventUrl};
use crate::scrape::tournament_player_list::{
    find_player, tournament_player_list, FindPlayerQuery, Player, PlayerList,
};
use crate::scrape::{
    ScrapeCache, ScrapeError, ScrapeResult, EVENT_TEAM_LIST_REFRESH,
    TOURNAMENT_EVENT_PLAYER_LIST_PAGES_REFRESH,
};
use crate::util::cache::{CacheGuard, CacheMapGuard};
use crate::util::guard_stack::GuardStack;

pub type TeamList = Vec<PlayerList>;

pub type EventTeamListGuard<'a> = GuardStack<
    'a,
    (
        CacheMapGuard<'a, (usize, String), TeamList>,
        CacheGuard<'a, TeamList>,
    ),
    TeamList,
>;

pub async fn event_team_list<'a>(
    tournament_id: usize,
    event: &Event,
    client: &'a Client<'a>,
    cache: &'a ScrapeCache,
) -> ScrapeResult<EventTeamListGuard<'a>> {
    cache
        .event_team_list
        .get((tournament_id, event.name.clone()))
        .await
        .try_push_guard_async(|event_team_list_cache| async move {
            event_team_list_cache
                .retrieve_or_update(Duration::from_secs(EVENT_TEAM_LIST_REFRESH), || async {
                    let tournament_page_url = format!(
                        "https://www.pickleballtournaments.com/tournamentinfo.pl?tid={}",
                        tournament_id
                    );

                    let tournament_player_list =
                        tournament_player_list(tournament_id, client, cache).await?;

                    let team_list = match event.url {
                        EventUrl::List(_) => {
                            scrape_team_list_rptbrackets(
                                event,
                                &*tournament_player_list,
                                client,
                                cache,
                            )
                            .await?
                        }
                        EventUrl::GroupList(_) => {
                            scrape_team_list_ereport(
                                event,
                                &tournament_page_url,
                                &*tournament_player_list,
                                client,
                                cache,
                            )
                            .await?
                        }
                        _ => Vec::new(),
                    };

                    Ok(team_list)
                })
                .await
        })
        .await
}

async fn scrape_team_list_rptbrackets<'a>(
    event: &Event,
    tournament_player_list: &[Player],
    client: &'a Client<'a>,
    cache: &'a ScrapeCache,
) -> ScrapeResult<TeamList> {
    let page_raw_html = cache
        .pages
        .retrieve_or_update(
            Duration::from_secs(TOURNAMENT_EVENT_PLAYER_LIST_PAGES_REFRESH),
            event.url.as_str(),
            |url| async { client.get(url).send().await },
            "could not load event player list",
        )
        .await?
        .clone();

    let page = Html::parse_document(&page_raw_html);

    Ok(page
        .select(&SELECTORS.rpt_player)
        .map(|e| e.inner_html())
        .map(|t| {
            t.split('/')
                .filter(|s| !s.trim().is_empty())
                .filter_map(|s| {
                    let name_matches = PATTERNS.name.captures(s).unwrap();

                    find_player(
                        FindPlayerQuery {
                            first_name: name_matches.get(2).map(|n| n.as_str().trim().to_owned()),
                            nick_name: name_matches.get(3).map(|n| n.as_str().to_owned()),
                            last_name: name_matches[1].to_owned(),
                        },
                        tournament_player_list,
                    )
                    .cloned()
                })
                .collect::<Vec<_>>()
        })
        .collect())
}

async fn scrape_team_list_ereport<'a>(
    event: &Event,
    tournament_page_url: &str,
    tournament_player_list: &[Player],
    client: &'a Client<'a>,
    cache: &'a ScrapeCache,
) -> ScrapeResult<TeamList> {
    let page_raw_html = cache
        .pages
        .retrieve_or_update(
            Duration::from_secs(TOURNAMENT_EVENT_PLAYER_LIST_PAGES_REFRESH),
            event.url.as_str(),
            |url| async {
                client
                    .get(url)
                    .header("Referer", tournament_page_url)
                    .header("Sec-Fetch-Site", "same-origin")
                    .send()
                    .await
            },
            "could not load event tournament bracket list",
        )
        .await?
        .clone();

    let page = Html::parse_document(&page_raw_html);

    page.select(&SELECTORS.ereport_section)
        .find(|e| event.name == e.inner_html())
        .map(|e| {
            e.next_siblings()
                .filter_map(ElementRef::wrap)
                .take_while(|e| &e.value().name.local != "h2")
                .map(|e| {
                    e.select(&SELECTORS.ereport_player)
                        .filter_map(|p| {
                            let name = p.inner_html();
                            let name_matches = PATTERNS.name.captures(&name).unwrap();

                            find_player(
                                FindPlayerQuery {
                                    first_name: name_matches
                                        .get(2)
                                        .map(|n| n.as_str().trim().to_owned()),
                                    nick_name: name_matches.get(3).map(|n| n.as_str().to_owned()),
                                    last_name: name_matches[1].to_owned(),
                                },
                                tournament_player_list,
                            )
                            .cloned()
                        })
                        .collect::<Vec<_>>()
                })
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
        })
        .ok_or_else(|| ScrapeError::from_str("event not found"))
}

struct Selectors {
    ereport_section: Selector,
    ereport_player: Selector,
    rpt_player: Selector,
}

static SELECTORS: Lazy<Selectors> = Lazy::new(|| Selectors {
    ereport_section: Selector::parse(".eventplayer-list > h2.section-title").unwrap(),
    ereport_player: Selector::parse(".team-wrap .col-name").unwrap(),
    rpt_player: Selector::parse(".rptbrackets > .tab-content > ul > li").unwrap(),
});

struct Patterns {
    name: Regex,
}

static PATTERNS: Lazy<Patterns> = Lazy::new(|| Patterns {
    name: Regex::new(r"([^,]+)(?:, ?([^<(]+)(?:\(([^)]+)\))?)?").unwrap(),
});
