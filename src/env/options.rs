use std::env;

use thiserror::Error;
use url::Url;

use crate::{logging::LogLevel};

/////////////////////////////////////////////////////
// EnvError
/////////////////////////////////////////////////////
#[derive(Debug, Clone, Error)]
pub enum EnvError {
    #[error("LOG_LEVEL contains invalid data (must be \"debug\", \"info\", \"warning\" or \"error\". Received: {log_level}")]
    InvalidLogLevel { log_level: String },
    #[error("Expected PORT to be a 16 bit unsigned integer, got: {port}")]
    InvalidPort { port: String },
    #[error("FLARESOLVERR_URL is set to an invalid url \"{url}\", error: {error}")]
    InvalidFlaresolverrUrl { url: String, error: url::ParseError },
    #[error("BYPARR_URL is set to an invalid url \"{url}\", error: {error}")]
    InvalidByparrUrl { url: String, error: url::ParseError },
}

/////////////////////////////////////////////////////
// Options
/////////////////////////////////////////////////////
#[derive(Debug, Clone)]
pub struct EnvOptions {
    pub log_level: LogLevel,
    pub port: u16,
    pub flaresolverr_url: Option<Url>,
    pub byparr_url: Option<Url>,
}

impl Default for EnvOptions {
    fn default() -> Self {
        Self {
            log_level: LogLevel::Info,
            port: 8091,
            flaresolverr_url: None,
            byparr_url: None,
        }
    }
}

impl EnvOptions {
    pub fn from_env() -> Result<Self, EnvError> {
        let default = EnvOptions::default();

        let log_level = Self::parse_log_level()?;
        let port = Self::parse_port()?;
        let flaresolverr_url = Self::parse_flaresolverr_url()?;
        let byparr_url = Self::parse_byparr_url()?;

        if flaresolverr_url.is_none() && byparr_url.is_none() {
            warning!("No proxy url set up, set FLARESOLVERR_URL or BYPARR_URL.");
        }

        Ok(Self {
            log_level: log_level.unwrap_or(default.log_level),
            port: port.unwrap_or(default.port),
            flaresolverr_url: flaresolverr_url,
            byparr_url: byparr_url,
        })
    }

    fn parse_log_level() -> Result<Option<LogLevel>, EnvError> {
        let Ok(log_level) = env::var("LOG_LEVEL") else {
            return Ok(None);
        };

        match log_level.to_lowercase().as_str() {
            "debug" | "trace" => Ok(Some(LogLevel::Trace)),
            "info" => Ok(Some(LogLevel::Info)),
            "warning" => Ok(Some(LogLevel::Warn)),
            "error" => Ok(Some(LogLevel::Error)),
            _ => Err(EnvError::InvalidLogLevel { log_level: log_level }),
        }
    }

    fn parse_port() -> Result<Option<u16>, EnvError> {
        let Ok(port) = env::var("PORT") else {
            return Ok(None);
        };

        let port = port.parse::<u16>().map_err(|_error| EnvError::InvalidPort { port: port })?;

        Ok(Some(port))
    }

    fn parse_flaresolverr_url() -> Result<Option<Url>, EnvError> {
        let Ok(flaresolverr_url) = env::var("FLARESOLVERR_URL") else {
            return Ok(None);
        };

        let flaresolverr_url = Url::parse(flaresolverr_url.as_str()).map_err(|error| {
            return EnvError::InvalidFlaresolverrUrl {
                url: flaresolverr_url,
                error: error,
            };
        })?;

        Ok(Some(flaresolverr_url))
    }

    fn parse_byparr_url() -> Result<Option<Url>, EnvError> {
        let Ok(byparr_url) = env::var("BYPARR_URL") else {
            return Ok(None);
        };

        let byparr_url = Url::parse(byparr_url.as_str()).map_err(|error| {
            return EnvError::InvalidByparrUrl {
                url: byparr_url,
                error: error,
            };
        })?;

        Ok(Some(byparr_url))
    }
}
