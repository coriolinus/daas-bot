mod error;
mod handlers;
mod into_response;
mod validate;

use std::sync::Arc;

use actix_web::{App, HttpResponse, HttpServer, middleware::Logger, post, web};
use anyhow::Context as _;
use log::{debug, warn};
use rusqlite::Connection;
use serenity::all::{Http, Interaction, Verifier};
use tokio::sync::Mutex;

pub use self::into_response::{Defer, Message, Pong};
use self::{
    error::{Error, Result},
    into_response::IntoResponse,
    validate::{XSignatureEd25519, XSignatureTimestamp},
};
use crate::{cli::Args, sql};

#[derive(derive_more::Debug, Clone)]
struct AppState {
    config: Arc<Args>,
    http: Arc<Http>,
    #[debug("<Verifier instance>")]
    verifier: Verifier,
    local_db: Arc<Mutex<Connection>>,
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

        let mut connection = Connection::open(&config.database_path).context(format!(
            "opening local database at {}",
            config.database_path.display()
        ))?;
        sql::migrations::runner()
            .run(&mut connection)
            .context("applying migrations to local db")?;
        let local_db = Arc::new(Mutex::new(connection));

        Ok(Self {
            config,
            http,
            verifier,
            local_db,
        })
    }
}

#[post("/")]
async fn handle_interaction(
    app_state: web::Data<AppState>,
    signature: Option<web::Header<XSignatureEd25519>>,
    timestamp: Option<web::Header<XSignatureTimestamp>>,
    body: web::Bytes,
) -> Result<HttpResponse> {
    debug!("handling request to /");

    // we specified the headers as optional in the extractors, because
    // in the event they are unset we still want to capture them and use our custom
    // validation error.
    let signature = signature
        .as_deref()
        .map(|header| &**header)
        .unwrap_or_default();
    let timestamp = timestamp
        .as_deref()
        .map(|header| &**header)
        .unwrap_or_default();

    app_state
        .verifier
        .verify(signature, timestamp, &body)
        .map_err(|_| Error::Validation)
        .inspect_err(|_| warn!("verifier failed to verify incoming request"))?;
    debug!("request headers validate successfully");

    let interaction = serde_json::from_slice::<Interaction>(&body)
        .map_err(|_| Error::MalformedInput("failed to deserialize interaction"))
        .inspect_err(|_| warn!("failed to deserialize interaction"))?;

    match interaction {
        Interaction::Ping(interaction) => handlers::ping(interaction)
            .await
            .map(IntoResponse::into_http),
        Interaction::Command(interaction) => {
            match interaction
                .data
                .options
                .first()
                .map(|option| option.name.as_ref())
            {
                Some("cleanup") => handlers::cleanup(interaction, &app_state)
                    .await
                    .map(IntoResponse::into_http),
                Some("disable") => handlers::disable(interaction, &app_state)
                    .await
                    .map(IntoResponse::into_http),
                Some("enable") => handlers::enable(interaction, &app_state)
                    .await
                    .map(IntoResponse::into_http),
                Some("export") => handlers::export(interaction, &app_state)
                    .await
                    .map(IntoResponse::into_http),
                Some("help") => handlers::help(interaction)
                    .await
                    .map(IntoResponse::into_http),
                _ => Err(Error::UnknownCommand),
            }
        }
        _ => Err(Error::UnsupportedInteractionType),
    }
    .inspect_err(|err| warn!("interaction handler returned error: {err}"))
}

/// Run the actix server
pub async fn run(args: Args, http: Http) -> anyhow::Result<()> {
    let app_state = AppState::new(args, http).context("constructing app state")?;
    let app_data = web::Data::new(app_state.clone());
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(app_data.clone())
            .service(handle_interaction)
    })
    .bind(("0.0.0.0", app_state.config.port))
    .context("binding to 0.0.0.0")?
    .run()
    .await
    .context("running the web server")
}
