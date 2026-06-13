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
    #[error("malformed input")]
    MalformedInput,
    #[error("unsupported interaction type")]
    UnsupportedInteractionType,
    #[error("unknown command")]
    UnknownCommand,
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Error::Validation => StatusCode::UNAUTHORIZED,
            Error::MalformedInput | Error::UnsupportedInteractionType | Error::UnknownCommand => {
                StatusCode::BAD_REQUEST
            }
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        let mut res = HttpResponse::new(self.status_code());

        let mime = actix_web::mime::TEXT_PLAIN_UTF_8.try_into_value().unwrap();
        res.headers_mut().insert(header::CONTENT_TYPE, mime);

        res.set_body(BoxBody::new(self.to_string()))
    }
}
