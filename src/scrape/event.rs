use std::fmt;
use std::ops::{Deref, Neg};
use std::time::Duration;

use once_cell::sync::Lazy;
use regex::Regex;
use rocket::serde::Serialize;
use scraper::{ElementRef, Html, Selector};

use crate::client::Client;
use crate::scrape::tournament_event_group_list::{Event, EventUrl};
use crate::scrape::tournament_player_list::{
    find_player, tournament_player_list, FindPlayerQuery, Player, PlayerList,
};
use crate::scrape::{
    ScrapeCache, ScrapeError, ScrapeResult, EVENT_BRACKET_REFRESH, EVENT_TEAM_LIST_REFRESH,
    TOURNAMENT_EVENT_PLAYER_LIST_PAGES_REFRESH,
};
use crate::util::cache::{CacheGuard, CacheMapGuard};
use crate::util::guard_stack::GuardStack;

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub enum Bracket {
    DoubleElim(Vec<BracketMatch>),
    RoundRobin(Vec<Vec<BracketMatch>>),
}

impl Default for Bracket {
    fn default() -> Self {
        Self::RoundRobin(Vec::new())
    }
}

pub type EventBracketGuard<'a> = GuardStack<
    'a,
    (
        CacheMapGuard<'a, (usize, String), Bracket>,
        CacheGuard<'a, Bracket>,
    ),
    Bracket,
>;

pub async fn event_bracket<'a>(
    tournament_id: usize,
    event: &Event,
    client: &'a Client<'a>,
    cache: &'a ScrapeCache,
) -> ScrapeResult<EventBracketGuard<'a>> {
    cache
        .event_bracket
        .get((tournament_id, event.name.clone()))
        .await
        .try_push_guard_async(|event_bracket_cache| async move {
            event_bracket_cache
                .retrieve_or_update(Duration::from_secs(EVENT_BRACKET_REFRESH), || async {
                    let teams = event_team_list(tournament_id, event, client, cache).await?;

                    let page_raw_html = cache
                        .pages
                        .retrieve_or_update(
                            Duration::from_secs(EVENT_BRACKET_REFRESH),
                            event.url.as_str(),
                            |url| async { client.get(url).send().await },
                            "could not load event bracket",
                        )
                        .await?
                        .clone();

                    let page = Html::parse_document(&page_raw_html);

                    if is_round_robin(&page) {
                        Ok(Bracket::RoundRobin(
                            page.select(&SELECTORS.bracket_table)
                                .next()
                                .and_then(|t| t.select(&SELECTORS.row).nth(4))
                                .map(|r| {
                                    r.select(&SELECTORS.cell)
                                        .skip(1)
                                        .step_by(3)
                                        .map(|c| {
                                            std::iter::successors(Some(GridCell(c)), |c| {
                                                c.neighbor(Direction::Down)
                                            })
                                            .step_by(4)
                                            .map(BracketPosition)
                                            .map(BracketNode::crawl_from)
                                            .map(|n| BracketMatch::from_node(&n, &teams))
                                            .collect()
                                        })
                                        .collect()
                                })
                                .unwrap_or_default(),
                        ))
                    } else {
                        Ok(Bracket::DoubleElim(
                            page.select(&SELECTORS.bracket_table)
                                .filter_map(|t| {
                                    t.select(&SELECTORS.match_label).max_by_key(|l| {
                                        l.inner_html()[1..].parse::<usize>().unwrap()
                                    })
                                })
                                .filter_map(|l| {
                                    l.ancestors().nth(1).and_then(|l| l.next_siblings().nth(1))
                                })
                                .filter_map(ElementRef::wrap)
                                .map(GridCell)
                                .map(BracketPosition)
                                .map(BracketNode::crawl_from)
                                .map(|n| BracketMatch::from_node(&n, &teams))
                                .collect(),
                        ))
                    }
                })
                .await
        })
        .await
}

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
                        EventUrl::Bracket(_) => {
                            scrape_team_list_bracket(event, &*tournament_player_list, client, cache)
                                .await?
                        }
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
                .filter_map(|s| name_to_player(s, tournament_player_list, &page_raw_html))
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
                            name_to_player(&p.inner_html(), tournament_player_list, &page_raw_html)
                        })
                        .collect::<Vec<_>>()
                })
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
        })
        .ok_or_else(|| ScrapeError::from_str("event not found"))
}

