use derive_more::{Display, Error as DeriveError, From};
use poise::serenity_prelude::{self as serenity};

macro_rules! impl_error {
    ($error:ident) => {
        #[derive(Debug, Display, DeriveError)]
        pub struct $error {
            #[display("{message}")]
            message: String,
        }

        impl $error {
            pub fn new(message: String) -> Self {
                Self { message }
            }
        }
    };
}

impl_error!(PermissionError);
impl_error!(InvalidChannelError);
impl_error!(InvalidGuildError);
impl_error!(InvalidRoleError);

#[derive(Debug, DeriveError, Display, From)]
pub enum Error {
    #[display("{_0}")]
    Permission(PermissionError),
    #[display("{_0}")]
    InvalidChannel(InvalidChannelError),
    #[display("{_0}")]
    InvalidGuild(InvalidGuildError),
    #[display("{_0}")]
    InvalidRole(InvalidRoleError),
    #[display("{_0}")]
    #[from]
    Discord(serenity::Error),
    #[display("{_0}")]
    #[from]
    Database(sqlx::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
