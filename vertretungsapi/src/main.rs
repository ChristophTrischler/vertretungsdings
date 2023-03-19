mod vertretundsdings;
mod check_loop;

use actix_web::{*, web::{Path, Json}};
use actix_web::web::Data;

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
    let data = vdays.try_lock().unwrap().clone();
    Ok(HttpResponse::Ok().json(data))
}

#[post("/days")]
async fn get_days(plan: Json<Plan>, vdays_data: Data<VdayCache>) -> impl Responder {
    let vdays = vdays_data.try_lock().unwrap().clone();
    let days: Vec<Day> = vdays.iter().map(|v| get_day(v, &plan)).collect();
    println!("{:?}", days);
    HttpResponse::Ok().json(days)
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    log::info!("starting HTTP server at http://localhost:8000");

    let (vdays,updated_list, handle, cancel_token)
        = init_vday_cache();

    HttpServer::new(move || {
        App::new()
            .app_data(Data::from(Arc::clone(&vdays)))
            .app_data(Data::from(Arc::clone(&updated_list)))
            .wrap(middleware::Logger::default())
            .service(get_vdays)
            .service(updated)
            .service(get_days)
    })
    .bind(("0.0.0.0", 8000))?
    .run()
    .await?;
    cancel_token.cancel();
    handle.await.unwrap();
    log::info!("application successfully shut down gracefully");
    Ok(())
}