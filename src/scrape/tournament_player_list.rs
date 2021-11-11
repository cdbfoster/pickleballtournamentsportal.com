use std::time::Duration;

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

#[derive(Clone, Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub id: usize,
    pub first_name: String,
    pub last_name: String,
    pub nick_name: Option<String>,
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

                        let player_selector =
                            Selector::parse("#menuPlayerList .playerlist-wrap table tr").unwrap();
                        let player_name_selector = Selector::parse(".col-player > a").unwrap();
                        let player_from_selector = Selector::parse(".col-from").unwrap();

                        let player_id_pattern = Regex::new(r"&amp;id=(\d+)").unwrap();
                        let player_name_pattern =
                            Regex::new(r"<span>([^<]+)</span>, ([^(<]+)(?:\(([^)]+)\))?").unwrap();

                        Ok(tournament_page_html
                            .select(&player_selector)
                            .map(|player_row| {
                                let name_element =
                                    player_row.select(&player_name_selector).next().unwrap();
                                let name_html = name_element.html();
                                let id = player_id_pattern.captures(&name_html).unwrap()[1]
                                    .parse()
                                    .unwrap();
                                let name_matches =
                                    player_name_pattern.captures(&name_html).unwrap();

                                let from_element =
                                    player_row.select(&player_from_selector).next().unwrap();

                                Player {
                                    id,
                                    first_name: name_matches[2].to_owned(),
                                    last_name: name_matches[1].to_owned(),
                                    nick_name: name_matches.get(3).map(|n| n.as_str().to_owned()),
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
