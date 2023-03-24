mod vertretundsdings;
mod check_loop;

use actix_web::{*, web::{Path, Json}};
use actix_web::web::Data;
use actix_cors::Cors;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgPool};
use sqlx::{Row};
use uuid::Uuid;

use std::sync::{Arc, Mutex};

use vertretundsdings::vertretungsdings::{VDay, Plan, get_day, Day};
use check_loop::init_vday_cache;

pub type VdayCache = Mutex<Vec<VDay>>;
pub type UpdatedList = Mutex<Vec<Uuid>>;


#[get("/update/{id}")]
async fn updated(id: Path<Uuid>, update_list: Data<UpdatedList>) -> impl Responder{
    let mut val = true;
    if let Ok(mut list) = update_list.try_lock() {
        match list.contains(&id) {
            true => val = false,
            false => list.push(*id)
        }
    }
    HttpResponse::Ok().json(val)
}


#[get("/vdays")]
async fn get_vdays(vdays: Data<VdayCache>)-> actix_web::Result<impl Responder>{
    let data = vdays.try_lock().unwrap();
    let vdays: &Vec<VDay> = data.as_ref();
    Ok(HttpResponse::Ok().json(&vdays))
}

#[post("/days")]
async fn get_days(plan: Json<Plan>, vdays_data: Data<VdayCache>) -> impl Responder {
    let vdays = vdays_data.try_lock().unwrap();
    let days: Vec<Day> = vdays.iter().map(|v| get_day(v, &plan)).collect();
    println!("{:?}", days);
    HttpResponse::Ok().json(days)
}

#[get("/days/{plan_id}")]
async fn get_days_by_plan_id(plan_id: Path<i64>, dbconnection: Data<PgPool>, 
    vdays_data: Data<VdayCache>) -> impl Responder {
    let query_result = sqlx::query("SELECT \"data\" FROM \"user\" WHERE \"discord_id\" = $1")
    .bind(plan_id.as_ref())
    .fetch_one(dbconnection.as_ref())
    .await;
    if let Ok(row) =  query_result{
        let data: String = row.try_get(0).unwrap();
        let plan: Plan = serde_json::from_str(&data).unwrap();
        let vdays = vdays_data.try_lock().unwrap();
        let days: Vec<Day> = vdays.iter().map(|vday| get_day(vday, &plan)).collect();
        HttpResponse::Ok().json(days)
    }
    else {
        HttpResponse::BadRequest()
        .body("[]")
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    log::info!("starting HTTP server at http://localhost:8000");

    let (vdays,updated_list, handle, cancel_token)
        = init_vday_cache();

    let pg_pool = Arc::new(
        PgPoolOptions::new()
        .max_connections(15)
        .connect_with(
            PgConnectOptions::new()
            .host("db")
            .database("vertretungsdings")
            .username("postgres")
            .password("pass")
        )
        .await
        .expect("Err creating client")
    );

    HttpServer::new(move || {
        App::new()
            .app_data(Data::from(Arc::clone(&vdays)))
            .app_data(Data::from(Arc::clone(&updated_list)))
            .wrap(middleware::Logger::default())
            .service(get_vdays)
            .service(updated)
            .service(get_days)
            .service(get_days_by_plan_id)
    })
    .bind(("0.0.0.0", 8000))?
    .run()
    .await?;
    cancel_token.cancel();
    handle.await.unwrap();
    log::info!("application successfully shut down gracefully");
    Ok(())
}