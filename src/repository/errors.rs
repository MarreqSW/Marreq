use diesel::result::Error as DieselError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepoError {
    #[error("not found")]
    NotFound,
    #[error("database error: {0}")]
    Db(#[from] DieselError),
    #[error("pool error: {0}")]
    Pool(String),
    #[error("bad input: {0}")]
    BadInput(String),
    #[error("unauthorized")]
    Unauthorized,
}
