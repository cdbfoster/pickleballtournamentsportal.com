use crate::util::cache::{Cache, CacheMap, PageCache};

use self::tournament_event_group_list::EventGroupList;
use self::tournament_info::Info;
use self::tournament_list::TournamentList;
use self::tournament_player_list::PlayerList;
use self::tournament_schedule::Schedule;

pub use self::result::{scrape_result, ScrapeResult};

pub mod tournament_event_group_list;
pub mod tournament_info;
pub mod tournament_list;
pub mod tournament_player_list;
pub mod tournament_schedule;

mod result;

#[derive(Default)]
pub struct ScrapeCache {
    pub pages: PageCache,
    pub tournament_list: Cache<TournamentList>,
    pub tournament_event_list: CacheMap<usize, EventGroupList>,
    pub tournament_info: CacheMap<usize, Info>,
    pub tournament_player_list: CacheMap<usize, PlayerList>,
    pub tournament_schedule: CacheMap<usize, Schedule>,
}

const TOURNAMENT_LIST_REFRESH: u64 = 3 * 60 * 60;
const TOURNAMENT_EVENT_BRACKET_LIST_REFRESH: u64 = 2 * 60 * 60;
const TOURNAMENT_EVENT_BRACKET_PAGE_REFRESH: u64 = 15 * 60;
const TOURNAMENT_EVENT_PLAYER_LIST_REFRESH: u64 = 3 * 60 * 60;
const TOURNAMENT_INFO_REFRESH: u64 = 3 * 60 * 60;
const TOURNAMENT_PAGE_REFRESH: u64 = 60 * 60;
const TOURNAMENT_PLAYER_LIST_REFRESH: u64 = 3 * 60 * 60;
const TOURNAMENT_SCHEDULE_REFRESH: u64 = 2 * 60 * 60;
