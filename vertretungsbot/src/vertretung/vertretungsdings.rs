use prettytable::*;
use serde::{Deserialize, Serialize};
use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::utils::Color;

#[derive(Default, Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Zyklus {
    #[default]
    I,
    II,
}

pub fn get_day(VDay(day_str, zyklus, v_lessons): &VDay, plan: &Plan) -> Day {
    let mut splits = day_str.split_whitespace().into_iter();
    let day_name = splits.next().unwrap();

    let mut res_day: Day = Day::new(&day_str.as_str());

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
        let normal = plan_day.lessons.get(i/2).unwrap_or(&WeekOption::None);
        match normal {
            WeekOption::AandB(l) => ls.push(l.to_lesson()),
            WeekOption::A(l) => match zyklus {
                Zyklus::I => ls.push(l.to_lesson()),
                Zyklus::II => {}
            },
            WeekOption::B(l) => match zyklus {
                Zyklus::II => ls.push(l.to_lesson()),
                Zyklus::I => {}
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
    pub class : String,
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
            time: time,
            subject: subject.to_string(),
            room: room.to_string(),
            teacher: teacher.to_string(),
            vtype: String::new(),
            message: String::new(),
        }
    }
    fn to_embed(&self) -> CreateEmbed {
        let timestr = format!("{}.", self.time);
        let emptystring = String::from(" ");
        let fields = vec![
            (timestr.as_str(), &emptystring, false),
            ("Fach", &self.subject, true),
            ("Raum", &self.room, true),
            ("Lehrer", &self.teacher, true),
            ("Art", &self.vtype, true),
            ("Mitteilung", &self.message, true),
        ]
        .into_iter()
        .filter(|(_, s, _)| s.len() > 0);

        let mut e = CreateEmbed::default();
        e.fields(fields);

        if self.vtype.len() > 0 || self.message.len() > 0 {
            e.color(Color::RED);
        }
        e
    }

    fn to_row(&self) -> Row {
        row![
            self.time.to_string(),
            self.subject,
            self.room,
            self.teacher,
            self.vtype,
            self.message
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
pub struct VDay(pub String, pub Zyklus, pub Vec<Lesson>);

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

    pub fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_titles(row![
            "Stunde",
            "Fach",
            "Raum",
            "Lehrer",
            "Type",
            "Mitteilung"
        ]);
        self.lessons
            .iter()
            .filter(|item| item.len() > 0)
            .for_each(|lesson| {
                for l in lesson {
                    table.add_row(l.to_row());
                }
            });
        table
    }

    pub fn to_embed(&self, m: &mut CreateMessage) {
        let embeds: Vec<CreateEmbed> = self
            .lessons
            .iter()
            .map(|ls| ls.iter().map(|l| l.to_embed()))
            .flatten()
            .collect();
        m.content(&self.day).set_embeds(embeds);
    }

    pub fn to_string(&self) -> String {
        format! {"```{}\n{}```",self.day, self.to_table().to_string()}
    }
}
