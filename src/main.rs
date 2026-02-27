mod app_config;
mod db;
mod handler;
mod leader_election;
mod link_chat;
mod state;

use app_config::{load_app_config, spawn_config_hot_reload};
use db::init_db;
use handler::Handler;
use leader_election::try_acquire_leadership;
use link_chat::spawn_line_to_discord_bridge;
use serenity::prelude::*;
use state::{ConfigKey, DbKey};
use std::env;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set");

    let db = init_db().await;
    let app_config = load_app_config().expect("read config failed");
    let config_arc = Arc::new(RwLock::new(app_config));
    spawn_config_hot_reload(config_arc.clone());

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    let http = client.http.clone();
    spawn_line_to_discord_bridge(config_arc.clone(), http);

    {
        let mut data = client.data.write().await;
        data.insert::<ConfigKey>(config_arc);
        data.insert::<DbKey>(db);
    }

    let _leader_guard = match try_acquire_leadership().await {
        Ok(guard) => guard,
        Err(err) => {
            eprintln!("leader election setup failed: {}", err);
            std::process::exit(1);
        }
    };

    if let Err(why) = client.start().await {
        println!("client error: {:?}", why);
    }
}
