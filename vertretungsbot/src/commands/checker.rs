use std::sync::Arc;
use std::time::Duration;

use serenity::model::id::UserId;
use serenity::{prelude::*};
use tracing::{error, info};
use sqlx::{Row};
use reqwest::Client;
use uuid::Uuid;
use std::env;

use crate::vertretung::vertretungsdings::{Plan, get_day, VDay};


use crate::DBConnection;    

pub async fn check_loop(arc_ctx: Arc<Context>){
    let min15 = Duration::from_secs(60);
    let client = Client::new();
    let id = Uuid::new_v4().to_string();
    let base_url = env::var("API_HOST").unwrap();
    loop {
        let update = client.get(format!("{base_url}/update/{id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

        if update {
            let vdays: Vec<VDay> = client.get(format!("{base_url}/vdays"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

            let ctx: &Context = arc_ctx.as_ref();
            let connection = {
                let data_read = ctx.data.read().await;
                data_read.get::<DBConnection>().unwrap().clone()
            };

            let query = sqlx::query(
                "SELECT \"discord_id\", \"embed\", \"data\" FROM \"user\" WHERE \"active\" = true"
            ); 
            let rows = query.fetch_all(connection.as_ref())
            .await
            .unwrap();

            for row in rows {
                let id: i64 = row.try_get(0).unwrap();
                let embed_activated: bool = row.try_get(1).unwrap();
                let data = row.try_get(2).unwrap();

                let user = UserId(id as u64)
                .to_user(ctx)
                .await
                .unwrap();
                
                let plan: Plan = serde_json::from_str(data).unwrap();

                for vday in &vdays {
                    let day = get_day(vday, &plan); 


                    if let Err(why) = user.direct_message(ctx,|m|{
                        if embed_activated {
                            day.to_embed(m);
                            m
                        }
                        else {
                            m.content(day.to_string())
                        }
                    }).await {
                        error!("Error sending dm: {:?}", why);
                    }
                }
            }
        }
        
        
        info!("checked for updates");

        tokio::time::sleep(min15).await;    
    }

}