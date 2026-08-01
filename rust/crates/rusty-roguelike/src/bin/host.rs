use std::{env, net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::{bail, Context, Result};
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use rusty_roguelike::{
    generate_authored_floor, starter_ruleset, GameSession, SessionCommandDto, SessionErrorDto,
    SessionView, WorldState,
};
use serde_json::json;
use tokio::sync::Mutex;
use tower_http::services::{ServeDir, ServeFile};

const EXPEDITION_SEED: u64 = 5_201;

#[tokio::main]
async fn main() -> Result<()> {
    let options = Options::parse()?;
    let index = options.static_root.join("index.html");
    if !index.is_file() {
        bail!(
            "static application is missing at {}; run pnpm run build first",
            index.display()
        );
    }

    let session = new_expedition()?;
    let state = AppState {
        session: Arc::new(Mutex::new(session)),
        save_slot: Arc::new(Mutex::new(None)),
    };
    let app = Router::new()
        .route(
            "/healthz",
            get(|| async { Json(json!({ "status": "ok", "owner": "rust" })) }),
        )
        .route(
            "/api/v1/bootstrap",
            get(|| async { Json(rusty_roguelike::bootstrap_readout()) }),
        )
        .route("/api/v1/session", get(session_view))
        .route("/api/v1/session/commands", post(session_command))
        .route("/api/v1/session/save", post(save_session))
        .route("/api/v1/session/reopen", post(reopen_session))
        .route("/api/v1/session/restart", post(restart_session))
        .with_state(state)
        .fallback_service(ServeDir::new(&options.static_root).fallback(ServeFile::new(index)));

    let listener = tokio::net::TcpListener::bind(options.address)
        .await
        .with_context(|| format!("could not bind {}", options.address))?;
    println!("Rusty Roguelike listening on http://{}", options.address);
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Clone)]
struct AppState {
    session: Arc<Mutex<GameSession>>,
    save_slot: Arc<Mutex<Option<String>>>,
}

async fn session_view(
    State(state): State<AppState>,
) -> Result<Json<SessionView>, (StatusCode, Json<SessionErrorDto>)> {
    let session = state.session.lock().await;
    session
        .view()
        .map(Json)
        .map_err(|error| classified_error(StatusCode::INTERNAL_SERVER_ERROR, error))
}

async fn session_command(
    State(state): State<AppState>,
    Json(command): Json<SessionCommandDto>,
) -> Result<Json<SessionView>, (StatusCode, Json<SessionErrorDto>)> {
    let mut session = state.session.lock().await;
    session
        .command(command.into())
        .map(Json)
        .map_err(|error| classified_error(StatusCode::CONFLICT, error))
}

async fn restart_session(
    State(state): State<AppState>,
) -> Result<Json<SessionView>, (StatusCode, Json<SessionErrorDto>)> {
    let replacement = new_expedition()
        .map_err(|error| internal_error("session_restart_failed", error.to_string()))?;
    let view = replacement
        .view()
        .map_err(|error| classified_error(StatusCode::INTERNAL_SERVER_ERROR, error))?;
    *state.session.lock().await = replacement;
    Ok(Json(view))
}

async fn save_session(
    State(state): State<AppState>,
) -> Result<Json<SessionView>, (StatusCode, Json<SessionErrorDto>)> {
    let (encoded, view) = {
        let session = state.session.lock().await;
        let encoded = session
            .encode_save()
            .map_err(|error| classified_error(StatusCode::CONFLICT, error))?;
        let view = session
            .view()
            .map_err(|error| classified_error(StatusCode::INTERNAL_SERVER_ERROR, error))?;
        (encoded, view)
    };
    *state.save_slot.lock().await = Some(encoded);
    Ok(Json(view))
}

async fn reopen_session(
    State(state): State<AppState>,
) -> Result<Json<SessionView>, (StatusCode, Json<SessionErrorDto>)> {
    let encoded = state.save_slot.lock().await.clone().ok_or_else(|| {
        internal_error(
            "session_save_missing",
            "no session has been saved in this host process".to_owned(),
        )
    })?;
    let replacement = GameSession::decode_save(&encoded)
        .map_err(|error| classified_error(StatusCode::CONFLICT, error))?;
    let view = replacement
        .view()
        .map_err(|error| classified_error(StatusCode::INTERNAL_SERVER_ERROR, error))?;
    *state.session.lock().await = replacement;
    Ok(Json(view))
}

fn new_expedition() -> Result<GameSession> {
    Ok(GameSession::new(WorldState::new(
        generate_authored_floor(EXPEDITION_SEED)?,
        starter_ruleset()?,
    )?)?)
}

fn classified_error(
    status: StatusCode,
    error: rusty_roguelike::SessionError,
) -> (StatusCode, Json<SessionErrorDto>) {
    (status, Json(SessionErrorDto::from(&error)))
}

fn internal_error(code: &str, detail: String) -> (StatusCode, Json<SessionErrorDto>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(SessionErrorDto {
            code: code.to_owned(),
            detail,
        }),
    )
}

struct Options {
    address: SocketAddr,
    static_root: PathBuf,
}

impl Options {
    fn parse() -> Result<Self> {
        let default_static_root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../dist/apps/app/browser");
        let mut address = "127.0.0.1:4417".parse()?;
        let mut static_root = default_static_root;
        let mut arguments = env::args().skip(1);
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--address" => {
                    address = arguments
                        .next()
                        .context("--address requires a value")?
                        .parse()
                        .context("--address must be a socket address")?;
                }
                "--static-root" => {
                    static_root =
                        PathBuf::from(arguments.next().context("--static-root requires a value")?);
                }
                _ => bail!("unknown argument {argument}"),
            }
        }
        Ok(Self {
            address,
            static_root,
        })
    }
}
