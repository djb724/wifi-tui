pub mod error;
pub mod state;
pub mod result;
pub mod render;
mod util;

use error::AppError;
use state::{
    AppState,
    AccessPointSort,
    InputMode
};
use crate::{network_manager::{NetworkManager, NmError, access_point::AccessPoint}, tui::result::SuccessResult};
use crossterm::event::{
    Event,
    KeyEvent,
    KeyCode,
    KeyModifiers,
};
use result::AppResult;
use render::render;

const MAX_SCROLLOFF: u16 = 3;


fn enter_app() -> Result<(), std::io::Error> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::cursor::SavePosition,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::cursor::Hide
    )?;
    Ok(())
}

fn exit_app() -> Result<(), std::io::Error> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show,
        crossterm::cursor::RestorePosition,
        crossterm::cursor::SetCursorStyle::SteadyBlock
    )?;
    Ok(())
}

pub struct AccessPointTui {
    state: AppState,
    network_manager: NetworkManager,
    should_close: Option<AppResult>
}

impl AccessPointTui {
    fn down(&mut self) -> Result<(), std::io::Error> {
        if self.state.processed_access_points.len() == 0 {
            return Ok(());
        }
        let n_entries = self.state.processed_access_points.len() as u16;
        let (_, end) = util::table_visible_range(&self.state)?;
        if self.state.hover_index < (self.state.processed_access_points.len() - 1) as u16 {
            self.state.hover_index += 1;
        }
        if self.state.hover_index > end - MAX_SCROLLOFF {
            if end < n_entries {
                self.state.scroll_offset += 1;
            }
        }
        Ok(())
    }

    fn up(&mut self) -> Result<(), std::io::Error> {
        let (start, _) = util::table_visible_range(&self.state)?;
        if self.state.hover_index > 0 {
            self.state.hover_index -= 1;
        }
        if self.state.hover_index < start + MAX_SCROLLOFF - 1 {
            if self.state.scroll_offset > 0 {
                self.state.scroll_offset -= 1;
            }
        }

        Ok(())
    }

    fn update_processed(&mut self) {
        // filter
        let mut processed = if self.state.filter.is_empty() {
            self.state.access_points.iter()
                .map(|(_, ap)| ap.clone())
                .collect::<Vec<AccessPoint>>()
        } else {
            self.state.access_points.iter()
                .filter(|(_, ap)| ap.ssid.to_lowercase().contains(&self.state.filter.to_lowercase()))
                .map(|(_, ap)| ap.clone())
                .collect::<Vec<AccessPoint>>()
        };

        // sort
        match &self.state.sort {
            AccessPointSort::Strength => {
                processed.sort_by(|a, b| b.strength.cmp(&a.strength));
            },
            AccessPointSort::StrengthReverse => {
                processed.sort_by(|a, b| a.strength.cmp(&b.strength));
            },
            AccessPointSort::Alpha => {
                processed.sort_by(|a, b| a.ssid.cmp(&b.ssid));
            },
            AccessPointSort::AlphaReverse => {
                processed.sort_by(|a, b| b.ssid.cmp(&a.ssid));
            }
        }

        self.state.processed_access_points = processed;
        if self.state.hover_index >= self.state.processed_access_points.len() as u16 {
            self.state.hover_index = if self.state.processed_access_points.len() == 0 {
                0
            } else {
                (self.state.processed_access_points.len() - 1) as u16
            };
        }
        self.state.scroll_offset = 0;
    }

    async fn handle_access_point_selection(&mut self) -> Result<(), AppError> {
        const WPA2_FLAG: u32 = 0x01;
        if let Some(access_point) = util::access_point_under_cursor(&self.state) {
            // use saved connection if it exists
            if let Some(connection) = &access_point.connection {
                let connection = self.network_manager.restore_connection(&connection).await?;
                self.should_close = Some(AppResult::Success(SuccessResult{
                    ssid: access_point.ssid.clone(),
                    wireless_interface: self.network_manager.wireless_interface.clone(),
                    uuid: connection.uuid.clone()
                }));
            } else if access_point.flags & WPA2_FLAG == WPA2_FLAG {
                self.state.input_mode = InputMode::Password;
            } else {
                self.network_manager.connect_open(&access_point).await?;
            }
        };

        Ok(())
    }
    
    async fn handle_secure_connect(&mut self) -> Result<(), AppError> {
        if let Some(access_point) = util::access_point_under_cursor(&self.state) {
            match self.network_manager.connect_secure(access_point, &self.state.password_input).await {
                Ok(saved_connection) => self.should_close = Some(AppResult::Success(SuccessResult{
                    ssid: saved_connection.ssid.clone(),
                    wireless_interface: self.network_manager.wireless_interface.clone(),
                    uuid: saved_connection.uuid.clone()
                })),
                Err(NmError::ConnectionFailed) => {
                    self.state.error_message = Some(format!("Unable to connect to connect to {}.", access_point.ssid))
                },
                Err(e) => return Err(AppError::Nm(e))
            }
        };
        self.state.input_mode = InputMode::Normal;
        self.state.password_input = String::from("");
        Ok(())
    }

