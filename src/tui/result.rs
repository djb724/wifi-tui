use super::error::AppError;

#[derive(Debug, Clone)] 
pub struct SuccessResult {
    pub ssid: String,
    pub wireless_interface: String,
    pub uuid: String
}

#[derive(Debug, Clone)] 
pub enum AppResult {
    Success(SuccessResult),
    Cancelled,
    Error(AppError)
}
