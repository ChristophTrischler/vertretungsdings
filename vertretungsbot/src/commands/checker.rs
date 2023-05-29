use reqwest::{Client, Response};
use serenity::futures::TryFutureExt;
use serenity::model::id::UserId;
use serenity::{prelude::*, CacheAndHttp};
use sqlx::postgres::PgRow;
use sqlx::Row;
use std::env;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use uuid::Uuid;

use crate::vertretung::vertretungsdings::{get_day, Plan, VDay};

use crate::DBConnection;

const MIN: std::time::Duration = Duration::from_secs(10);

pub fn init_check_loop(
    arc_http: Arc<CacheAndHttp>,
    arc_data: Arc<RwLock<TypeMap>>,
) -> (JoinHandle<()>, CancellationToken) {
    let cancel_token = CancellationToken::new();
    (
        tokio::spawn(check_loop(arc_http, arc_data, cancel_token.clone())),
        cancel_token,
    )
}

async fn check_loop(
    arc_http: Arc<CacheAndHttp>,
    arc_data: Arc<RwLock<TypeMap>>,
    cancel_token: CancellationToken,
) {
    let client = Client::new();
    let id = Uuid::new_v4().to_string();
    let base_url = env::var("API_HOST").expect("API_HOST missing in env");
    let http = arc_http.as_ref();
    loop {
        let update = client
            .get(format!("{base_url}/update/{id}"))
            .send()
            .and_then(Response::json)
            .await
            .unwrap_or(false);

        info!("update: {update}");

        if update {
            let vdays: Vec<VDay> = client
                .get(format!("{base_url}/vdays"))
                .send()
                .and_then(Response::json)
                .await
                .unwrap_or_default();

            let connection = {
                let data_read = arc_data.read().await;
                match data_read.get::<DBConnection>() {
                    Some(c) => c.clone(),
                    _ => continue,
                }
            };

            let query = sqlx::query(
                "SELECT \"discord_id\", \"embed\", \"data\" FROM \"user\" WHERE \"active\" = true",
            );
            let rows = query
                .fetch_all(connection.as_ref())
                .await
                .unwrap_or_default();

            for row in rows {
                if let Err(e) = read_db_row_and_message(row, http, &vdays).await {
                    error!("err sendig dm: {:#?}", e);
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

async fn read_db_row_and_message(
    row: PgRow,
    http: &CacheAndHttp,
    vdays: &Vec<VDay>,
) -> Result<(), Box<dyn Error>> {
    let id: i64 = row.try_get(0)?;
    let embed_activated: bool = row.try_get(1)?;
    let data = row.try_get(2)?;

    let user = UserId(id as u64).to_user(http).await?;

    let plan: Plan = serde_json::from_str(data)?;

    for vday in vdays {
        let day = get_day(vday, &plan);
        user.direct_message(http, |m| {
            if embed_activated {
                day.to_embed(m);
                m
            } else {
                m.content(day.to_string())
            }
        })
        .await?;
    }
    Ok(())
}
