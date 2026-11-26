//! Smart capture: free text in English or Indonesian becomes tasks with a day,
//! a time, a priority, a repeat, an estimate and a list. This is the on-device
//! baseline every device has; Apple's on-device model refines it where present.

use crate::dates;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Parsed {
    pub title: String,
    pub due: Option<i64>,
    pub time: Option<u32>,
    pub priority: u8,
    pub list: String,
    pub repeat: String,
    pub estimate: u32,
    pub reminder: bool,
}

pub struct Settings<'a> {
    pub today: i64,
    pub minute: u32,
    /// `(id, lowercase names)` of the person's lists.
    pub lists: &'a [(String, Vec<String>)],
    pub default_list: &'a str,
    /// A brain dump also splits on commas and sentences.
    pub dump: bool,
}

pub const MAX_DRAFTS: usize = 12;
pub const MAX_TITLE: usize = 140;

const VERBS: [&str; 42] = [
    "call",
    "buy",
    "email",
    "send",
    "pay",
    "book",
    "finish",
    "write",
    "read",
    "clean",
    "go",
    "schedule",
    "review",
    "prepare",
    "pick",
    "meet",
    "text",
    "fix",
    "water",
    "feed",
    "order",
    "plan",
    "update",
    "submit",
    "check",
    "telepon",
    "beli",
    "kirim",
    "bayar",
    "selesaikan",
    "tulis",
    "baca",
    "bersihkan",
    "pergi",
    "jadwalkan",
    "siapkan",
    "jemput",
    "temui",
    "hubungi",
    "cek",
    "perbarui",
    "kerjakan",
];

const FILLERS: [&str; 12] = [
    "remind me to ",
    "i need to ",
    "i have to ",
    "i should ",
    "need to ",
    "don't forget to ",
    "ingatkan saya untuk ",
    "ingatkan aku untuk ",
    "ingatkan saya ",
    "aku harus ",
    "saya harus ",
    "jangan lupa ",
];

const KEYWORDS: [(&str, &[&str]); 7] = [
    (
        "work",
        &[
            "meeting",
            "report",
            "client",
            "deadline",
            "slides",
            "invoice client",
            "standup",
            "rapat",
            "laporan",
            "klien",
            "presentasi",
            "kantor",
        ],
    ),
    (
        "health",
        &[
            "gym", "run", "workout", "doctor", "dentist", "meds", "medicine", "yoga", "olahraga",
            "lari", "dokter", "obat", "vitamin",
        ],
    ),
    (
        "home",
        &[
            "groceries",
            "grocery",
            "laundry",
            "clean",
            "dishes",
            "milk",
            "trash",
            "plants",
            "belanja",
            "cuci",
            "sapu",
            "susu",
            "sampah",
            "masak",
        ],
    ),
    (
        "money",
        &[
            "pay", "bill", "rent", "tax", "budget", "bank", "bayar", "tagihan", "sewa", "pajak",
            "transfer", "listrik",
        ],
    ),
    (
        "study",
        &[
            "study",
            "exam",
            "assignment",
            "homework",
            "lecture",
            "thesis",
            "class",
            "belajar",
            "ujian",
            "tugas kuliah",
            "skripsi",
            "kelas",
        ],
    ),
    (
        "family",
        &[
            "mom",
            "mum",
            "dad",
            "kids",
            "parents",
            "family",
            "ibu",
            "ayah",
            "mama",
            "papa",
            "anak",
            "orang tua",
            "keluarga",
        ],
    ),
    (
        "personal",
        &[
            "birthday",
            "haircut",
            "passport",
            "journal",
            "ulang tahun",
            "potong rambut",
            "paspor",
        ],
    ),
];