fn is_round_robin(page: &Html) -> bool {
    page.select(&SELECTORS.table)
        .nth(1)
        .and_then(|t| t.select(&SELECTORS.row).next())
        .map(|r| r.select(&SELECTORS.cell).all(|e| e.inner_html().is_empty()))
        .unwrap_or(false)
}

async fn scrape_team_list_bracket<'a>(
    event: &Event,
    tournament_player_list: &[Player],
    client: &'a Client<'a>,
    cache: &'a ScrapeCache,
) -> ScrapeResult<TeamList> {
    let page_raw_html = cache
        .pages
        .retrieve_or_update(
            Duration::from_secs(EVENT_BRACKET_REFRESH),
            event.url.as_str(),
            |url| async { client.get(url).send().await },
            "could not load event bracket",
        )
        .await?
        .clone();

    let page = Html::parse_document(&page_raw_html);

    if is_round_robin(&page) {
        let mut players = page
            .select(&SELECTORS.table)
            .nth(1)
            .unwrap()
            .select(&SELECTORS.row)
            .skip(2)
            .map(|r| r.select(&SELECTORS.cell).nth(1).unwrap().inner_html())
            .map(|t| {
                PATTERNS
                    .player
                    .captures_iter(&t)
                    .filter_map(|c| name_to_player(&c[1], tournament_player_list, &page_raw_html))
                    .collect::<Vec<_>>()
            })
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>();

        // This would happen if the bracket only displays last names for whatever goddamn reason.
        // Use a player filter that doesn't look for commas.
        if players.is_empty() {
            players = page
                .select(&SELECTORS.table)
                .nth(1)
                .unwrap()
                .select(&SELECTORS.row)
                .skip(2)
                .map(|r| r.select(&SELECTORS.cell).nth(1).unwrap().inner_html())
                .filter(|t| !t.contains("Matches Won") && !t.contains("Point Differential"))
                .map(|t| {
                    let splits = t.split('-').collect::<Vec<_>>();

                    if splits.len() == 1 {
                        // One player, no hyphens
                        splits
                            .iter()
                            .filter_map(|p| {
                                name_to_player(p, tournament_player_list, &page_raw_html)
                            })
                            .collect()
                    } else if splits.len() == 2 {
                        if let Some(p) = name_to_player(&t, tournament_player_list, &page_raw_html)
                        {
                            // Check to see if this is a hyphenated last name.
                            vec![p]
                        } else {
                            // Otherwise, search for the names individually.
                            splits
                                .iter()
                                .filter_map(|p| {
                                    name_to_player(p, tournament_player_list, &page_raw_html)
                                })
                                .collect()
                        }
                    } else if splits.len() == 3 {
                        // One of the names are hyphenated, so return whichever has more matches.
                        let first_joined = [splits[..2].join("-"), splits[2].to_owned()]
                            .iter()
                            .filter_map(|p| {
                                name_to_player(p, tournament_player_list, &page_raw_html)
                            })
                            .collect::<Vec<_>>();

                        let second_joined = [splits[0].to_owned(), splits[1..].join("-")]
                            .iter()
                            .filter_map(|p| {
                                name_to_player(p, tournament_player_list, &page_raw_html)
                            })
                            .collect::<Vec<_>>();

                        if first_joined.len() >= second_joined.len() {
                            first_joined
                        } else {
                            second_joined
                        }
                    } else {
                        // Apparently both names are hyphenated.
                        [splits[..2].join("-"), splits[2..].join("-")]
                            .iter()
                            .filter_map(|p| {
                                name_to_player(p, tournament_player_list, &page_raw_html)
                            })
                            .collect::<Vec<_>>()
                    }
                })
                .filter(|t| !t.is_empty())
                .collect();
        }

        Ok(players)
    } else {
        Ok(page
            .select(&SELECTORS.bracket_table)
            .flat_map(|t| {
                t.select(&SELECTORS.row)
                    .skip(1)
                    .filter_map(|r| {
                        let mut c = r.select(&SELECTORS.cell).take(2);
                        c.next()
                            .map(|t| t.inner_html())
                            .filter(|v| !v.is_empty())
                            .and_then(|v| {
                                if v == "(bye)" {
                                    c.next().map(|t| t.inner_html()).filter(|v| !v.is_empty())
                                } else {
                                    Some(v)
                                }
                            })
                    })
                    .map(|t| {
                        PATTERNS
                            .player
                            .captures_iter(&t)
                            .filter_map(|c| {
                                name_to_player(&c[1], tournament_player_list, &page_raw_html)
                            })
                            .collect::<Vec<_>>()
                    })
                    .filter(|t| !t.is_empty())
            })
            .collect())
    }
}

