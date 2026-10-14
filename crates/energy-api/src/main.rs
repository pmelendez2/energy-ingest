// SPDX-License-Identifier: MIT

//! Local API process: loopback/container network binding is configured
//! by deployment. Do not expose this service directly to the Internet.

use std::{env, net::SocketAddr};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = env::var("ENGY_DB_URL")?;
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;
    let address: SocketAddr = env::var("ENGY_API_LISTEN")
        .unwrap_or_else(|_| "127.0.0.1:8080".into())
        .parse()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    let docs = env::var("ENGY_API_DEV_DOCS").as_deref() == Ok("1");
    if docs && !address.ip().is_loopback() {
        return Err("developer docs require a loopback listener".into());
    }
    let app = if docs {
        energy_api::developer_router(pool)
    } else {
        energy_api::router(pool)
    };
    axum::serve(listener, app).await?;
    Ok(())
}
