mod check_loop;
mod create_weeks_list;
mod vertretundsdings;

use actix_cors::Cors;
use actix_web::web::Data;
use actix_web::{
    web::{Json, Path},
    *,
};

use chrono::NaiveDate;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgPool};
use sqlx::Row;
use uuid::Uuid;

use std::error::Error;
use std::sync::{Arc, Mutex};

use check_loop::init_vday_cache;
use create_weeks_list::{create_weeks_list, WeekZyklusList, Zyklus};
use vertretundsdings::vertretungsdings::{get_day, Day, Plan, VDay};

pub type VdayCache = Mutex<Vec<VDay>>;
pub type UpdatedList = Mutex<Vec<Uuid>>;

#[get("/update/{id}")]
async fn updated(id: Path<Uuid>, update_list: Data<UpdatedList>) -> impl Responder {
    let mut val = true;
    if let Ok(mut list) = update_list.try_lock() {
        match list.contains(&id) {
            true => val = false,
            false => list.push(*id),
        }
    }
    HttpResponse::Ok().json(val)
}

#[get("/vdays")]
async fn get_vdays(vdays: Data<VdayCache>) -> impl Responder {
    match vdays.try_lock() {
        Ok(data) => {
            let days: &Vec<VDay> = data.as_ref();
            HttpResponse::Ok().json(days)
        }
        _ => HttpResponse::InternalServerError().json(Vec::<VDay>::new()),
    }
}

#[post("/days")]
async fn get_days(plan: Json<Plan>, vdays_data: Data<VdayCache>) -> impl Responder {
    match vdays_data.try_lock() {
        Ok(vdays) => {
            let days: Vec<Day> = vdays.iter().map(|v| get_day(v, &plan)).collect();
            HttpResponse::Ok().json(days)
        }
        _ => HttpResponse::InternalServerError().json(Vec::<Day>::new()),
    }
}

#[get("/days/{plan_id}")]
async fn get_days_by_plan_id(
    plan_id: Path<i64>,
    dbconnection: Data<PgPool>,
    vdays_data: Data<VdayCache>,
) -> impl Responder {
    match days_by_plan_id(plan_id.as_ref(), dbconnection, vdays_data).await {
        Ok(days) => HttpResponse::Ok().json(days),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

async fn days_by_plan_id(
    plan_id: &i64,
    dbconnection: Data<PgPool>,
    vdays_data: Data<VdayCache>,
) -> Result<Vec<Day>, Box<dyn Error>> {
    let row = sqlx::query("SELECT \"data\" FROM \"user\" WHERE \"discord_id\" = $1")
        .bind(plan_id)
        .fetch_one(dbconnection.as_ref())
        .await?;
    let str_data_plan = row.try_get(0)?;
    let plan: Plan = serde_json::from_str(str_data_plan)?;
    let vdays_res = vdays_data.try_lock().map_err(|err| err.to_string())?;
    let vdays_vec: &Vec<VDay> = &vdays_res.as_ref();
    let days: Vec<Day> = vdays_vec.iter().map(|vday| get_day(vday, &plan)).collect();
    Ok(days)
}

#[get("/zyklus/{date_str}")]
async fn get_week_zyklus_by_date(
    date: Path<NaiveDate>,
    week_zyklus_list: Data<Mutex<WeekZyklusList>>,
) -> impl Responder {
    match week_zyklus_list
        .try_lock()
        .ok()
        .and_then(|zl| zl.get(&date))
    {
        Some(z) => HttpResponse::Ok().json(z),
        None => HttpResponse::InternalServerError().json(":|"),
    }
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    log::info!("starting HTTP server at http://localhost:8000");

    let week_list = create_weeks_list().await.expect("Err loading zyklus");

    let (vdays, updated_list, handle, cancel_token) = init_vday_cache(&week_list);

    let pg_pool = Arc::new(
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
            .expect("Err creating client"),
    );

    HttpServer::new(move || {
        App::new()
            .app_data(Data::from(Arc::clone(&vdays)))
            .app_data(Data::from(Arc::clone(&updated_list)))
            .app_data(Data::from(Arc::clone(&pg_pool)))
            .app_data(Data::from(Arc::clone(&week_list)))
            .wrap(Cors::default().allow_any_origin().allow_any_method())
            .wrap(middleware::Logger::default())
            .wrap(
                Cors::default()
                .allow_any_origin()
                .allow_any_method() 
            )
            .service(get_vdays)
            .service(updated)
            .service(get_days) 
            .service(get_days_by_plan_id)
            .service(get_week_zyklus_by_date)
    })
    .bind(("0.0.0.0", 8000))?
    .run()
    .await?;
    cancel_token.cancel();
    handle.await?;
    log::info!("application successfully shut down gracefully");
    Ok(())
}
