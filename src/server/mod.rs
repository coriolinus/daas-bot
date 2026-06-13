mod error;
mod handlers;
mod validate;

use std::sync::Arc;

use actix_web::{App, HttpResponse, HttpServer, body::BoxBody, post, web};
use anyhow::Context as _;
use serenity::all::{Http, Interaction, Verifier};

use self::{
    error::{Error, Result},
    validate::{XSignatureEd25519, XSignatureTimestamp},
};
use crate::cli::Args;

pub type FallibleResponse = Result<HttpResponse<BoxBody>>;

#[derive(derive_more::Debug, Clone)]
struct AppState {
    #[expect(unused)]
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
) -> FallibleResponse {
    data.verifier
        .verify(&signature, &timestamp, &body)
        .map_err(|_| Error::Validation)?;

    let interaction =
        serde_json::from_slice::<Interaction>(&body).map_err(|_| Error::MalformedInput)?;

    match interaction {
        Interaction::Ping(interaction) => handlers::ping(interaction).await,
        Interaction::Command(interaction) => {
            match interaction
                .data
                .options
                .first()
                .map(|option| option.name.as_ref())
            {
                Some("cleanup") => handlers::cleanup(interaction).await,
                Some("disable") => handlers::disable(interaction).await,
                Some("enable") => handlers::enable(interaction).await,
                Some("export") => handlers::export(interaction).await,
                Some("help") => handlers::help(interaction).await,
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
