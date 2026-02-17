use crossterm::terminal;
use crate::tui::state::AppState;
use crate::network_manager::access_point::AccessPoint;

pub fn max_table_height() -> Result<u16, std::io::Error> {
    let (_, h) = terminal::size()?;
    return Ok(h - 4);
}

pub fn table_visible_range(state: &AppState) -> Result<(u16, u16), std::io::Error> {
    let max_table_height = max_table_height()?;
    let n_entries = state.processed_access_points.len();
    let start = state.scroll_offset;
    let end = if start + max_table_height > n_entries as u16 {
        n_entries as u16
    } else {
        start + max_table_height
    };

    Ok((start, end))
}

pub fn access_point_under_cursor(state: &AppState) -> Option<&AccessPoint> {
    let i = state.hover_index as usize;
    state.processed_access_points.get(i)
}

