use derive_more::{Display, Error as DeriveError, From};
use poise::serenity_prelude::{self as serenity};

#[derive(Debug, Display, DeriveError)]
pub struct PermissionError {
    #[display("{message}")]
    message: String,
}

impl PermissionError {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

#[derive(Debug, DeriveError, Display, From)]
pub enum Error {
    #[display("{_0}")]
    Permission(PermissionError),
    #[display("{_0}")]
    #[from]
    Discord(serenity::Error),
    #[display("{_0}")]
    #[from]
    Database(sqlx::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
