pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("a database operation failed")]
    Rusqlite(#[from] rusqlite::Error),
    #[error("creating or joining a blocking task failed")]
    Blocking(#[from] tokio::task::JoinError),
}
