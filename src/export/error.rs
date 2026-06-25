#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("working with export database")]
    ExportDatabase(#[from] crate::sql::Error),
    #[error("calling Discord http API")]
    Http(#[source] serenity::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