struct Selectors {
    ereport_section: Selector,
    ereport_player: Selector,
    rpt_player: Selector,
    table: Selector,
    bracket_table: Selector,
    row: Selector,
    cell: Selector,
    match_label: Selector,
    match_link: Selector,
}

static SELECTORS: Lazy<Selectors> = Lazy::new(|| Selectors {
    ereport_section: Selector::parse(".eventplayer-list > h2.section-title").unwrap(),
    ereport_player: Selector::parse(".team-wrap .col-name").unwrap(),
    rpt_player: Selector::parse(".rptbrackets > .tab-content > ul > li").unwrap(),
    table: Selector::parse("table").unwrap(),
    bracket_table: Selector::parse("hr + table").unwrap(),
    row: Selector::parse("tr").unwrap(),
    cell: Selector::parse("td").unwrap(),
    match_label: Selector::parse("td a").unwrap(),
    match_link: Selector::parse("font i").unwrap(),
});

struct Patterns {
    name: Regex,
    player: Regex,
    borders: Regex,
    scores: Regex,
    match_link: Regex,
}

static PATTERNS: Lazy<Patterns> = Lazy::new(|| Patterns {
    name: Regex::new(r"([^,]+)(?:, ?([^<(]+)(?:\(([^)]+)\))?)?").unwrap(),
    player: Regex::new(r"([^,]+,[^-]+)-?").unwrap(),
    borders: Regex::new(r#"style="(border-bottom:[^;"]+)?;?(border-left:[^"]+)?""#).unwrap(),
    scores: Regex::new(r"((?:\d+-\d+,?)+)").unwrap(),
    match_link: Regex::new(r"\((\w+) to #(\d+)\)").unwrap(),
});

fn sanitize_name(name: &str) -> String {
    name.replace("&nbsp;", " ")
}

fn name_to_query(name: &str) -> Option<FindPlayerQuery> {
    let sanitized = sanitize_name(name);
    let name_matches = PATTERNS.name.captures(&sanitized)?;
    Some(FindPlayerQuery {
        first_name: name_matches.get(2).map(|n| n.as_str().trim().to_owned()),
        nick_name: name_matches.get(3).map(|n| n.as_str().to_owned()),
        last_name: name_matches[1].to_owned(),
    })
}

fn name_to_player(name: &str, players: &[Player], source: &str) -> Option<Player> {
    let query = name_to_query(name)?;
    find_player(query, players, None, Some(source)).cloned()
}

fn resolve_team<'a>(text: &str, teams: &'a [Vec<Player>]) -> Option<&'a Vec<Player>> {
    let player_captures = PATTERNS.player.captures_iter(text).collect::<Vec<_>>();

    let query_teams = |queries: &[FindPlayerQuery]| -> Option<&'a Vec<Player>> {
        teams.iter().find(|t| {
            let mut found_players = Vec::with_capacity(t.len());
            t.len() == queries.len()
                && queries.iter().all(|q| {
                    find_player(q.clone(), t, Some(&found_players), None)
                        .map(|p| found_players.push(p.clone()))
                        .is_some()
                })
        })
    };

    if !player_captures.is_empty() {
        let queries = player_captures
            .into_iter()
            .filter_map(|c| name_to_query(&c[1]))
            .collect::<Vec<_>>();

        if queries.is_empty() {
            return None;
        }

        query_teams(&queries)
    } else {
        let splits = text.split('-').collect::<Vec<_>>();

        if splits.len() == 1 {
            // One player, no hyphens
            query_teams(&[FindPlayerQuery::from_last_name(splits[0])])
        } else if splits.len() == 2 {
            // Check to see if this is a hyphenated name first, then search individually.
            query_teams(&[FindPlayerQuery::from_last_name(text)]).or_else(|| {
                query_teams(&[
                    FindPlayerQuery::from_last_name(splits[0]),
                    FindPlayerQuery::from_last_name(splits[1]),
                ])
            })
        } else if splits.len() == 3 {
            // One of the names are hyphenated, so return whichever matches.
            query_teams(&[
                FindPlayerQuery::from_last_name(&splits[..2].join("-")),
                FindPlayerQuery::from_last_name(splits[2]),
            ])
            .or_else(|| {
                query_teams(&[
                    FindPlayerQuery::from_last_name(splits[0]),
                    FindPlayerQuery::from_last_name(&splits[1..].join("-")),
                ])
            })
        } else {
            // Apparently both names are hyphenated.
            query_teams(&[
                FindPlayerQuery::from_last_name(&splits[..2].join("-")),
                FindPlayerQuery::from_last_name(&splits[2..].join("-")),
            ])
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Direction {
    Up,
    Right,
    Down,
    Left,
}

impl Neg for Direction {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            Direction::Up => Direction::Down,
            Direction::Right => Direction::Left,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
        }
    }
}

