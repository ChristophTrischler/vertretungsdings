use std::sync::Arc;
use std::time::Duration;

use serenity::model::id::UserId;
use serenity::{prelude::*, CacheAndHttp};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use sqlx::{Row};
use reqwest::Client;
use uuid::Uuid;
use std::env;

use crate::vertretung::vertretungsdings::{Plan, get_day, VDay};

use crate::DBConnection;

const MIN: std::time::Duration = Duration::from_secs(10);


pub fn init_check_loop(arc_http: Arc<CacheAndHttp>, arc_data: Arc<RwLock<TypeMap>>) -> (JoinHandle<()>, CancellationToken){
    
    let cancel_token = CancellationToken::new();
    (
        tokio::spawn(
            check_loop(
                arc_http, 
                arc_data, 
                cancel_token.clone()
            )
        ),
        cancel_token
    )
    
}

async fn check_loop(arc_http: Arc<CacheAndHttp>, arc_data: Arc<RwLock<TypeMap>>,cancel_token: CancellationToken){
    let client = Client::new();
    let id = Uuid::new_v4().to_string();
    let base_url = env::var("API_HOST").unwrap();
    let http = arc_http.as_ref();
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

            let connection = {
                let data_read = arc_data.read().await;
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
                .to_user(http)
                .await
                .unwrap();
                
                let plan: Plan = serde_json::from_str(data).unwrap();

                for vday in &vdays {
                    let day = get_day(vday, &plan); 


                    if let Err(why) = user.direct_message(http,|m|{
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

        tokio::select! {
            _ = sleep(MIN) => {
                continue;
            }

            _ = cancel_token.cancelled() => {
                info!("gracefully shutting down cache purge job");
                break;
            }
        };
    }

}