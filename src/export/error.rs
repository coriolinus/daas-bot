use tokio::sync::mpsc;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("working with export database")]
    ExportDatabase(#[from] crate::sql::Error),
    #[error("calling Discord http API")]
    Http(#[source] serenity::Error),
    #[error("channel '{channel}' was unexpectedly closed; send failed")]
    SendFailed {
        channel: &'static str,
        #[source]
        source: Box<dyn std::error::Error + Send>,
    },
    #[error("{context}")]
    Io {
        context: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to await reading from filesystem")]
    ReadFilesystemJoinFailure(#[source] tokio::task::JoinError),
}

impl Error {
    pub(crate) fn send_failed<T>(
        channel: &'static str,
    ) -> impl FnOnce(mpsc::error::SendError<T>) -> Self
    where
        T: 'static + Send,
    {
        move |send_err| Self::SendFailed {
            channel,
            source: Box::new(send_err),
        }
    }

    pub(crate) fn io(context: &'static str) -> impl FnOnce(std::io::Error) -> Self {
        move |source| Self::Io { context, source }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
