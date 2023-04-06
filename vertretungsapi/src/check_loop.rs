use crate::vertretundsdings::vertretungsdings::{check_change, ChangeOption};
use crate::UpdatedList;
use crate::VdayCache;
use chrono::Utc;
use log::info;
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use tokio::{task::JoinHandle, time::sleep};
use tokio_util::sync::CancellationToken;

pub fn init_vday_cache() -> (
    Arc<VdayCache>,
    Arc<UpdatedList>,
    JoinHandle<()>,
    CancellationToken,
) {
    let cache = Arc::new(VdayCache::default());
    let updated_list = Arc::new(UpdatedList::default());
    let cancel = CancellationToken::new();

    (
        Arc::clone(&cache),
        Arc::clone(&updated_list),
        tokio::spawn(spawn_check_loop(
            Arc::clone(&cache),
            Arc::clone(&updated_list),
            cancel.clone(),
        )),
        cancel,
    )
}

async fn spawn_check_loop(
    cache: Arc<VdayCache>,
    updated_list: Arc<UpdatedList>,
    stop_signal: CancellationToken,
) {
    let mut times = HashMap::new();
    loop {
        let mut updated = false;
        let mut vdays_local = Vec::new();
        let mut date = (Utc::now() - chrono::Duration::days(1)).naive_utc().date();

        for i in 1..=10 {
            let last = if let Some(s) = times.get_mut(&i) {
                s
            } else {
                times.insert(i, String::new());
                times.get_mut(&i).unwrap()
            };

            match check_change(i, last, &mut date).await {
                ChangeOption::Some(vday) => {
                    vdays_local.push(vday);
                    updated = true;
                }
                ChangeOption::Same(vday) => vdays_local.push(vday),
                ChangeOption::None => continue,
                ChangeOption::End => break,
            };
        }
        if let Ok(mut vdays) = cache.try_lock() {
            vdays.clear();
            vdays.append(&mut vdays_local);
        }
        if updated {
            if let Ok(mut list) = updated_list.try_lock() {
                list.clear();
            }
            info!("vdays changed");
        }
        info!("checked for updates");

        tokio::select! {
            _ = sleep(Duration::from_secs(900)) => {
                continue;
            }

            _ = stop_signal.cancelled() => {
                log::info!("gracefully shutting down cache purge job");
                break;
            }
        };
    }
}

