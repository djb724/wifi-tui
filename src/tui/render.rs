use std::io::{
    Stdout,
    Write
};
use std::fmt;
use crossterm::{
    QueueableCommand,
    cursor,
    terminal,
    style::{self, Print}
};
use crate::tui::state::{
    AppState,
    InputMode
};
use crate::tui::util;

fn right_pad<S>(len: usize, s: &S) -> String 
where S: fmt::Display
{
    let s_str = format!("{}", s);
    if s_str.len() >= len {
        s_str
    } else {
        let mut padded = String::new();
        padded.push_str(&s_str);
        for _ in 0..(len - s_str.len()) {
            padded.push(' ');
        }
        padded
    }
}

fn render_table(out: &mut Stdout, state: &AppState, height: u16, width: u16) -> Result<(), std::io::Error> {
    let n_entries = state.processed_access_points.len();
    let (start, end) = util::table_visible_range(state)?;

    // table header
    out.queue(style::SetBackgroundColor(style::Color::DarkGrey))?
        .queue(style::SetForegroundColor(style::Color::White))?
        .queue(Print("        SSID                            BSSID                   STRENTGH        FLAGS           MODE                    \r\n"))?
        .queue(style::ResetColor)?;

    // table body
    for i in start..end {
        let ap_props = &state.processed_access_points[i as usize];
        
        out.queue(style::ResetColor)?;

        if i == state.hover_index {
            out.queue(style::SetForegroundColor(style::Color::White))?
                .queue(style::SetBackgroundColor(style::Color::AnsiValue(238)))?
                .queue(Print("    \u{276f} "))?;
        } else {
            out.queue(style::SetForegroundColor(style::Color::Grey))?
                .queue(Print("      "))?;
        }

        if let Some(ap) = util::access_point_under_cursor(state) {
            if ap.ssid == ap_props.ssid {
                out.queue(style::SetForegroundColor(style::Color::Yellow))?;
            }
        }

        if let Some(_) = ap_props.connection {
            out.queue(Print("* "))?;
        } else {
            out.queue(Print("  "))?;
        }

        out.queue(Print(right_pad(32, &ap_props.ssid)))?
            .queue(cursor::MoveToColumn(40))?
            .queue(Print(right_pad(24, &ap_props.hw_address)))?
            .queue(cursor::MoveToColumn(64))?
            .queue(Print(right_pad(16, &ap_props.strength)))?
            .queue(cursor::MoveToColumn(80))?
            .queue(Print(right_pad(16, &ap_props.flags)))?
            .queue(cursor::MoveToColumn(96))?
            .queue(Print(right_pad(24, &ap_props.mode)))?
            .queue(Print("\r\n"))?;
    }

    // table footer
    out.queue(cursor::MoveTo(0, height + 1))?
        .queue(style::SetBackgroundColor(style::Color::Reset))?
        .queue(style::SetForegroundColor(style::Color::Grey))?
        .queue(Print(format!("{} - {} / {}", start + 1, end, n_entries)))?;
    
    Ok(())
}

pub fn render (state: &AppState) -> Result<(), std::io::Error> {
    let mut out = std::io::stdout();
    let (w, h) = terminal::size()?;
    let table_height = util::max_table_height()?;

    out.queue(cursor::MoveTo(0, 0))?
        .queue(terminal::Clear(terminal::ClearType::All))?;

    let max_width = if w < 120 {
        w
    } else {
        120
    };

    render_table(&mut out, &state, table_height, max_width)?;

    // render status
    if let Some(error_message) = &state.error_message {
        out.queue(cursor::MoveTo(0, h - 1))?
            .queue(style::SetBackgroundColor(style::Color::Red))?
            .queue(Print(error_message))?;
    }

    // render text field
    out.queue(cursor::MoveTo(0, h - 2))?
        .queue(style::SetBackgroundColor(style::Color::AnsiValue(16)))?
        .queue(style::SetForegroundColor(style::Color::White))?
        .queue(Print("                                                                                                                        "))? // 120
        .queue(cursor::MoveToColumn(0))?;

    match state.input_mode {
        InputMode::Normal => {
            if !state.filter.is_empty() {
                out.queue(Print("/"))?
                    .queue(Print(&state.filter))?;
            }
            out.queue(cursor::Hide)?;
        },
        InputMode::Password => {
            if let Some(ap) = util::access_point_under_cursor(state) {
                out.queue(Print("Password for "))?
                    .queue(Print(&ap.ssid))?
                    .queue(Print(": "))?;
                if state.show_password {
                    out.queue(Print(&state.password_input))?;
                } else {
                    let mut hidden = String::new();
                    for _ in 0..state.password_input.len() {
                        hidden.push('*');
                    }
                    out.queue(Print(hidden))?;
                }
                out.queue(cursor::Show)?
                    .queue(cursor::SetCursorStyle::BlinkingBar)?;
            }
        },
        InputMode::Filter => {
            out.queue(Print("/"))?
                .queue(Print(&state.filter))?
                .queue(cursor::Show)?
                .queue(cursor::SetCursorStyle::BlinkingBar)?;
        }
    }

    out.queue(style::ResetColor)?;
    out.flush()?;

    Ok(())
}
