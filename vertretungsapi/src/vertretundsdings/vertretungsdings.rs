use chrono::naive::NaiveDate;
use futures::TryFutureExt;
use itertools::Itertools;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::{
    env,
    sync::{Arc, Mutex},
};

use crate::create_weeks_list::{WeekZyklusList, Zyklus};

#[derive(Debug)]
pub enum ChangeOption<T> {
    Some(T),
    Same(T),
    None,
    End,
}

pub async fn check_change(
    number: i64,
    last_time: &mut String,
    last_date: &mut NaiveDate,
    weeks_zykluses: Arc<Mutex<WeekZyklusList>>,
) -> ChangeOption<VDay> {
    let url = format!(
        "https://geschuetzt.bszet.de/s-lk-vw/Vertretungsplaene/V_PlanBGy/V_DC_00{}.html",
        number
    );
    let c = Client::new();
    let result = c
        .get(url)
        .basic_auth("bsz-et-2223", Some(env::var("PW").expect("no PW in env")))
        .send()
        .map_ok(|e| {
            if e.status().is_success() {
                Option::Some(e)
            } else {
                Option::None
            }
        })
        .await
        .ok()
        .flatten();

    let res = match result {
        Some(r) => r,
        None => return ChangeOption::End,
    };

    let headers = res.headers();
    let this_time = headers
        .get("last-modified")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let text = res.text().await.unwrap();
    let vday = if let Ok(weeks) = weeks_zykluses.try_lock() {
        match get_vday(&text, last_date, &weeks) {
            Some(vday) => vday,
            None => return ChangeOption::None,
        }
    } else {
        return ChangeOption::None;
    };

    return match last_time.to_string().eq(this_time.as_str()) {
        true => ChangeOption::Same(vday),
        false => {
            *last_time = this_time.to_string();
            ChangeOption::Some(vday)
        }
    };
}

pub fn get_vday(
    text: &String,
    last_date: &mut NaiveDate,
    weeks_zykluses: &WeekZyklusList,
) -> Option<VDay> {
    let doc = Html::parse_document(text);

    let date_selection = Selector::parse("h1.list-table-caption").unwrap();
    let date = doc
        .select(&date_selection)
        .next()
        .unwrap()
        .inner_html()
        .trim()
        .to_string();

    let date_str = date.split_whitespace().last().unwrap();
    let this_date = NaiveDate::parse_from_str(date_str, "%d.%m.%Y").unwrap();

    if this_date <= *last_date {
        return None;
    } else {
        *last_date = this_date;
    }

    let zyklus = weeks_zykluses.get(&this_date).unwrap_or_default();

    let table_body_selection = Selector::parse("tbody").unwrap();
    let table_row_selection = Selector::parse("tr").unwrap();
    let table_field_selection = Selector::parse("td").unwrap();

    let mut v_lessons: Vec<Lesson> = Vec::new();

    let table = doc.select(&table_body_selection).next().unwrap();

    for row in table.select(&table_row_selection) {
        let fields = row.select(&table_field_selection);

        let mut content_fields: Vec<String> = fields
            .map(|item| item.inner_html().trim().to_string())
            .collect();
        if row.inner_html().contains("&nbsp;") {
            let last_lesson = v_lessons.last().unwrap().to_vec();
            for (i, s) in &mut content_fields
                .iter_mut()
                .enumerate()
                .filter(|(_i, s)| s.contains("&nbsp;"))
            {
                let replacement = last_lesson.get(i).unwrap();
                *s = replacement.to_string();
            }
        }

        v_lessons.push(Lesson {
            class: content_fields.get(0).unwrap().into(),
            time: content_fields
                .get(1)
                .unwrap()
                .trim_end_matches('.')
                .parse()
                .unwrap(),
            subject: content_fields.get(2).unwrap().into(),
            room: content_fields.get(3).unwrap().into(),
            teacher: content_fields.get(4).unwrap().into(),
            vtype: content_fields.get(5).unwrap().into(),
            message: content_fields.get(6).unwrap().into(),
        });
    }
    v_lessons = v_lessons
        .into_iter()
        .unique_by(Lesson::convert_to_compareable)
        .collect();

    return Some(VDay(date, zyklus, v_lessons.into()));
}