/// Break free text into the tasks it names.
pub fn segments(text: &str, dump: bool) -> Vec<String> {
    let mut text = text.replace(['\r'], "\n").replace(';', "\n");
    for separator in [
        " and then ",
        " then ",
        " lalu ",
        " kemudian ",
        " terus ",
        " and also ",
        " dan juga ",
    ] {
        text = replace_ignore_case(&text, separator, "\n");
    }
    if dump {
        for separator in [". ", ", ", "! ", "? "] {
            text = text.replace(separator, "\n");
        }
    }
    let mut out = Vec::new();
    for line in text.split('\n') {
        // "call mom and buy milk" is two tasks; "salt and pepper" is one.
        let words: Vec<&str> = line.split_whitespace().collect();
        let mut current: Vec<&str> = Vec::new();
        let mut i = 0;
        while i < words.len() {
            let joint = matches!(words[i].to_lowercase().as_str(), "and" | "dan");
            if joint
                && !current.is_empty()
                && i + 1 < words.len()
                && VERBS.contains(&clean(words[i + 1]).as_str())
            {
                out.push(current.join(" "));
                current.clear();
            } else {
                current.push(words[i]);
            }
            i += 1;
        }
        out.push(current.join(" "));
    }
    out.into_iter()
        .map(|s| {
            s.trim()
                .trim_start_matches(['-', '•', '*', ',', '.'])
                .trim_end_matches([',', '.', ';'])
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .take(MAX_DRAFTS)
        .collect()
}

fn replace_ignore_case(text: &str, needle: &str, with: &str) -> String {
    let lower = text.to_lowercase();
    // Lowercasing can change byte lengths outside ASCII; leave such text alone.
    if lower.len() != text.len() {
        return text.replace(needle, with);
    }
    let mut out = String::with_capacity(text.len());
    let mut from = 0;
    while let Some(found) = lower[from..].find(needle) {
        let at = from + found;
        out.push_str(&text[from..at]);
        out.push_str(with);
        from = at + needle.len();
    }
    out.push_str(&text[from..]);
    out
}

fn clean(word: &str) -> String {
    word.trim_matches(|c: char| {
        matches!(c, ',' | '.' | '?' | ':' | ';' | '(' | ')' | '"' | '\'') || c == '!'
    })
    .to_lowercase()
}

fn weekday_of(word: &str, after_preposition: bool) -> Option<u32> {
    let full = match word {
        "monday" | "senin" => Some(0),
        "tuesday" | "selasa" => Some(1),
        "wednesday" | "rabu" => Some(2),
        "thursday" | "kamis" => Some(3),
        "friday" | "jumat" | "jum'at" => Some(4),
        "saturday" | "sabtu" => Some(5),
        "sunday" => Some(6),
        _ => None,
    };
    full.or(match word {
        "mon" if after_preposition => Some(0),
        "tue" if after_preposition => Some(1),
        "wed" if after_preposition => Some(2),
        "thu" if after_preposition => Some(3),
        "fri" if after_preposition => Some(4),
        "sat" if after_preposition => Some(5),
        "sun" | "minggu" if after_preposition => Some(6),
        _ => None,
    })
}

fn month_of(word: &str) -> Option<u32> {
    const NAMES: [&[&str]; 12] = [
        &["jan", "january", "januari"],
        &["feb", "february", "februari"],
        &["mar", "march", "maret"],
        &["apr", "april"],
        &["may", "mei"],
        &["jun", "june", "juni"],
        &["jul", "july", "juli"],
        &["aug", "august", "agu", "agustus"],
        &["sep", "sept", "september"],
        &["oct", "october", "okt", "oktober"],
        &["nov", "november"],
        &["dec", "december", "des", "desember"],
    ];
    NAMES
        .iter()
        .position(|names| names.contains(&word))
        .map(|i| i as u32 + 1)
}

fn day_number(word: &str) -> Option<u32> {
    let digits = word
        .trim_end_matches("st")
        .trim_end_matches("nd")
        .trim_end_matches("rd")
        .trim_end_matches("th");
    digits.parse::<u32>().ok().filter(|d| (1..=31).contains(d))
}

/// The upcoming weekday, never today.
fn next_weekday(today: i64, weekday: u32) -> i64 {
    let ahead = (i64::from(weekday) - i64::from(dates::weekday(today))).rem_euclid(7);
    today + if ahead == 0 { 7 } else { ahead }
}

fn month_date(today: i64, month: u32, day: u32) -> Option<i64> {
    let (year, ..) = dates::civil_from_days(today);
    [year, year + 1]
        .into_iter()
        .filter(|y| day <= dates::month_length(*y, month))
        .map(|y| dates::days_from_civil(y, month, day))
        .find(|d| *d >= today)
}

#[derive(Clone, Copy)]
enum Half {
    Am,
    Pm,
}

/// `17:30`, `5.30pm`, `5pm`, `9` (only when `bare`).
fn clock_word(word: &str, bare: bool) -> Option<(u32, u32, Option<Half>)> {
    let (body, half) = if let Some(b) = word.strip_suffix("am") {
        (b, Some(Half::Am))
    } else if let Some(b) = word.strip_suffix("pm") {
        (b, Some(Half::Pm))
    } else {
        (word, None)
    };
    if body.is_empty()
        || !body
            .chars()
            .all(|c| c.is_ascii_digit() || c == ':' || c == '.')
    {
        return None;
    }
    let mut parts = body.split([':', '.']);
    let hour: u32 = parts.next()?.parse().ok()?;
    let minute = match parts.next() {
        Some(m) if m.len() == 2 => m.parse().ok()?,
        Some(_) => return None,
        None if half.is_some() || bare => 0,
        None => return None,
    };
    if parts.next().is_some()
        || hour > 23
        || minute > 59
        || (half.is_some() && !(1..=12).contains(&hour))
    {
        return None;
    }
    Some((hour, minute, half))
}

fn to_24(hour: u32, half: Half) -> u32 {
    match half {
        Half::Am => hour % 12,
        Half::Pm => hour % 12 + 12,
    }
}

fn unit_minutes(word: &str) -> Option<u32> {
    match word {
        "m" | "min" | "mins" | "minute" | "minutes" | "menit" | "mnt" => Some(1),
        "h" | "hr" | "hrs" | "hour" | "hours" | "jam" => Some(60),
        _ => None,
    }
}

/// `30m`, `45min`, `2h`.
fn joined_duration(word: &str) -> Option<u32> {
    let split = word.find(|c: char| !c.is_ascii_digit())?;
    let (digits, unit) = word.split_at(split);
    let n: u32 = digits.parse().ok()?;
    if unit == "j" {
        return n.checked_mul(60);
    }
    n.checked_mul(unit_minutes(unit)?)
}

pub fn parse_one(segment: &str, settings: &Settings) -> Parsed {
    let mut parsed = Parsed {
        repeat: "none".into(),
        list: settings.default_list.into(),
        ..Parsed::default()
    };
    let mut text = segment.trim().to_string();
    for filler in FILLERS {
        if text.len() > filler.len()
            && text.is_char_boundary(filler.len())
            && text[..filler.len()].eq_ignore_ascii_case(filler)
        {
            if filler.starts_with("remind") || filler.starts_with("ingatkan") {
                parsed.reminder = true;
            }
            text = text[filler.len()..].to_string();
            break;
        }
    }
    let raw: Vec<&str> = text.split_whitespace().collect();
    let words: Vec<String> = raw.iter().map(|w| clean(w)).collect();
    let mut used = vec![false; raw.len()];
    let mut period: Option<u32> = None;
    let mut explicit_list = false;
    let at = |i: usize| words.get(i).map(String::as_str).unwrap_or("");

    let mut i = 0;
    while i < raw.len() {
        if used[i] {
            i += 1;
            continue;
        }
        let word = at(i);
        let next = at(i + 1);
        let third = at(i + 2);
        let previous = if i == 0 { "" } else { at(i - 1) };
        let mut take = 0usize;

        // Priority.
        let shout = raw[i].to_lowercase();
        if shout.starts_with('!') {
            let level = match shout.as_str() {
                "!!!" | "!high" | "!tinggi" => 3,
                "!!" | "!medium" | "!med" | "!sedang" => 2,
                "!" | "!low" | "!rendah" => 1,
                _ => 0,
            };
            if level > 0 {
                parsed.priority = level;
                take = 1;
            }
        } else if matches!(word, "p1" | "p2" | "p3") {
            parsed.priority = match word {
                "p1" => 3,
                "p2" => 2,
                _ => 1,
            };
            take = 1;
        } else if matches!(word, "urgent" | "asap" | "segera" | "mendesak") {
            parsed.priority = 3;
            if i == 0 || i + 1 == raw.len() {
                take = 1;
            }
        }

        // A list by hashtag.
        if take == 0 && raw[i].starts_with('#') && raw[i].len() > 1 {
            let tag = raw[i][1..].to_lowercase();
            let tag = tag.trim_end_matches([',', '.']);
            if let Some((id, _)) = settings
                .lists
                .iter()
                .find(|(id, names)| id == tag || names.iter().any(|n| n == tag))
                .or_else(|| {
                    settings.lists.iter().find(|(id, names)| {
                        id.starts_with(tag) || names.iter().any(|n| n.starts_with(tag))
                    })
                })
            {
                parsed.list = id.clone();
                explicit_list = true;
                take = 1;
            }
        }

        // Repeats.
        if take == 0 {
            let every = matches!(word, "every" | "setiap" | "tiap");
            if every && matches!(next, "weekday" | "weekdays") {
                parsed.repeat = "weekdays".into();
                take = 2;
            } else if every && next == "hari" && third == "kerja" {
                parsed.repeat = "weekdays".into();
                take = 3;
            } else if every && matches!(next, "day" | "hari") {
                parsed.repeat = "daily".into();
                take = 2;
            } else if every && matches!(next, "week" | "minggu" | "pekan") {
                parsed.repeat = "weekly".into();
                take = 2;
            } else if every && matches!(next, "month" | "bulan") {
                parsed.repeat = "monthly".into();
                take = 2;
            } else if every && weekday_of(next, true).is_some() {
                parsed.repeat = "weekly".into();
                parsed.due = weekday_of(next, true).map(|d| next_weekday(settings.today, d));
                take = 2;
            } else if matches!(word, "daily" | "everyday") {
                parsed.repeat = "daily".into();
                take = 1;
            } else if word == "weekly" && i + 1 == raw.len() {
                parsed.repeat = "weekly".into();
                take = 1;
            } else if word == "monthly" && i + 1 == raw.len() {
                parsed.repeat = "monthly".into();
                take = 1;
            }
        }

        // Days.
        if take == 0 {
            let today = settings.today;
            let (due, n): (Option<i64>, usize) = match (word, next, third) {
                ("day", "after", "tomorrow") => (Some(today + 2), 3),
                ("hari", "ini", _) => (Some(today), 2),
                ("malam", "ini", _) => {
                    period = Some(20 * 60);
                    (Some(today), 2)
                }
                ("today", ..) => (Some(today), 1),
                ("tonight", ..) => {
                    period = Some(20 * 60);
                    (Some(today), 1)
                }
                ("tomorrow" | "tmr" | "tmrw" | "besok" | "esok", ..) => (Some(today + 1), 1),
                ("lusa", ..) => (Some(today + 2), 1),
                ("next", "week", _) | ("minggu" | "pekan", "depan", _) => {
                    (Some(next_weekday(today, 0)), 2)
                }
                ("next", "month", _) | ("bulan", "depan", _) => (Some(dates::add_month(today)), 2),
                ("this", "weekend", _) | ("akhir", "pekan", _) => {
                    let w = dates::weekday(today);
                    (
                        Some(if w >= 5 {
                            today
                        } else {
                            today + i64::from(5 - w)
                        }),
                        2,
                    )
                }
                ("in", number, unit) if number.parse::<i64>().is_ok() => {
                    let n: i64 = number.parse().unwrap_or(0);
                    match unit {
                        "day" | "days" if n <= 3650 => (Some(today + n), 3),
                        "week" | "weeks" if n <= 520 => (Some(today + 7 * n), 3),
                        _ => (None, 0),
                    }
                }
                (number, unit, "lagi") if number.parse::<i64>().is_ok() => {
                    let n: i64 = number.parse().unwrap_or(0);
                    match unit {
                        "hari" if n <= 3650 => (Some(today + n), 3),
                        "minggu" | "pekan" if n <= 520 => (Some(today + 7 * n), 3),
                        _ => (None, 0),
                    }
                }
                ("next" | "on" | "hari", day, _) if weekday_of(day, true).is_some() => {
                    (weekday_of(day, true).map(|d| next_weekday(today, d)), 2)
                }
                _ => {
                    if let Some(d) = weekday_of(word, false) {
                        (Some(next_weekday(today, d)), 1)
                    } else if let (Some(d), Some(m)) = (day_number(word), month_of(next)) {
                        (month_date(today, m, d), 2)
                    } else if let (Some(m), Some(d)) = (month_of(word), day_number(next)) {
                        (month_date(today, m, d), 2)
                    } else {
                        (None, 0)
                    }
                }
            };
            if let Some(due) = due {
                parsed.due = Some(due);
                take = n;
            }
        }

        // Estimates, before times: "2 jam" is a length, "jam 2" an hour.
        if take == 0 && !matches!(previous, "jam" | "pukul" | "at") {
            if let Some(minutes) = joined_duration(word).filter(|m| (1..=720).contains(m)) {
                parsed.estimate = minutes;
                take = 1;
            } else if let (Ok(n), Some(unit)) = (word.parse::<u32>(), unit_minutes(next)) {
                let minutes = n.saturating_mul(unit);
                if (1..=720).contains(&minutes) {
                    parsed.estimate = minutes;
                    take = 2;
                }
            }
        }

        // Clock times.
        if take == 0 {
            let cued = matches!(word, "at" | "jam" | "pukul");
            let (candidate, offset) = if cued { (next, 1) } else { (word, 0) };
            if let Some((hour, minute, half)) = clock_word(candidate, cued) {
                let after = at(i + offset + 1);
                let (hour, extra) = match (half, after) {
                    (Some(h), _) => (to_24(hour, h), 0),
                    (None, "am") if hour <= 12 => (to_24(hour, Half::Am), 1),
                    (None, "pm") if hour <= 12 => (to_24(hour, Half::Pm), 1),
                    (None, "pagi") => (hour % 12, 1),
                    (None, "siang") => (if hour < 7 { hour + 12 } else { hour }, 1),
                    (None, "sore" | "malam") => (if hour < 12 { hour + 12 } else { hour }, 1),
                    (None, _) if cued && (1..=6).contains(&hour) => (hour + 12, 0),
                    (None, _) => (hour, 0),
                };
                if hour < 24 {
                    parsed.time = Some(hour * 60 + minute);
                    take = 1 + offset + extra;
                }
            } else if matches!(word, "noon" | "midday") {
                parsed.time = Some(12 * 60);
                take = 1;
            }
        }

        // Parts of a day, only after a day or "this" / "in the", so "Morning run" stays a title.
        if take == 0 {
            let cued = i > 0 && used[i - 1] && parsed.due.is_some();
            let part = |w: &str| match w {
                "morning" | "pagi" => Some(9 * 60),
                "afternoon" | "siang" => Some(14 * 60),
                "sore" => Some(16 * 60),
                "evening" | "malam" => Some(19 * 60),
                _ => None,
            };
            if cued && part(word).is_some() {
                period = part(word);
                take = 1;
            } else if word == "this" && part(next).is_some() {
                period = part(next);
                parsed.due.get_or_insert(settings.today);
                take = 2;
            } else if word == "in" && next == "the" && part(third).is_some() {
                period = part(third);
                take = 3;
            }
        }

        if take > 0 {
            for slot in used.iter_mut().skip(i).take(take) {
                *slot = true;
            }
            // The word that introduced it goes too: "on Friday", "by tomorrow".
            let mut back = i;
            while back > 0
                && !used[back - 1]
                && matches!(
                    at(back - 1),
                    "at" | "on" | "by" | "jam" | "pukul" | "pada" | "for" | "selama"
                )
            {
                back -= 1;
                used[back] = true;
            }
            i += take;
        } else {
            i += 1;
        }
    }

    if parsed.time.is_none() {
        parsed.time = period;
    }
    if parsed.time.is_some() {
        if parsed.due.is_none() {
            let passed = parsed.time.is_some_and(|t| t <= settings.minute);
            parsed.due = Some(settings.today + i64::from(passed));
        }
        parsed.reminder = true;
    }
    if parsed.repeat != "none" && parsed.due.is_none() {
        parsed.due = Some(settings.today);
    }

    let mut title = raw
        .iter()
        .zip(&used)
        .filter(|(_, used)| !**used)
        .map(|(w, _)| *w)
        .collect::<Vec<_>>()
        .join(" ");
    title = title
        .trim_matches([' ', ',', '.', ';', ':', '-'])
        .to_string();
    if title.is_empty() {
        title = segment.trim().to_string();
    }
    parsed.title = capitalize(&truncate(&title, MAX_TITLE));

    if !explicit_list {
        let lower = format!(" {} ", words.join(" "));
        if let Some((id, _)) = KEYWORDS.iter().find(|(id, keys)| {
            settings.lists.iter().any(|(l, _)| l == id)
                && keys.iter().any(|k| lower.contains(&format!(" {k} ")))
        }) {
            parsed.list = (*id).into();
        }
    }
    parsed
}

pub fn parse(text: &str, settings: &Settings) -> Vec<Parsed> {
    segments(text, settings.dump)
        .iter()
        .map(|s| parse_one(s, settings))
        .collect()
}

pub fn truncate(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

pub fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Friday 27 November 2026, 10:00.
    fn today() -> i64 {
        dates::parse_key("2026-11-27").unwrap()
    }

    fn lists() -> Vec<(String, Vec<String>)> {
        ["inbox", "work", "home", "family", "money", "health"]
            .iter()
            .map(|id| {
                (
                    (*id).to_string(),
                    vec![
                        crate::text::list_name(crate::text::Lang::En, id, "").to_lowercase(),
                        crate::text::list_name(crate::text::Lang::Id, id, "").to_lowercase(),
                    ],
                )
            })
            .collect()
    }

    fn one(text: &str) -> Parsed {
        let lists = lists();
        parse_one(
            text,
            &Settings {
                today: today(),
                minute: 600,
                lists: &lists,
                default_list: "inbox",
                dump: false,
            },
        )
    }

    fn all(text: &str, dump: bool) -> Vec<Parsed> {
        let lists = lists();
        parse(
            text,
            &Settings {
                today: today(),
                minute: 600,
                lists: &lists,
                default_list: "inbox",
                dump,
            },
        )
    }

    fn day(p: &Parsed) -> String {
        p.due.map(dates::key).unwrap_or_default()
    }

    #[test]
    fn reads_a_day_time_priority_and_list() {
        let p = one("Call mom tomorrow at 5pm !high #family");
        assert_eq!(p.title, "Call mom");
        assert_eq!(day(&p), "2026-11-28");
        assert_eq!(p.time, Some(17 * 60));
        assert_eq!((p.priority, p.list.as_str()), (3, "family"));
        assert!(p.reminder);
    }

    #[test]
    fn reads_indonesian() {
        let p = one("Telepon ibu besok jam 7 malam");
        assert_eq!(p.title, "Telepon ibu");
        assert_eq!(day(&p), "2026-11-28");
        assert_eq!(p.time, Some(19 * 60));
        assert_eq!(p.list, "family");
        let q = one("Bayar tagihan listrik 3 hari lagi urgent");
        assert_eq!(q.title, "Bayar tagihan listrik");
        assert_eq!(day(&q), "2026-11-30");
        assert_eq!((q.priority, q.list.as_str()), (3, "money"));
    }

    #[test]
    fn an_estimate_is_not_an_hour() {
        let p = one("Write report 2 hours on Monday");
        assert_eq!(p.title, "Write report");
        assert_eq!(p.estimate, 120);
        assert_eq!(p.time, None);
        assert_eq!(day(&p), "2026-11-30");
        let q = one("Rapat jam 2");
        assert_eq!(q.time, Some(14 * 60));
        assert_eq!(q.estimate, 0);
        assert_eq!(one("Stretch 15m").estimate, 15);
    }

    #[test]
    fn a_past_time_means_tomorrow() {
        assert_eq!(day(&one("Standup 9am")), "2026-11-28");
        assert_eq!(day(&one("Lunch 12:30")), "2026-11-27");
    }

    #[test]
    fn morning_in_a_title_stays() {
        let p = one("Morning run");
        assert_eq!(p.title, "Morning run");
        assert_eq!((p.due, p.time), (None, None));
        let q = one("Run tomorrow morning");
        assert_eq!((q.title.as_str(), q.time), ("Run", Some(540)));
    }

    #[test]
    fn repeats_and_dates_by_name() {
        let p = one("Water plants every day");
        assert_eq!(
            (p.title.as_str(), p.repeat.as_str()),
            ("Water plants", "daily")
        );
        assert_eq!(day(&p), "2026-11-27");
        let q = one("Gym setiap senin");
        assert_eq!(
            (q.repeat.as_str(), day(&q).as_str()),
            ("weekly", "2026-11-30")
        );
        assert_eq!(day(&one("Taxes 5 Jan")), "2027-01-05");
        assert_eq!(day(&one("Party Dec 31st")), "2026-12-31");
    }

    #[test]
    fn splits_a_brain_dump() {
        let tasks = all(
            "buy milk, call mom tomorrow and pay rent; send slides then book flights",
            true,
        );
        let titles: Vec<_> = tasks.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(
            titles,
            [
                "Buy milk",
                "Call mom",
                "Pay rent",
                "Send slides",
                "Book flights"
            ]
        );
        assert_eq!(all("Buy salt and pepper", false).len(), 1);
    }

    #[test]
    fn strips_filler_and_survives_odd_input() {
        let p = one("Remind me to stretch");
        assert_eq!(p.title, "Stretch");
        assert!(p.reminder);
        assert_eq!(all("   ", false).len(), 0);
        assert_eq!(one("tomorrow").title, "Tomorrow");
        assert_eq!(one("Ünïcode 📝 task in 99999999999 days").estimate, 0);
        assert!(one(&"a".repeat(500)).title.chars().count() <= MAX_TITLE);
        assert_eq!(all(&"x; ".repeat(40), false).len(), MAX_DRAFTS);
    }
}
