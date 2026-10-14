// SPDX-License-Identifier: MIT

//! Operator-only local API token administration.
//! Avoid recording the create command output in logs or shell transcripts.

use energy_store::{create_api_client, list_api_clients, revoke_api_client};
use sqlx::mysql::MySqlPoolOptions;
use std::{env, process::ExitCode};

const USAGE: &str = "Usage: energy-api-token \
    create NAME | list | revoke CLIENT_ID";

async fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() == 1 && args[0] == "--help" {
        println!("{USAGE}");
        return Ok(());
    }
    let url = env::var("ENGY_DB_URL").map_err(|_| "ENGY_DB_URL is required")?;
    let pool = MySqlPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .map_err(|_| "database connection failed")?;
    match args.as_slice() {
        [command, name] if command == "create" => {
            let issued = create_api_client(&pool, name)
                .await
                .map_err(|e| e.to_string())?;
            eprintln!("Client created: {}", issued.client_id);
            eprintln!("Token displayed once; store it securely.");
            println!("{}", issued.token);
        }
        [command] if command == "list" => {
            let clients =
                list_api_clients(&pool).await.map_err(|e| e.to_string())?;
            for client in clients {
                let status = if client.revoked_at.is_some() {
                    "revoked"
                } else {
                    "active"
                };
                println!("{}\t{}\t{}", client.id, client.name, status);
            }
        }
        [command, id] if command == "revoke" => {
            let id =
                id.parse::<u64>().map_err(|_| "client ID must be numeric")?;
            if id == 0 {
                return Err("client ID must be positive".into());
            }
            if !revoke_api_client(&pool, id).await.map_err(|e| e.to_string())? {
                return Err("client not found or already revoked".into());
            }
            println!("Client revoked: {id}");
        }
        _ => return Err(USAGE.into()),
    }
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
