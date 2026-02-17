use crate::tui::result::AppResult;
use crate::network_manager::access_point::AccessPoint;
use std::collections::HashMap;

pub enum AccessPointSort {
    Strength,
    StrengthReverse,
    Alpha,
    AlphaReverse
}

pub enum InputMode {
    Normal,
    Password,
    Filter
}

pub struct AppState {
    pub hover_index: u16,
    pub scroll_offset: u16,
    pub access_points: HashMap<String, AccessPoint>,
    pub processed_access_points: Vec<AccessPoint>,
    pub password_input: String,
    pub show_password: bool,
    pub filter: String,
    pub sort: AccessPointSort,
    pub input_mode: InputMode,
    pub error_message: Option<String>
}
