use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use tracing::{info};


use crate::DBConnection;

#[command]
pub async fn embed(ctx: &Context, msg: &Message, mut args: Args)->CommandResult{
    let id = msg.author.id.0 as i64;
    info!("{} used !activate", id);
    let arg = args.single::<bool>()?;

    let connection = {
        let data_read = ctx.data.read().await;
        data_read.get::<DBConnection>().unwrap().clone()
    };

    sqlx::query("UPDATE \"user\" SET \"embed\"=$1 WHERE \"discord_id\"=$2")
    .bind(arg)
    .bind(id)
    .execute(connection.as_ref())
    .await?;

    Ok(())
}

#[command]
pub async fn set(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult{
    let id = msg.author.id.0 as i64;
    info!("{} used !activate", id);
    let status = args.single::<bool>()?;
    

    let connection = {
        let data_read = ctx.data.read().await;
        data_read.get::<DBConnection>().unwrap().clone()
    };

    sqlx::query("UPDATE \"user\" SET \"active\"=$1 WHERE \"discord_id\"=$2")
    .bind(status)   
    .bind(id)
    .execute(connection.as_ref())
    .await?;

    Ok(())
}