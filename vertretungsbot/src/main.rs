mod commands;
mod vertretung;

use std::collections::HashSet;
use std::env;
use std::sync::Arc;

use commands::checker::init_check_loop;
use serenity::async_trait;
use serenity::client::bridge::gateway::ShardManager;
use serenity::framework::standard::macros::group;
use serenity::framework::StandardFramework;
use serenity::http::Http;
use serenity::model::event::ResumedEvent;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use tracing::{error, info};

use crate::commands::send_plan::*;
use crate::commands::setter::*;
use crate::commands::update::*;
pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

pub struct DBConnection;
impl TypeMapKey for DBConnection {
    type Value = Arc<PgPool>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

#[group]
#[commands(send_plan, update, set, embed)]
struct General;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let http = Http::new(&token);

    // We will fetch your bot's owners and id
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // Create the framework
    let framework = StandardFramework::new()
        .configure(|c| c.owners(owners).prefix("!"))
        .group(&GENERAL_GROUP);

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .framework(framework)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<DBConnection>(Arc::new(
            PgPoolOptions::new()
                .max_connections(15)
                .connect_with(
                    PgConnectOptions::new()
                        .host("db")
                        .database("vertretungsdings")
                        .username("postgres")
                        .password("pass"),
                )
                .await
                .expect("error to connect to DB"),
        ));
    }
    let shard_manager = client.shard_manager.clone();

    let (loop_handle, loop_stop) =
        init_check_loop(Arc::clone(&client.cache_and_http), Arc::clone(&client.data));

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
        loop_stop.cancel();
        loop_handle.await.expect("couldn't stop loop nice");
    });

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
