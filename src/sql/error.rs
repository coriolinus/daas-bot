pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{context}")]
    Rusqlite {
        context: &'static str,
        #[source]
        source: rusqlite::Error,
    },
    #[error("creating or joining a blocking task failed")]
    Blocking(#[from] tokio::task::JoinError),
}

impl Error {
    pub(crate) fn sql(context: &'static str) -> impl FnOnce(rusqlite::Error) -> Self {
        move |source| Self::Rusqlite { context, source }
    }
}
