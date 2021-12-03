use std::time::Duration;

use regex::Regex;
use scraper::{Html, Selector};

use crate::client::Client;
use crate::scrape::{ScrapeCache, ScrapeResult, TOURNAMENT_INFO_REFRESH, TOURNAMENT_PAGE_REFRESH};
use crate::util::cache::{CacheGuard, CacheMapGuard};
use crate::util::guard_stack::GuardStack;

pub type Info = Vec<(String, String)>;

pub type InfoGuard<'a> =
    GuardStack<'a, (CacheMapGuard<'a, usize, Info>, CacheGuard<'a, Info>), Info>;

pub async fn tournament_info<'a>(
    tournament_id: usize,
    client: &'a Client<'a>,
    cache: &'a ScrapeCache,
) -> ScrapeResult<InfoGuard<'a>> {
    cache
        .tournament_info
        .get(tournament_id)
        .await
        .try_push_guard_async(|info_cache| async move {
            info_cache
                .retrieve_or_update(Duration::from_secs(TOURNAMENT_INFO_REFRESH), || async {
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

                    let nav_item_selector =
                        Selector::parse(".nav > .nav-item > .nav-link").unwrap();

                    let href_pattern = Regex::new(r#"href="([^"]+)""#).unwrap();

                    Ok(tournament_page
                        .select(&nav_item_selector)
                        .filter_map(|l| {
                            let html = l.html();
                            href_pattern
                                .captures(&html)
                                .map(|c| (c[1].to_owned(), l.inner_html()))
                        })
                        .filter(|(l, _)| l.starts_with('#'))
                        .filter(|(l, _)| {
                            [
                                "#menuSchedule",
                                "#menuPlayerList",
                                "#menuEventList",
                                "#menuPlayersNeedingPartners",
                                "#menuFindPlayer",
                            ]
                            .iter()
                            .all(|t| l != t)
                        })
                        .filter_map(|(l, n)| {
                            let selector = Selector::parse(&l).unwrap();
                            tournament_page
                                .select(&selector)
                                .next()
                                .map(|e| (n, e.inner_html()))
                        })
                        .collect())
                })
                .await
        })
        .await
}
