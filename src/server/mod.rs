mod error;
mod handlers;
mod into_interaction_response;
mod validate;

use std::sync::Arc;

use actix_web::{App, HttpResponse, HttpServer, post, web};
use anyhow::Context as _;
use serenity::all::{Http, Interaction, Verifier};

pub use self::into_interaction_response::{Defer, Message, Pong};
use self::{
    error::{Error, Result},
    into_interaction_response::IntoInteractionResponse,
    validate::{XSignatureEd25519, XSignatureTimestamp},
};
use crate::cli::Args;

#[derive(derive_more::Debug, Clone)]
struct AppState {
    config: Arc<Args>,
    #[expect(unused)]
    http: Arc<Http>,
    #[debug("<Verifier instance>")]
    verifier: Verifier,
}

impl AppState {
    pub fn new(config: impl Into<Arc<Args>>, http: impl Into<Arc<Http>>) -> anyhow::Result<Self> {
        let config = config.into();
        let http = http.into();

        let mut public_key = [0; 32];
        hex::decode_to_slice(&config.public_key, &mut public_key)
            .context("hex-decoding public key")?;
        let verifier =
            Verifier::try_new(public_key).context("cryptographically parsing public key")?;

        Ok(Self {
            config,
            http,
            verifier,
        })
    }
}

#[post("/")]
async fn handle_interaction(
    data: web::Data<AppState>,
    signature: web::Header<XSignatureEd25519>,
    timestamp: web::Header<XSignatureTimestamp>,
    body: web::Bytes,
) -> Result<HttpResponse> {
    /// takes something which can become an interaction response and wraps it up appropriately
    fn into_http_response<T: IntoInteractionResponse>(response: Result<T>) -> Result<HttpResponse> {
        response.map(|into_interaction| {
            HttpResponse::Ok().json(into_interaction.into_interaction_response())
        })
    }

    data.verifier
        .verify(&signature, &timestamp, &body)
        .map_err(|_| Error::Validation)?;

    let interaction =
        serde_json::from_slice::<Interaction>(&body).map_err(|_| Error::MalformedInput)?;

    match interaction {
        Interaction::Ping(interaction) => into_http_response(handlers::ping(interaction).await),
        Interaction::Command(interaction) => {
            match interaction
                .data
                .options
                .first()
                .map(|option| option.name.as_ref())
            {
                Some("cleanup") => into_http_response(handlers::cleanup(interaction).await),
                Some("disable") => into_http_response(handlers::disable(interaction).await),
                Some("enable") => into_http_response(handlers::enable(interaction).await),
                Some("export") => into_http_response(handlers::export(interaction).await),
                Some("help") => into_http_response(handlers::help(interaction).await),
                _ => Err(Error::UnknownCommand),
            }
        }
        _ => Err(Error::UnsupportedInteractionType),
    }
}

/// Run the actix server
pub async fn run(args: Args, http: Http) -> anyhow::Result<()> {
    let app_state = AppState::new(args, http).context("constructing app state")?;
    let app_data = web::Data::new(app_state.clone());
    HttpServer::new(move || {
        App::new()
            .app_data(app_data.clone())
            .service(handle_interaction)
    })
    .bind(("0.0.0.0", app_state.config.port))
    .context("binding to 0.0.0.0")?
    .run()
    .await
    .context("running the web server")
}
