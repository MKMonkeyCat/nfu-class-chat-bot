mod crawler;
mod db;
mod discord;
mod google_calendar;
mod leader_election;
mod link_chat;
mod state;

use config::{load_all_configs, spawn_config_hot_reload};
use crawler::spawn_crawler;
use db::init_db;
use discord::Handler;
use leader_election::try_acquire_leadership;
use link_chat::spawn_line_to_discord_bridge;
use serenity::prelude::*;
use state::{ConfigKey, DbKey};
use std::env;
use std::sync::Arc;

use crate::state::CrawlerKey;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set");

    let db = init_db().await;
    let (app_config, crawler_config) = load_all_configs().await.expect("read config failed");
    let config_arc = Arc::new(RwLock::new(app_config));
    let crawler_arc = Arc::new(RwLock::new(crawler_config));
    spawn_config_hot_reload(config_arc.clone(), crawler_arc.clone());

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
        data.insert::<ConfigKey>(config_arc.clone());
        data.insert::<CrawlerKey>(crawler_arc.clone());
        data.insert::<DbKey>(db.clone());
    }

    let _leader_guard = match try_acquire_leadership().await {
        Ok(guard) => guard,
        Err(err) => {
            eprintln!("leader election setup failed: {}", err);
            std::process::exit(1);
        }
    };

    spawn_crawler(crawler_arc.clone(), client.http.clone(), db.clone());

    if let Err(why) = client.start().await {
        println!("client error: {:?}", why);
    }
}
