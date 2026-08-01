use std::{env, net::SocketAddr, path::PathBuf};

use anyhow::{bail, Context, Result};
use axum::{routing::get, Json, Router};
use serde_json::json;
use tower_http::services::{ServeDir, ServeFile};

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

    let app = Router::new()
        .route(
            "/healthz",
            get(|| async { Json(json!({ "status": "ok", "owner": "rust" })) }),
        )
        .route(
            "/api/v1/bootstrap",
            get(|| async { Json(rusty_roguelike::bootstrap_readout()) }),
        )
        .fallback_service(ServeDir::new(&options.static_root).fallback(ServeFile::new(index)));

    let listener = tokio::net::TcpListener::bind(options.address)
        .await
        .with_context(|| format!("could not bind {}", options.address))?;
    println!("Rusty Roguelike listening on http://{}", options.address);
    axum::serve(listener, app).await?;
    Ok(())
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
