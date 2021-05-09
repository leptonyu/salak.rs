use crate::*;
use std::convert::TryInto;

impl_from_environment!(Level, LevelFilter);

impl TryInto<LevelFilter> for Property {
    type Error = PropertyError;
    fn try_into(self) -> Result<LevelFilter, PropertyError> {
        match self {
            Property::Str(du) => match &du.to_lowercase()[..] {
                "off" => Ok(LevelFilter::Off),
                "trace" => Ok(LevelFilter::Trace),
                "debug" => Ok(LevelFilter::Debug),
                "info" => Ok(LevelFilter::Info),
                "warn" => Ok(LevelFilter::Warn),
                "error" => Ok(LevelFilter::Error),
                _ => Err(PropertyError::parse_failed("Invalid LevelFilter")),
            },
            _ => Err(PropertyError::parse_failed(
                "LevelFilter only support string",
            )),
        }
    }
}

impl TryInto<Level> for Property {
    type Error = PropertyError;
    fn try_into(self) -> Result<Level, PropertyError> {
        match self {
            Property::Str(du) => match &du.to_lowercase()[..] {
                "trace" => Ok(Level::Trace),
                "debug" => Ok(Level::Debug),
                "info" => Ok(Level::Info),
                "warn" => Ok(Level::Warn),
                "error" => Ok(Level::Error),
                _ => Err(PropertyError::parse_failed("Invalid LevelFilter")),
            },
            _ => Err(PropertyError::parse_failed(
                "LevelFilter only support string",
            )),
        }
    }
}
