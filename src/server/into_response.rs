use actix_web::HttpResponse;
use serenity::all::{CreateInteractionResponse, CreateInteractionResponseMessage};

pub trait IntoResponse: Sized {
    /// Convert this item into the appropriate `CreateInteractionResponse` variant
    fn into_interaction(self) -> CreateInteractionResponse;

    /// Convert this item into a HttpResponse with status 200 and JSON encoding.
    fn into_http(self) -> HttpResponse {
        HttpResponse::Ok().json(self.into_interaction())
    }
}

pub struct Pong;
impl IntoResponse for Pong {
    fn into_interaction(self) -> CreateInteractionResponse {
        CreateInteractionResponse::Pong
    }
}

#[derive(Debug, derive_more::Deref, derive_more::From, derive_more::Into)]
pub struct Message(CreateInteractionResponseMessage);
impl IntoResponse for Message {
    fn into_interaction(self) -> CreateInteractionResponse {
        CreateInteractionResponse::Message(self.into())
    }
}

#[derive(Debug, derive_more::Deref, derive_more::From, derive_more::Into)]
pub struct Defer(CreateInteractionResponseMessage);
impl IntoResponse for Defer {
    fn into_interaction(self) -> CreateInteractionResponse {
        CreateInteractionResponse::Defer(self.into())
    }
}