#[derive(Clone, Copy)]
struct GridCell<'a>(ElementRef<'a>);

impl<'a> GridCell<'a> {
    fn neighbor(&self, direction: Direction) -> Option<Self> {
        match direction {
            Direction::Up => {
                let index = self.parent()?.children().position(|e| e == *self.0)?;

                self.parent()
                    .and_then(|p| p.prev_siblings().nth(1))
                    .and_then(|p| p.children().nth(index))
                    .and_then(ElementRef::wrap)
                    .map(Self)
            }
            Direction::Right => self
                .next_siblings()
                .nth(1)
                .and_then(ElementRef::wrap)
                .map(Self),
            Direction::Down => {
                let index = self.parent()?.children().position(|e| e == *self.0)?;

                self.parent()
                    .and_then(|p| p.next_siblings().nth(1))
                    .and_then(|p| p.children().nth(index))
                    .and_then(ElementRef::wrap)
                    .map(Self)
            }
            Direction::Left => self
                .prev_siblings()
                .nth(1)
                .and_then(ElementRef::wrap)
                .map(Self),
        }
    }
}

impl<'a> Deref for GridCell<'a> {
    type Target = ElementRef<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Copy)]
struct BracketPosition<'a>(GridCell<'a>);

impl<'a> BracketPosition<'a> {
    fn paths(&self) -> impl Iterator<Item = Direction> {
        #[derive(Debug, Default)]
        struct Openings {
            horizontal: bool,
            vertical: bool,
        }

        fn get_openings(pos: &BracketPosition<'_>) -> Openings {
            let html = pos.html();
            let captures = PATTERNS.borders.captures(&html);

            Openings {
                horizontal: captures.as_ref().and_then(|c| c.get(1)).is_some(),
                vertical: captures.as_ref().and_then(|c| c.get(2)).is_some(),
            }
        }

        let openings = get_openings(self);

        let down_openings = self
            .neighbor(Direction::Down)
            .map(Self)
            .map(|p| get_openings(&p))
            .unwrap_or_default();

        let left_openings = self
            .neighbor(Direction::Left)
            .map(Self)
            .map(|p| get_openings(&p))
            .unwrap_or_default();

        let mut paths = Vec::new();

        if openings.vertical {
            paths.push(Direction::Up);
        }

        if openings.horizontal {
            paths.push(Direction::Right);
        }

        if down_openings.vertical {
            paths.push(Direction::Down);
        }

        if left_openings.horizontal {
            paths.push(Direction::Left);
        }

        paths.into_iter()
    }
}

