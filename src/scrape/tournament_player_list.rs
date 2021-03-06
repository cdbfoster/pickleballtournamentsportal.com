use std::time::Duration;

use once_cell::sync::Lazy;
use regex::Regex;
use rocket::serde::Serialize;
use scraper::{Html, Selector};

use crate::client::Client;
use crate::scrape::{
    ScrapeCache, ScrapeResult, TOURNAMENT_PAGE_REFRESH, TOURNAMENT_PLAYER_LIST_REFRESH,
};
use crate::util::cache::{CacheGuard, CacheMapGuard};
use crate::util::guard_stack::GuardStack;

pub type PlayerList = Vec<Player>;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub id: usize,
    pub first_name: String,
    pub last_name: String,
    pub nick_names: Vec<String>,
    pub from: String,
}

pub type PlayerListGuard<'a> = GuardStack<
    'a,
    (
        CacheMapGuard<'a, usize, PlayerList>,
        CacheGuard<'a, PlayerList>,
    ),
    PlayerList,
>;

pub async fn tournament_player_list<'a>(
    tournament_id: usize,
    client: &'a Client<'a>,
    cache: &'a ScrapeCache,
) -> ScrapeResult<PlayerListGuard<'a>> {
    cache
        .tournament_player_list
        .get(tournament_id)
        .await
        .try_push_guard_async(|player_list_cache| async move {
            player_list_cache
                .retrieve_or_update(
                    Duration::from_secs(TOURNAMENT_PLAYER_LIST_REFRESH),
                    || async {
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

                        let tournament_page_html = Html::parse_document(&tournament_page_raw_html);

                        Ok(tournament_page_html
                            .select(&SELECTORS.player)
                            .map(|player_row| {
                                let name_element =
                                    player_row.select(&SELECTORS.player_name).next().unwrap();
                                let name_html = name_element.html();
                                let id = PATTERNS.player_id.captures(&name_html).unwrap()[1]
                                    .parse()
                                    .unwrap();
                                let name_matches =
                                    PATTERNS.player_name.captures(&name_html).unwrap();

                                let from_element =
                                    player_row.select(&SELECTORS.player_from).next().unwrap();

                                Player {
                                    id,
                                    first_name: name_matches[2].trim().to_owned(),
                                    last_name: name_matches[1].to_owned(),
                                    nick_names: PATTERNS
                                        .player_nick_name
                                        .captures_iter(&name_html)
                                        .map(|c| c[1].to_owned())
                                        .collect(),
                                    from: from_element.inner_html(),
                                }
                            })
                            .collect::<Vec<_>>())
                    },
                )
                .await
        })
        .await
}

struct Selectors {
    player: Selector,
    player_name: Selector,
    player_from: Selector,
}

static SELECTORS: Lazy<Selectors> = Lazy::new(|| Selectors {
    player: Selector::parse("#menuPlayerList .playerlist-wrap table tr").unwrap(),
    player_name: Selector::parse(".col-player > a").unwrap(),
    player_from: Selector::parse(".col-from").unwrap(),
});

struct Patterns {
    player_id: Regex,
    player_name: Regex,
    player_nick_name: Regex,
}

static PATTERNS: Lazy<Patterns> = Lazy::new(|| Patterns {
    player_id: Regex::new(r"&amp;id=(\d+)").unwrap(),
    player_name: Regex::new(r"<span>([^<]+)</span>, ([^(<]+)").unwrap(),
    player_nick_name: Regex::new(r"\(([^)]+)\)").unwrap(),
});

#[derive(Clone, Debug, Default)]
pub(super) struct FindPlayerQuery {
    /// This could contain either a first name or a nickname, since we won't always know.
    pub(super) first_name: Option<String>,
    /// If the caller fills this out, it is guaranteed to be a nickname.
    pub(super) nick_name: Option<String>,
    /// PickleballTournaments.com always specifies at least the last name.
    pub(super) last_name: String,
}

impl FindPlayerQuery {
    pub(super) fn from_last_name(name: &str) -> Self {
        Self {
            last_name: name.to_owned(),
            ..Default::default()
        }
    }
}

pub(super) fn find_player<'a>(
    query: FindPlayerQuery,
    players: &'a [Player],
    exclude: Option<&[Player]>,
    context: Option<&str>,
) -> Option<&'a Player> {
    let hits = players
        .iter()
        .filter(|p| exclude.is_none() || !exclude.unwrap().contains(p))
        .filter(|p| query.last_name == p.last_name)
        .filter(|p| {
            query.nick_name.is_none() || p.nick_names.contains(query.nick_name.as_ref().unwrap())
        })
        .filter(|p| {
            query.first_name.is_none()
                || query.first_name.as_ref() == Some(&p.first_name)
                || p.nick_names.contains(query.first_name.as_ref().unwrap())
        })
        .collect::<Vec<_>>();

    if hits.is_empty() {
        None
    } else if hits.len() == 1 {
        Some(hits[0])
    } else if let Some(context) = context {
        // Break ties by seeing if any of the hits appear whole in the context
        Some(
            *hits
                .iter()
                .find(|p| {
                    context.contains(&format!(
                        "{} {}{}",
                        p.first_name,
                        if p.nick_names.is_empty() {
                            String::new()
                        } else {
                            format!("({}) ", p.nick_names.join(") ("))
                        },
                        p.last_name,
                    ))
                })
                .unwrap_or(&hits[0]),
        )
    } else {
        Some(hits[0])
    }
}
