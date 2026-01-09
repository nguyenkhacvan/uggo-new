use std::{collections::HashMap, time::Duration};

use ddragon::DDragonClient;
use ratatui::widgets::ListState;
use ugg_types::{
    client_runepage::RunePage,
    client_summoner::Summoner,
    mappings::{Mode, Region, Role},
    matchups::MatchupData,
    overview::Overview,
};
use uggo_config::Config;
use uggo_lol_client::LeagueClient; // Đây là client mới (Async)
use uggo_ugg_api::UggApi;         // Đây là API mới (Async)

use crate::util;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    ChampSelect,
    ModeSelect,
    RoleSelect,
    RegionSelect,
    VersionSelect,
    BuildSelect,
    HelpMenu,
    Logger,
}

pub struct AppContext {
    pub api: UggApi,
    pub client: Option<LeagueClient>, // Option vì client có thể chưa mở
    pub state: State,
    pub ddragon: DDragonClient,
    pub config: Config,
    pub mode: Mode,
    pub role: Role,
    pub region: Region,
    
    // UI States
    pub champ_list: Vec<(String, String)>,
    pub champ_list_state: ListState,
    pub selected_champ: Option<String>,
    pub selected_champ_overview: Option<Overview>,
    pub selected_champ_matchups: Option<MatchupData>,
    pub champ_by_key: HashMap<String, String>,
    
    // Misc
    pub version: String,
    pub show_left_pane: bool,
    
    // Scroll states for popups
    pub mode_scroll_state: ListState,
    pub region_scroll_state: ListState,
    pub role_scroll_state: ListState,
    pub version_scroll_state: ListState,
    pub build_scroll_state: ListState,
    
    pub logger_state: tui_logger::TuiWidgetState,
    
    #[cfg(debug_assertions)]
    pub render_duration: Duration,
}

impl AppContext {
    // Chuyển thành ASYNC function
    pub async fn new() -> anyhow::Result<Self> {
        let config = Config::new()?;
        let mut api = UggApi::new();
        
        // Fetch version ngay khi khởi động (Async)
        let version = api.fetch_current_version().await?;

        let ddragon = DDragonClient::new(version.as_str())?;
        let champs = ddragon.champions()?;
        let mut champ_data = champs.iter().map(|c| (c.name.clone(), c.key.clone())).collect::<Vec<_>>();
        champ_data.sort_by(|a, b| a.0.cmp(&b.0));

        let champ_by_key = champs.iter().map(|c| (c.key.clone(), c.name.clone())).collect();

        // Thử kết nối tới League Client (Async init nếu cần, nhưng lcc::new vẫn sync, 
        // tuy nhiên việc tìm process giờ dùng sysinfo nên rất nhanh)
        let client = match uggo_lol_client::lcc::LeagueClientConnector::parse_lockfile() {
            Ok(lockfile) => LeagueClient::new(lockfile).ok(),
            Err(_) => None,
        };

        Ok(Self {
            api,
            client,
            state: State::ChampSelect,
            ddragon,
            config,
            mode: Mode::Normal,
            role: Role::Automatic,
            region: Region::World,
            champ_list: champ_data,
            champ_list_state: ListState::default(),
            selected_champ: None,
            selected_champ_overview: None,
            selected_champ_matchups: None,
            champ_by_key,
            version,
            show_left_pane: true,
            mode_scroll_state: ListState::default(),
            region_scroll_state: ListState::default(),
            role_scroll_state: ListState::default(),
            version_scroll_state: ListState::default(),
            build_scroll_state: ListState::default(),
            logger_state: tui_logger::TuiWidgetState::default(),
            #[cfg(debug_assertions)]
            render_duration: Duration::default(),
        })
    }
    
    // Các hàm update data cũng cần chuyển sang async nếu chúng gọi API
    pub async fn select_champion(&mut self, champ_name: &str) -> anyhow::Result<()> {
        self.selected_champ = Some(champ_name.to_string());
        
        // Async call
        let overview = self.api.get_overview(
            champ_name, 
            self.mode, 
            self.role, 
            self.region
        ).await?;
        
        let matchups = self.api.get_matchups(
             champ_name,
             self.mode,
             self.role,
             self.region
        ).await?;

        self.selected_champ_overview = Some(*overview);
        self.selected_champ_matchups = Some(*matchups);
        
        Ok(())
    }
    
    // Hàm này giữ nguyên được vì chỉ set duration
    #[cfg(debug_assertions)]
    pub fn set_render_duration(&mut self, duration: Duration) {
        self.render_duration = duration;
    }
}
