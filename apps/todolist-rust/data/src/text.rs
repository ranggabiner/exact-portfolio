//! The words the data module writes into view models, in English and Indonesian.
//! The view's own labels live in `strings/`; these are the ones built from data.

use crate::dates;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lang {
    En,
    Id,
}

impl Lang {
    pub fn from_tag(tag: &str) -> Lang {
        if tag.to_ascii_lowercase().starts_with("id") || tag.to_ascii_lowercase().starts_with("in")
        {
            Lang::Id
        } else {
            Lang::En
        }
    }

    pub fn pick<'a>(self, en: &'a str, id: &'a str) -> &'a str {
        match self {
            Lang::En => en,
            Lang::Id => id,
        }
    }
}

const WEEKDAYS_EN: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
const WEEKDAYS_ID: [&str; 7] = [
    "Senin", "Selasa", "Rabu", "Kamis", "Jumat", "Sabtu", "Minggu",
];
const SHORT_EN: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const SHORT_ID: [&str; 7] = ["Sen", "Sel", "Rab", "Kam", "Jum", "Sab", "Min"];
const MONTHS_EN: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const MONTHS_ID: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "Mei", "Jun", "Jul", "Agu", "Sep", "Okt", "Nov", "Des",
];

pub fn weekday_name(lang: Lang, day: i64) -> &'static str {
    let i = dates::weekday(day) as usize;
    match lang {
        Lang::En => WEEKDAYS_EN[i],
        Lang::Id => WEEKDAYS_ID[i],
    }
}

pub fn weekday_short(lang: Lang, day: i64) -> &'static str {
    let i = dates::weekday(day) as usize;
    match lang {
        Lang::En => SHORT_EN[i],
        Lang::Id => SHORT_ID[i],
    }
}

/// `"Fri, Nov 27"` / `"Jum, 27 Nov"`.
pub fn short_date(lang: Lang, day: i64) -> String {
    let (_, m, d) = dates::civil_from_days(day);
    match lang {
        Lang::En => format!(
            "{}, {} {d}",
            weekday_short(lang, day),
            MONTHS_EN[m as usize - 1]
        ),
        Lang::Id => format!(
            "{}, {d} {}",
            weekday_short(lang, day),
            MONTHS_ID[m as usize - 1]
        ),
    }
}

/// `"Friday, Nov 27"`.
pub fn long_date(lang: Lang, day: i64) -> String {
    let (_, m, d) = dates::civil_from_days(day);
    match lang {
        Lang::En => format!(
            "{}, {} {d}",
            weekday_name(lang, day),
            MONTHS_EN[m as usize - 1]
        ),
        Lang::Id => format!(
            "{}, {d} {}",
            weekday_name(lang, day),
            MONTHS_ID[m as usize - 1]
        ),
    }
}

/// A day named from today: Today, Tomorrow, a weekday within the week, else a date.
pub fn day_label(lang: Lang, day: i64, today: i64) -> String {
    match day - today {
        0 => lang.pick("Today", "Hari ini").into(),
        1 => lang.pick("Tomorrow", "Besok").into(),
        -1 => lang.pick("Yesterday", "Kemarin").into(),
        2..=6 => weekday_name(lang, day).into(),
        _ => short_date(lang, day),
    }
}

/// `17:30` as `5:30 PM` or `17.30`.
pub fn clock(lang: Lang, minutes: u32) -> String {
    let (h, m) = (minutes / 60, minutes % 60);
    match lang {
        Lang::En => {
            let twelve = match h % 12 {
                0 => 12,
                other => other,
            };
            format!("{twelve}:{m:02} {}", if h < 12 { "AM" } else { "PM" })
        }
        Lang::Id => format!("{h:02}.{m:02}"),
    }
}

/// `90` as `1h 30m` or `1 j 30 m`.
pub fn duration(lang: Lang, minutes: u32) -> String {
    let (h, m) = (minutes / 60, minutes % 60);
    match (lang, h, m) {
        (Lang::En, 0, m) => format!("{m}m"),
        (Lang::En, h, 0) => format!("{h}h"),
        (Lang::En, h, m) => format!("{h}h {m}m"),
        (Lang::Id, 0, m) => format!("{m} mnt"),
        (Lang::Id, h, 0) => format!("{h} jam"),
        (Lang::Id, h, m) => format!("{h} jam {m} mnt"),
    }
}

pub fn list_name(lang: Lang, id: &str, custom: &str) -> String {
    if !custom.is_empty() {
        return custom.into();
    }
    match id {
        "inbox" => lang.pick("Inbox", "Kotak Masuk"),
        "work" => lang.pick("Work", "Kerja"),
        "personal" => lang.pick("Personal", "Pribadi"),
        "study" => lang.pick("Study", "Belajar"),
        "health" => lang.pick("Health", "Kesehatan"),
        "home" => lang.pick("Home", "Rumah"),
        "money" => lang.pick("Money", "Keuangan"),
        "family" => lang.pick("Family", "Keluarga"),
        _ => lang.pick("List", "Daftar"),
    }
    .into()
}

pub fn repeat_label(lang: Lang, repeat: &str) -> &'static str {
    match (repeat, lang) {
        ("daily", Lang::En) => "Daily",
        ("daily", Lang::Id) => "Harian",
        ("weekdays", Lang::En) => "Weekdays",
        ("weekdays", Lang::Id) => "Hari kerja",
        ("weekly", Lang::En) => "Weekly",
        ("weekly", Lang::Id) => "Mingguan",
        ("monthly", Lang::En) => "Monthly",
        ("monthly", Lang::Id) => "Bulanan",
        _ => "",
    }
}

pub fn count_tasks(lang: Lang, n: usize) -> String {
    match lang {
        Lang::En if n == 1 => "1 task".into(),
        Lang::En => format!("{n} tasks"),
        Lang::Id => format!("{n} tugas"),
    }
}