pub fn get_day(VDay(day_str, zyklus, v_lessons): &VDay, plan: &Plan) -> Day {
    let mut splits = day_str.split_whitespace().into_iter();
    let day_name = splits.next().unwrap();

    let mut res_day: Day = Day::new(&day_str.to_string());

    for v_lesson in v_lessons.iter().filter(|item| {
        item.class.contains(&plan.class_name) && is_in(&item.subject, &plan.subjects)
    }) {
        res_day
            .lessons
            .get_mut((v_lesson.time - 1) as usize)
            .unwrap()
            .push(v_lesson.clone());
    }

    let plan_day = plan
        .days
        .iter()
        .find(|item| item.day.contains(day_name))
        .unwrap();
    let empty_times = res_day
        .lessons
        .iter_mut()
        .enumerate()
        .filter(|(i, item)| i % 2 == 0 && item.len() == 0);

    for (i, ls) in empty_times {
        let normal = plan_day.lessons.get(i / 2).unwrap();
        match normal {
            WeekOption::AandB(l) => ls.push(l.to_lesson()),
            WeekOption::A(l) => match zyklus {
                Zyklus::I => ls.push(l.to_lesson()),
                Zyklus::II => (),
            },
            WeekOption::B(l) => match zyklus {
                Zyklus::II => ls.push(l.to_lesson()),
                Zyklus::I => (),
            },
            WeekOption::AorB(l1, l2) => match zyklus {
                Zyklus::I => ls.push(l1.to_lesson()),
                Zyklus::II => ls.push(l2.to_lesson()),
            },
            WeekOption::None => (),
        }
    }
    return res_day;
}

fn is_in(string: &str, vec: &Vec<String>) -> bool {
    for s in vec {
        if string.contains(s) {
            return true;
        }
    }
    return false;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanLesson {
    time: i64,
    subject: String,
    room: String,
    teacher: String,
}

impl PlanLesson {
    pub fn to_lesson(&self) -> Lesson {
        Lesson::new(
            self.time,
            self.subject.as_str(),
            self.room.as_str(),
            self.teacher.as_str(),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lesson {
    pub class: String,
    pub time: i64,
    pub subject: String,
    pub room: String,
    pub teacher: String,
    pub vtype: String,
    pub message: String,
}

impl Lesson {
    fn new(time: i64, subject: &str, room: &str, teacher: &str) -> Lesson {
        Lesson {
            class: String::new(),
            time,
            subject: subject.to_string(),
            room: room.to_string(),
            teacher: teacher.to_string(),
            vtype: String::new(),
            message: String::new(),
        }
    }

    fn convert_to_compareable(&self) -> (String, String, String, String, String, String, i32) {
        (
            self.class.to_string(),
            self.subject.to_string(),
            self.room.to_string(),
            self.teacher.to_string(),
            self.vtype.to_string(),
            self.message.to_string(),
            ((self.time + 1) / 2) as i32,
        )
    }

    fn to_vec(&self) -> Vec<String> {
        vec![
            self.class.to_string(),
            self.time.to_string(),
            self.subject.to_string(),
            self.room.to_string(),
            self.teacher.to_string(),
            self.vtype.to_string(),
            self.message.to_string(),
        ]
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Plan {
    pub class_name: String,
    pub days: Vec<PlanDay>,
    pub subjects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum WeekOption {
    #[default]
    None,
    AandB(PlanLesson),
    A(PlanLesson),
    B(PlanLesson),
    AorB(PlanLesson, PlanLesson),
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PlanDay {
    pub day: String,
    pub lessons: [WeekOption; 5],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VDay(String, Zyklus, Vec<Lesson>);

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct Day {
    pub day: String,
    pub lessons: [Vec<Lesson>; 10],
}

impl Day {
    pub fn new(day: &str) -> Day {
        Day {
            day: day.to_string(),
            lessons: Default::default(),
        }
    }
}
