use std::fmt;

use crate::network_manager::NmError;

#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    Dbus(zbus::Error),
    Nm(NmError),
    Implementation(&'static str),
    ExpectationFailed(&'static str)
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::Io(ioe) => write!(f, "Io error: {:?}", ioe),
            AppError::Dbus(de) => write!(f, "Dbus error: {:?}", de),
            AppError::Nm(e) => write!(f, "Dbus error: {:?}", e),
            AppError::Implementation(e) => write!(f, "Implemantation error: {}", e),
            AppError::ExpectationFailed(e) => write!(f, "Expectation failed: {}", e)
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(ioe: std::io::Error) -> Self {
        AppError::Io(ioe)
    }
}

impl From<zbus::Error> for AppError {
    fn from(de: zbus::Error) -> Self {
        AppError::Dbus(de)
    }
}

impl From<NmError> for AppError {
    fn from(ne: NmError) -> Self {
        AppError::Nm(ne)
    }
}

impl Clone for AppError {
    fn clone(&self) -> Self {
        match self {
            AppError::Io(ioe) => AppError::Io(std::io::Error::new(ioe.kind(), format!("{:?}", ioe))),
            AppError::Dbus(de) => AppError::Dbus(de.clone()),
            AppError::Nm(ne) => AppError::Nm(ne.clone()),
            AppError::Implementation(e) => AppError::Implementation(e),
            AppError::ExpectationFailed(e) => AppError::ExpectationFailed(e)
        }
    }
}

impl std::error::Error for AppError {}
