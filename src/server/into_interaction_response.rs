use serenity::all::{CreateInteractionResponse, CreateInteractionResponseMessage};

pub trait IntoInteractionResponse {
    /// Convert this item into the appropriate `CreateInteractionResponse` variant
    fn into_interaction_response(self) -> CreateInteractionResponse;
}

pub struct Pong;
impl IntoInteractionResponse for Pong {
    fn into_interaction_response(self) -> CreateInteractionResponse {
        CreateInteractionResponse::Pong
    }
}

#[derive(Debug, derive_more::Deref, derive_more::From, derive_more::Into)]
pub struct Message(CreateInteractionResponseMessage);
impl IntoInteractionResponse for Message {
    fn into_interaction_response(self) -> CreateInteractionResponse {
        CreateInteractionResponse::Message(self.into())
    }
}

#[derive(Debug, derive_more::Deref, derive_more::From, derive_more::Into)]
pub struct Defer(CreateInteractionResponseMessage);
impl IntoInteractionResponse for Defer {
    fn into_interaction_response(self) -> CreateInteractionResponse {
        CreateInteractionResponse::Defer(self.into())
    }
}
