use chrono::NaiveDate;
use log::info;
use lopdf::Document;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt::Debug,
    sync::{Arc, Mutex},
};

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub enum Zyklus {
    #[default]
    I,
    II,
}

impl Zyklus {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "I" => Some(Zyklus::I),
            "II" => Some(Zyklus::II),
            _ => None,
        }
    }
}

struct FirstAndLast<T> {
    first: Option<T>,
    last: Option<T>,
}

impl<T: Clone> FirstAndLast<T> {
    fn new() -> Self {
        Self {
            first: None,
            last: None,
        }
    }

    fn push(&mut self, value: T) {
        if let None = self.first {
            self.first = Some(value.clone());
        }
        self.last = Some(value);
    }

    fn clear(&mut self) {
        self.first = None;
        self.last = None;
    }
}

#[derive(Debug)]
enum ConvertedOption {
    Date(NaiveDate),
    Zyklus(Zyklus),
    None,
    Reset,
}

impl ConvertedOption {
    fn convert(mut s: &str) -> Self {
        s = s.trim_end_matches(' ');
        if let Ok(date) = NaiveDate::parse_from_str(s, "%d.%m.%Y") {
            return ConvertedOption::Date(date);
        }
        if let Some(wz) = Zyklus::from_str(s) {
            return ConvertedOption::Zyklus(wz);
        }
        if s.parse::<i32>().is_ok() {
            return ConvertedOption::Reset;
        }
        return ConvertedOption::None;
    }
}

enum Compared<T> {
    Bigger,
    Smaller,
    Right(T),
}

#[derive(Debug)]
struct WeekZyklus {
    start: NaiveDate,
    end: NaiveDate,
    zyklus: Zyklus,
}

impl WeekZyklus {
    fn compare(&self, date: &NaiveDate) -> Compared<Zyklus> {
        if date < &self.start {
            return Compared::Smaller;
        }
        if date > &self.end {
            return Compared::Bigger;
        }
        return Compared::Right(self.zyklus.clone());
    }
}

#[derive(Debug)]
pub struct WeekZyklusList(Vec<WeekZyklus>);

impl WeekZyklusList {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        info!("new WeekZyklus");
        let url = "https://frei.bszet.de/index.php?dir=/Blockplaene/BGy";
        let buf = reqwest::get(url).await?.text().await?;
        let doc = Html::parse_document(&buf);
        let item_selector = Selector::parse("td.FileListCellText")?;
        let a_selector = Selector::parse("a")?;
        let mut wzl = WeekZyklusList(Vec::new());
        for el in doc.select(&item_selector) {
            for a in el.select(&a_selector) {
                let url = a.value().attr("href").unwrap_or_default();
                if url.ends_with(".pdf") {
                    wzl.add(format!("https://frei.bszet.de/{url}").as_str())
                        .await?;
                }
            }
        }
        let x = (wzl.0.len() as f32).log2();
        let needed_size = (2 as i32).pow(x.ceil() as u32) as usize;
        let high_date = NaiveDate::from_ymd_opt(262143, 12, 31).unwrap();
        while wzl.0.len() < needed_size {
            wzl.0.push(WeekZyklus {
                start: high_date,
                end: high_date,
                zyklus: Zyklus::I,
            })
        }
        Ok(wzl)
    }

    pub async fn add(&mut self, pdf_url: &str) -> Result<(), Box<dyn Error>> {
        info!("added date from {pdf_url} to list");
        let text: &str = {
            let buf = reqwest::get(pdf_url).await?.bytes().await?;
            let doc = Document::load_mem(&buf)?;
            &doc.extract_text(&[1])?
        };
        let mut days = FirstAndLast::new();
        for line in text.split_terminator('\n') {
            match ConvertedOption::convert(line) {
                ConvertedOption::None => (),
                ConvertedOption::Date(d) => days.push(d),
                ConvertedOption::Zyklus(zyklus) => {
                    if let (Some(start), Some(end)) = (days.first, days.last) {
                        self.0.push(WeekZyklus { start, end, zyklus });
                    }
                }
                ConvertedOption::Reset => days.clear(),
            }
        }
        Ok(())
    }

    pub fn get(&self, date: &NaiveDate) -> Option<Zyklus> {
        let mut position = self.0.len() / 2;
        let mut change = position;
        while change != 0 {
            change /= 2;
            match self.0.get(position - 1)?.compare(&date) {
                Compared::Bigger => position += change,
                Compared::Smaller => position -= change,
                Compared::Right(z) => return Some(z),
            }
        }
        None
    }
}

pub async fn create_weeks_list() -> Result<Arc<Mutex<WeekZyklusList>>, Box<dyn Error>> {
    Ok(Arc::new(Mutex::new(WeekZyklusList::new().await?)))
}
