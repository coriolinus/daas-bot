use std::fmt::Write as _;

use actix_web::{
    HttpResponse, ResponseError,
    body::BoxBody,
    http::{
        StatusCode,
        header::{self, TryIntoHeaderValue as _},
    },
};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("security request header validation failed")]
    Validation,
    #[error("malformed input: {0}")]
    MalformedInput(&'static str),
    #[error("unsupported interaction type")]
    UnsupportedInteractionType,
    #[error("unknown command")]
    UnknownCommand,
    #[error("interacting with the local database")]
    LocalDb(#[from] crate::sql::Error),
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Error::Validation => StatusCode::UNAUTHORIZED,
            Error::MalformedInput(_)
            | Error::UnsupportedInteractionType
            | Error::UnknownCommand => StatusCode::BAD_REQUEST,
            Error::LocalDb(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        let mut res = HttpResponse::new(self.status_code());

        let mime = actix_web::mime::TEXT_PLAIN_UTF_8.try_into_value().unwrap();
        res.headers_mut().insert(header::CONTENT_TYPE, mime);

        let mut err_messages = String::new();
        let mut err: Option<&dyn std::error::Error> = Some(&self);
        while let Some(head) = err {
            let _ = writeln!(&mut err_messages, "{head}");
            err = head.source();
        }

        res.set_body(BoxBody::new(err_messages))
    }
}
