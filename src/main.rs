mod tui;
mod network_manager;
use tui::result::AppResult;
use tui::AccessPointTui;
use clap::Parser;

#[tokio::main]
async fn main() {
    // TODO: parse args for other functionalities

    let mut app = match AccessPointTui::new().await {
        Ok(app) => app,
        Err(e) => {
            eprintln!("{}", e);
            return;
        }
    };
    let result = app.run_app().await;

    match result {
        AppResult::Cancelled => println!("Cancelled by user input"),
        AppResult::Error(e) => println!("{}", e),
        AppResult::Success(s) => println!("Wireless device {} successfully connected to {} ({})", &s.wireless_interface, &s.ssid, &s.uuid)
    };
}