    async fn handle_key(&mut self, ke: KeyEvent) -> Result<(), AppError> {
        // Exit on ^C press
        if ke.modifiers.contains(KeyModifiers::CONTROL) && ke.code == KeyCode::Char('c') {
            self.should_close = Some(AppResult::Cancelled);
            return Ok(());
        }

        match &mut self.state.input_mode {
            InputMode::Normal => {
                if ke.code == KeyCode::Char('j') {
                    self.down().map_err(AppError::Io)?;
                } else if ke.code == KeyCode::Down {
                    self.down().map_err(AppError::Io)?;
                } else if ke.code == KeyCode::Char('k') {
                    self.up().map_err(AppError::Io)?;
                } else if ke.code == KeyCode::Up {
                    self.up().map_err(AppError::Io)?;
                } else if ke.code == KeyCode::Char('n') && ke.modifiers.contains(KeyModifiers::CONTROL) {
                    self.down().map_err(AppError::Io)?;
                } else if ke.code == KeyCode::Char('p') && ke.modifiers.contains(KeyModifiers::CONTROL) {
                    self.up().map_err(AppError::Io)?;
                } else if ke.code == KeyCode::Char('q') {
                    self.should_close = Some(AppResult::Cancelled);
                } else if ke.code == KeyCode::Char('/') {
                    self.state.input_mode = InputMode::Filter;
                } else if ke.code == KeyCode::Enter {
                    self.handle_access_point_selection().await?;
                } else if ke.code == KeyCode::Char('a') {
                    self.state.sort = AccessPointSort::Alpha;
                    self.update_processed();
                } else if ke.code == KeyCode::Char('A') {
                    self.state.sort = AccessPointSort::AlphaReverse;
                    self.update_processed();
                } else if ke.code == KeyCode::Char('s') {
                    self.state.sort = AccessPointSort::Strength;
                    self.update_processed();
                } else if ke.code == KeyCode::Char('S') {
                    self.state.sort = AccessPointSort::StrengthReverse;
                    self.update_processed();
                } else if ke.code == KeyCode::Esc {
                    self.state.filter.clear();
                    self.update_processed();
                }
            },

            InputMode::Password => {
                if ke.modifiers.contains(KeyModifiers::CONTROL) && ke.code == KeyCode::Char('s') {
                    self.state.show_password = !self.state.show_password;
                } else if let KeyCode::Char(c) = ke.code {
                    self.state.password_input.push(c);
                } else if ke.code == KeyCode::Backspace {
                    self.state.password_input.pop();
                } else if ke.code == KeyCode::Esc {
                    self.state.input_mode = InputMode::Normal;
                } else if ke.code == KeyCode::Enter {
                    self.handle_secure_connect().await?;
                }
            },

            InputMode::Filter => {
                if ke.code ==KeyCode::Down {
                    self.down().map_err(AppError::Io)?;
                } else if ke.code == KeyCode::Up {
                    self.up().map_err(AppError::Io)?;
                } else if ke.code == KeyCode::Char('n') && ke.modifiers.contains(KeyModifiers::CONTROL) {
                    self.down().map_err(AppError::Io)?;
                } else if ke.code == KeyCode::Char('p') && ke.modifiers.contains(KeyModifiers::CONTROL) {
                    self.up().map_err(AppError::Io)?;
                } else if let KeyCode::Char(c) = ke.code {
                    self.state.filter.push(c);
                    self.update_processed();
                } else if ke.code == KeyCode::Backspace {
                    self.state.filter.pop();
                    self.update_processed();
                } else if ke.code == KeyCode::Enter {
                    self.state.input_mode = InputMode::Normal;
                } else if ke.code == KeyCode::Esc {
                    self.state.filter.clear();
                    self.state.input_mode = InputMode::Normal;
                }
            }
        }

        Ok(())
    }

    pub async fn handle_event(&mut self) -> Result<(), AppError> {
        match crossterm::event::read() {
            Ok(Event::Key(ke)) => self.handle_key(ke).await,
            Ok(_) => Ok(()), // no need for state changes
            Err(e) => Err(AppError::Io(e))
        }
    }

    pub async fn new() -> Result<Self, AppError> {
        let network_manager = match NetworkManager::new().await {
            Ok(nm) => nm,
            Err(e) => return Err(e.into())
        };

        let access_points = match network_manager.get_access_points().await {
            Ok(aps) => aps,
            Err(e) => return Err(AppError::Nm(e))
        };

        Ok(Self {
            state: AppState {
                hover_index: 0,
                scroll_offset: 0,
                access_points: access_points,
                processed_access_points: Vec::<AccessPoint>::new(),
                password_input: String::from(""),
                show_password: false,
                filter: String::from(""),
                sort: AccessPointSort::Strength,
                input_mode: InputMode::Normal,
                error_message: None
            },
            network_manager,
            should_close: None
        })
    }

    pub async fn run_app(&mut self) -> AppResult {

        self.update_processed();

        if let Err(e) = enter_app() {
            exit_app().unwrap();
            return AppResult::Error(AppError::Io(e));
        }

        let result = loop {
            if let Err(e) = render(&self.state) {
                break AppResult::Error(AppError::Io(e));
            };

            if let Err(e) = self.handle_event().await {
                break AppResult::Error(e);
            };

            if let Some(res) = &self.should_close {
                break res.clone();
            }
        };

        // exit app
        if let Err(e) = exit_app() {
            return AppResult::Error(AppError::Io(e));
        }

        result
    }
}

