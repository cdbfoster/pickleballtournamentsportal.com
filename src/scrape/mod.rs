use crate::util::cache::{Cache, CacheMap, PageCache};

use self::tournament_event_group_list::EventGroupList;
use self::tournament_info::Info;
use self::tournament_list::TournamentList;
use self::tournament_player_list::PlayerList;
use self::tournament_schedule::Schedule;

pub use self::result::{scrape_result, ScrapeError, ScrapeResult};

pub mod tournament_event_group_list;
pub mod tournament_info;
pub mod tournament_list;
pub mod tournament_player_list;
pub mod tournament_schedule;

mod result;

#[derive(Default)]
pub struct ScrapeCache {
    pages: PageCache,
    tournament_list: Cache<TournamentList>,
    tournament_event_list: CacheMap<usize, EventGroupList>,
    tournament_info: CacheMap<usize, Info>,
    tournament_player_list: CacheMap<usize, PlayerList>,
    tournament_schedule: CacheMap<usize, Schedule>,
}

const TOURNAMENT_LIST_REFRESH: u64 = 3 * 60 * 60;
const TOURNAMENT_EVENT_BRACKET_LIST_REFRESH: u64 = 2 * 60 * 60;
const TOURNAMENT_EVENT_BRACKET_PAGE_REFRESH: u64 = 15 * 60;
const TOURNAMENT_EVENT_PLAYER_LIST_REFRESH: u64 = 3 * 60 * 60;
const TOURNAMENT_INFO_REFRESH: u64 = 3 * 60 * 60;
const TOURNAMENT_PAGE_REFRESH: u64 = 60 * 60;
const TOURNAMENT_PLAYER_LIST_REFRESH: u64 = 3 * 60 * 60;
const TOURNAMENT_SCHEDULE_REFRESH: u64 = 2 * 60 * 60;
