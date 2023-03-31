use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use tracing::{error, info};
use reqwest::Client;


use crate::vertretung::vertretungsdings::{Plan};


use crate::DBConnection;


#[command]
pub async fn send_plan(ctx: &Context, msg: &Message, mut _args: Args) -> CommandResult{
    let id = msg.author.id.0 as i64;
    info!("{} used !send_plan", id);  

    let connection = {
        let data_read = ctx.data.read().await;
        data_read.get::<DBConnection>().unwrap().clone()
    };  
    

    let opt_url = msg.attachments.first();
    if opt_url.is_none() {
        send_file_error(ctx, msg).await;
        return Ok(());
    }
    let url = &opt_url.unwrap().url;

    let client = Client::new();
    let opt_plan: Option<Plan> = client.get(url)
    .send().await
    .unwrap()
    .json().await 
    .ok();


    if opt_plan.is_none() {
        send_file_error(ctx, msg).await;
        return Ok(());
    }
    let plan = opt_plan.unwrap();

    let plan_str = serde_json::to_string(&plan).unwrap();

    sqlx::query("INSERT INTO \"user\" VALUES ($1,$2,$3,$4) 
        ON CONFLICT (discord_id) DO UPDATE SET \"data\" = EXCLUDED.data")
        .bind(id)
        .bind(true)
        .bind(false)
        .bind(plan_str)
        .execute(connection.as_ref())
        .await?;

    Ok(())
}

async fn send_file_error(ctx: &Context, msg: &Message){
    if let Err(why) = msg.channel_id.say(&ctx.http, "Error with atteched file").await {
        error!("Error sending message: {:?}", why);
    }
}