impl<'a> Deref for BracketPosition<'a> {
    type Target = GridCell<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> fmt::Debug for BracketPosition<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.inner_html())
    }
}

#[derive(Debug)]
struct BracketNode<'a> {
    current: BracketPosition<'a>,
    children: Vec<BracketNode<'a>>,
}

impl<'a> BracketNode<'a> {
    fn crawl_from(node: BracketPosition<'a>) -> Self {
        Self {
            current: node,
            children: node
                .paths()
                .filter(|&d| d != Direction::Right && d != Direction::Left)
                .filter_map(|mut d| {
                    let mut n = node;
                    while let Some(o) = n.neighbor(d) {
                        if d == Direction::Left && o.inner_html() != "&nbsp;" {
                            return Some(Self::crawl_from(BracketPosition(o)));
                        } else {
                            n = BracketPosition(o);
                            d = n.paths().find(|&e| e != -d)?;
                        }
                    }
                    None
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub enum BracketMatchChild {
    Winner(BracketMatch),
    Seed(Vec<Player>),
}

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct BracketMatch {
    id: usize,
    winner: Vec<Player>,
    loser_to: Option<usize>,
    winner_to: Option<usize>,
    format: Option<String>,
    scores: Vec<(usize, usize)>,
    children: Vec<BracketMatchChild>,
}

impl BracketMatch {
    fn from_node(node: &BracketNode, teams: &[Vec<Player>]) -> Self {
        let id = node
            .current
            .neighbor(Direction::Left)
            .and_then(|n| n.select(&SELECTORS.match_label).next())
            .and_then(|l| l.inner_html()[1..].parse().ok())
            .unwrap_or(0);

        let winner = resolve_team(&node.current.inner_html(), teams)
            .cloned()
            .unwrap_or_default();

        let scores = node
            .current
            .neighbor(Direction::Down)
            .and_then(|n| {
                if let Some(t) = n.select(&SELECTORS.table).next() {
                    t.select(&SELECTORS.cell).next().map(GridCell)
                } else {
                    Some(n)
                }
            })
            .map(|n| n.inner_html())
            .and_then(|s| {
                let c = PATTERNS.scores.captures(&s)?;
                Some(
                    c[1].split(',')
                        .filter_map(|g| {
                            let mut s = g.split('-');
                            Some((
                                s.next()?.parse::<usize>().ok()?,
                                s.next()?.parse::<usize>().ok()?,
                            ))
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .unwrap_or_default();

        let (loser_to, winner_to) = node
            .current
            .neighbor(Direction::Down)
            .and_then(|n| n.select(&SELECTORS.match_link).next())
            .map(|n| n.inner_html())
            .and_then(|s| {
                let c = PATTERNS.match_link.captures(&s)?;
                let link = c[2].parse::<usize>().ok()?;
                match &c[1] {
                    "Loser" => Some((Some(link), None)),
                    "Winner" => Some((None, Some(link))),
                    _ => None,
                }
            })
            .unwrap_or_default();

        let children = node
            .children
            .iter()
            .map(|c| {
                if c.children.is_empty() {
                    BracketMatchChild::Seed(
                        resolve_team(&c.current.inner_html(), teams)
                            .cloned()
                            .unwrap_or_default(),
                    )
                } else {
                    BracketMatchChild::Winner(BracketMatch::from_node(c, teams))
                }
            })
            .collect();

        Self {
            id,
            winner,
            loser_to,
            winner_to,
            format: None, // XXX Get this working next time a bracket is in progress
            scores,
            children,
        }
    }
}
