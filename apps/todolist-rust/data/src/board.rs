//! The view models: everything a screen shows is computed here from the document,
//! the person's onboarding answers and the instant the view passes in.

use crate::dates::{self, Local};
use crate::model::{Db, Task};
use crate::parse::{self, Parsed, Settings};
use crate::shapes::*;
use crate::text::{self, Lang};

pub struct Now {
    pub at: f64,
    pub offset: f64,
    pub lang: Lang,
    pub local: Local,
}

impl Now {
    pub fn new(context: &Context) -> Now {
        let now = if context.now.is_finite() {
            context.now
        } else {
            0.0
        };
        let offset = if context.offset.is_finite() {
            context.offset.clamp(-900.0, 900.0)
        } else {
            0.0
        };
        Now {
            at: now,
            offset,
            lang: Lang::from_tag(&context.lang),
            local: dates::local(now, offset),
        }
    }
    pub fn today(&self) -> i64 {
        self.local.day
    }
}

pub fn due_day(task: &Task) -> Option<i64> {
    dates::parse_key(&task.due)
}

pub fn completed_day(task: &Task, offset: f64) -> Option<i64> {
    task.completed_at.map(|at| dates::local(at, offset).day)
}

/// By day, then time, then priority, then age.
pub fn order(a: &Task, b: &Task) -> std::cmp::Ordering {
    let day = |t: &Task| due_day(t).unwrap_or(i64::MAX);
    let time = |t: &Task| dates::parse_time(&t.time).unwrap_or(u32::MAX);
    day(a)
        .cmp(&day(b))
        .then(time(a).cmp(&time(b)))
        .then(b.priority.cmp(&a.priority))
        .then(a.created_at.total_cmp(&b.created_at))
}

/// The order to work in: high priority first, then what is overdue, then the clock.
pub fn urgency(a: &Task, b: &Task) -> std::cmp::Ordering {
    let day = |t: &Task| due_day(t).unwrap_or(i64::MAX);
    let time = |t: &Task| dates::parse_time(&t.time).unwrap_or(u32::MAX);
    b.priority
        .cmp(&a.priority)
        .then(b.flagged.cmp(&a.flagged))
        .then(day(a).cmp(&day(b)))
        .then(time(a).cmp(&time(b)))
        .then(a.created_at.total_cmp(&b.created_at))
}

fn tint_of(color: &str) -> String {
    color.to_string()
}

fn due_label(task: &Task, now: &Now) -> String {
    let Some(day) = due_day(task) else {
        return String::new();
    };
    match dates::parse_time(&task.time) {
        Some(m) if day == now.today() => text::clock(now.lang, m),
        Some(m) => format!(
            "{} · {}",
            text::day_label(now.lang, day, now.today()),
            text::clock(now.lang, m)
        ),
        None => text::day_label(now.lang, day, now.today()),
    }
}

pub fn is_overdue(task: &Task, now: &Now) -> bool {
    if task.done {
        return false;
    }
    match due_day(task) {
        Some(day) if day < now.today() => true,
        Some(day) if day == now.today() => {
            dates::parse_time(&task.time).is_some_and(|m| m < now.local.minute)
        }
        _ => false,
    }
}

pub fn row(db: &Db, task: &Task, now: &Now) -> TaskRow {
    let list = db.list(&task.list).cloned().unwrap_or_default();
    let list_name = text::list_name(now.lang, &list.id, &list.name);
    let sub_done = task.subtasks.iter().filter(|s| s.done).count();
    let mut meta = Vec::new();
    if task.list != "inbox" {
        meta.push(list_name.clone());
    }
    if task.repeat != "none" {
        meta.push(text::repeat_label(now.lang, &task.repeat).to_string());
    }
    if !task.subtasks.is_empty() {
        meta.push(format!("{sub_done}/{}", task.subtasks.len()));
    }
    if task.estimate > 0 {
        meta.push(text::duration(now.lang, task.estimate));
    }
    meta.retain(|m| !m.is_empty());
    TaskRow {
        id: task.id.clone(),
        title: task.title.clone(),
        notes: task.notes.clone(),
        list_id: task.list.clone(),
        list_name,
        list_symbol: if list.symbol.is_empty() {
            "tray.fill".into()
        } else {
            list.symbol.clone()
        },
        list_color: if list.color.is_empty() {
            "blue".into()
        } else {
            tint_of(&list.color)
        },
        due: task.due.clone(),
        time: task.time.clone(),
        due_label: due_label(task, now),
        meta: meta.join(" · "),
        overdue: is_overdue(task, now),
        priority: f64::from(task.priority),
        flagged: task.flagged,
        done: task.done,
        repeat: task.repeat.clone(),
        sub_done: sub_done as f64,
        sub_total: task.subtasks.len() as f64,
        estimate: f64::from(task.estimate),
        reminder: task.reminder,
    }
}

fn rows(db: &Db, tasks: &[&Task], now: &Now) -> Vec<TaskRow> {
    tasks.iter().map(|t| row(db, t, now)).collect()
}

fn section(
    db: &Db,
    id: &str,
    title: &str,
    symbol: &str,
    tint: &str,
    tasks: &[&Task],
    now: &Now,
) -> Section {
    Section {
        id: id.into(),
        title: title.into(),
        symbol: symbol.into(),
        tint: tint.into(),
        tasks: rows(db, tasks, now),
    }
}

/// Open tasks due today or earlier, in the order to work in.
pub fn due_now<'a>(db: &'a Db, now: &Now) -> Vec<&'a Task> {
    let mut tasks: Vec<&Task> = db
        .tasks
        .iter()
        .filter(|t| !t.done && due_day(t).is_some_and(|d| d <= now.today()))
        .collect();
    tasks.sort_by(|a, b| urgency(a, b));
    tasks
}

pub fn lists(db: &Db, now: &Now) -> Vec<ListInfo> {
    db.lists
        .iter()
        .map(|l| {
            let open = db
                .tasks
                .iter()
                .filter(|t| t.list == l.id && !t.done)
                .count();
            let done = db.tasks.iter().filter(|t| t.list == l.id && t.done).count();
            ListInfo {
                id: l.id.clone(),
                name: text::list_name(now.lang, &l.id, &l.name),
                symbol: l.symbol.clone(),
                color: l.color.clone(),
                open: open as f64,
                done: done as f64,
                progress: if open + done == 0 {
                    0.0
                } else {
                    done as f64 / (open + done) as f64
                },
                removable: l.id != "inbox",
            }
        })
        .collect()
}

pub fn streak(db: &Db, now: &Now) -> u32 {
    let days: std::collections::BTreeSet<i64> = db
        .tasks
        .iter()
        .filter_map(|t| completed_day(t, now.offset))
        .collect();
    let mut day = now.today();
    if !days.contains(&day) {
        day -= 1;
    }
    let mut count = 0;
    while days.contains(&day) {
        count += 1;
        day -= 1;
    }
    count
}

pub fn stats(db: &Db, profile: &Profile, now: &Now) -> Stats {
    let today = now.today();
    let done_on = |day: i64| {
        db.tasks
            .iter()
            .filter(|t| completed_day(t, now.offset) == Some(day))
            .count()
    };
    let done_today = done_on(today);
    let due_today = due_now(db, now).len();
    let week: Vec<(i64, usize)> = (0..7)
        .rev()
        .map(|back| (today - back, done_on(today - back)))
        .collect();
    let peak = week.iter().map(|(_, n)| *n).max().unwrap_or(0).max(1);
    let bars = week
        .iter()
        .map(|(day, n)| DayBar {
            key: dates::key(*day),
            label: text::weekday_short(now.lang, *day).to_string(),
            value: *n as f64,
            ratio: *n as f64 / peak as f64,
            today: *day == today,
        })
        .collect();
    let total_done = db.tasks.iter().filter(|t| t.done).count();
    let recent: Vec<&Task> = db
        .tasks
        .iter()
        .filter(|t| due_day(t).is_some_and(|d| d <= today && d > today - 7))
        .collect();
    let rate = if recent.is_empty() {
        0.0
    } else {
        recent.iter().filter(|t| t.done).count() as f64 / recent.len() as f64
    };
    let mut by_weekday = [0usize; 7];
    for day in db.tasks.iter().filter_map(|t| completed_day(t, now.offset)) {
        by_weekday[dates::weekday(day) as usize] += 1;
    }
    let best = (0..7)
        .max_by_key(|i| (by_weekday[*i], std::cmp::Reverse(*i)))
        .filter(|i| by_weekday[*i] > 0);
    let best_day = best
        .map(|i| {
            let monday = today - i64::from(dates::weekday(today));
            text::weekday_name(now.lang, monday + i as i64).to_string()
        })
        .unwrap_or_default();
    let streak = streak(db, now);
    let focus: Vec<_> = db
        .sessions
        .iter()
        .filter(|s| dates::local(s.ended_at, now.offset).day > today - 7)
        .collect();
    let insight = if total_done == 0 {
        now.lang
            .pick(
                "Finish one task to start your streak.",
                "Selesaikan satu tugas untuk memulai rentetanmu.",
            )
            .to_string()
    } else if streak >= 3 {
        match now.lang {
            Lang::En => format!("{streak} days in a row. Protect it with one small task tomorrow."),
            Lang::Id => {
                format!("{streak} hari berturut-turut. Jaga dengan satu tugas kecil besok.")
            }
        }
    } else if total_done >= 5 {
        match now.lang {
            Lang::En => {
                format!("You finish the most on {best_day}s. Save the hard task for that day.")
            }
            Lang::Id => format!(
                "Kamu paling produktif di hari {best_day}. Simpan tugas berat untuk hari itu."
            ),
        }
    } else {
        now.lang
            .pick(
                "Good start. Patterns show up after a few days.",
                "Awal yang bagus. Polanya muncul setelah beberapa hari.",
            )
            .to_string()
    };
    Stats {
        done_today: done_today as f64,
        due_today: due_today as f64,
        goal: profile.capacity.clamp(1.0, 20.0),
        ring: if done_today + due_today == 0 {
            0.0
        } else {
            done_today as f64 / (done_today + due_today) as f64
        },
        streak: f64::from(streak),
        week_done: week.iter().map(|(_, n)| *n as f64).sum(),
        total_done: total_done as f64,
        focus_minutes: focus.iter().map(|s| f64::from(s.minutes)).sum(),
        focus_sessions: focus.len() as f64,
        rate,
        bars,
        best_day,
        insight,
    }
}

/// Minutes after midnight in the focus window the person named, as `[from, to)`.
pub fn peak_window(peak: &str) -> (u32, u32) {
    match peak {
        "early" => (5 * 60, 9 * 60),
        "midday" => (12 * 60, 14 * 60),
        "afternoon" => (14 * 60, 18 * 60),
        "evening" => (18 * 60, 22 * 60),
        "night" => (22 * 60, 26 * 60),
        _ => (9 * 60, 12 * 60),
    }
}

pub fn in_window(minute: u32, (from, to): (u32, u32)) -> bool {
    (from..to).contains(&minute) || (to > 1440 && minute < to - 1440)
}

fn weekly_day(weekly: &str) -> u32 {
    match weekly {
        "sunday" => 6,
        "monday" => 0,
        _ => 4,
    }
}

fn suggestions(db: &Db, profile: &Profile, now: &Now) -> Vec<Suggestion> {
    let due = due_now(db, now);
    let overdue = due
        .iter()
        .filter(|t| due_day(t).is_some_and(|d| d < now.today()))
        .count();
    let lang = now.lang;
    let mut out = Vec::new();
    let mut push = |kind: &str,
                    title: String,
                    body: String,
                    symbol: &str,
                    tint: &str,
                    task: &str,
                    action: String| {
        out.push(Suggestion {
            kind: kind.into(),
            title,
            body,
            symbol: symbol.into(),
            tint: tint.into(),
            task_id: task.into(),
            action,
        });
    };
    let capacity = profile.capacity.clamp(1.0, 20.0) as usize;
    if due.len() > capacity {
        let extra = due.len() - capacity;
        push(
            "balance",
            lang.pick("Today looks full", "Hari ini terlalu padat")
                .into(),
            match lang {
                Lang::En => format!(
                    "{} on your plate, you like {capacity}. Move {extra} to tomorrow.",
                    text::count_tasks(lang, due.len())
                ),
                Lang::Id => format!(
                    "Ada {} padahal batasmu {capacity}. Pindahkan {extra} ke besok.",
                    text::count_tasks(lang, due.len())
                ),
            },
            "scalemass.fill",
            "orange",
            "",
            lang.pick("Balance my day", "Seimbangkan hari").into(),
        );
    }
    if overdue >= 2 {
        push(
            "catchup",
            lang.pick("Fresh start", "Mulai segar").into(),
            match lang {
                Lang::En => {
                    format!("{overdue} tasks slipped. Bring them to today without the guilt.")
                }
                Lang::Id => {
                    format!("{overdue} tugas terlewat. Bawa ke hari ini, tanpa rasa bersalah.")
                }
            },
            "arrow.uturn.forward.circle.fill",
            "red",
            "",
            lang.pick("Move to today", "Pindah ke hari ini").into(),
        );
    }
    let next = due.first().copied();
    if let Some(task) = next {
        if in_window(now.local.minute, peak_window(&profile.peak)) {
            push(
                "focus",
                lang.pick("You're in your focus window", "Ini jam fokusmu")
                    .into(),
                match lang {
                    Lang::En => {
                        format!("Give “{}” your best {} minutes.", task.title, profile.focus)
                    }
                    Lang::Id => format!("Beri “{}” {} menit terbaikmu.", task.title, profile.focus),
                },
                "bolt.fill",
                "indigo",
                &task.id,
                lang.pick("Start focus", "Mulai fokus").into(),
            );
        } else if profile.struggles.iter().any(|s| s == "procrastination") {
            push(
                "twomin",
                lang.pick("Just two minutes", "Cukup dua menit").into(),
                match lang {
                    Lang::En => format!(
                        "Open “{}” and do two minutes. Stopping is allowed.",
                        task.title
                    ),
                    Lang::Id => format!(
                        "Buka “{}” dan kerjakan dua menit. Boleh berhenti.",
                        task.title
                    ),
                },
                "hourglass",
                "purple",
                &task.id,
                lang.pick("Start 2 minutes", "Mulai 2 menit").into(),
            );
        } else if due.len() >= 2
            && profile
                .struggles
                .iter()
                .any(|s| s == "starting" || s == "overwhelm")
        {
            push(
                "pick",
                lang.pick("Not sure where to start?", "Bingung mulai dari mana?")
                    .into(),
                match lang {
                    Lang::En => format!("Start with “{}”. It matters most right now.", task.title),
                    Lang::Id => format!(
                        "Mulai dari “{}”. Itu yang paling penting sekarang.",
                        task.title
                    ),
                },
                "wand.and.stars",
                "pink",
                &task.id,
                lang.pick("Open it", "Buka").into(),
            );
        }
    }
    if dates::weekday(now.today()) == weekly_day(&profile.weekly) && db.tasks.iter().any(|t| t.done)
    {
        push(
            "review",
            lang.pick("Weekly review day", "Hari tinjauan mingguan")
                .into(),
            lang.pick(
                "Two minutes to see what moved this week.",
                "Dua menit untuk melihat kemajuan minggu ini.",
            )
            .into(),
            "chart.line.uptrend.xyaxis",
            "green",
            "",
            lang.pick("See my week", "Lihat mingguku").into(),
        );
    }
    let wind = dates::parse_time(&profile.wind).unwrap_or(22 * 60);
    if now.local.minute + 60 >= wind {
        push(
            "tomorrow",
            lang.pick("Close your day", "Tutup harimu").into(),
            lang.pick(
                "Write tomorrow's list now and let your mind rest.",
                "Tulis daftar besok sekarang, biar pikiran bisa istirahat.",
            )
            .into(),
            "moon.stars.fill",
            "indigo",
            "",
            lang.pick("Plan tomorrow", "Rencanakan besok").into(),
        );
    }
    out.truncate(2);
    out
}

fn greeting(profile: &Profile, now: &Now) -> String {
    let hello = match now.local.minute / 60 {
        4..=10 => now.lang.pick("Good morning", "Selamat pagi"),
        11..=14 => now.lang.pick("Good afternoon", "Selamat siang"),
        15..=17 => now.lang.pick("Good afternoon", "Selamat sore"),
        18..=22 => now.lang.pick("Good evening", "Selamat malam"),
        _ => now.lang.pick("Late night", "Masih terjaga"),
    };
    let name = profile.name.trim();
    if name.is_empty() {
        hello.to_string()
    } else {
        format!("{hello}, {}", parse::truncate(name, 24))
    }
}

pub fn board(db: &Db, profile: &Profile, now: &Now) -> Board {
    let today = now.today();
    let lang = now.lang;
    let mut overdue: Vec<&Task> = db
        .tasks
        .iter()
        .filter(|t| !t.done && due_day(t).is_some_and(|d| d < today))
        .collect();
    overdue.sort_by(|a, b| order(a, b));
    let mut current: Vec<&Task> = db
        .tasks
        .iter()
        .filter(|t| !t.done && due_day(t) == Some(today))
        .collect();
    current.sort_by(|a, b| order(a, b));
    let mut done: Vec<&Task> = db
        .tasks
        .iter()
        .filter(|t| t.done && completed_day(t, now.offset) == Some(today))
        .collect();
    done.sort_by(|a, b| {
        b.completed_at
            .unwrap_or(0.0)
            .total_cmp(&a.completed_at.unwrap_or(0.0))
    });

    let mut sections = Vec::new();
    if !overdue.is_empty() {
        sections.push(section(
            db,
            "overdue",
            lang.pick("Overdue", "Terlewat"),
            "exclamationmark.circle.fill",
            "red",
            &overdue,
            now,
        ));
    }
    if !current.is_empty() {
        sections.push(section(
            db,
            "today",
            lang.pick("Today", "Hari ini"),
            "sun.max.fill",
            "orange",
            &current,
            now,
        ));
    }
    if !done.is_empty() {
        sections.push(section(
            db,
            "done",
            lang.pick("Completed", "Selesai"),
            "checkmark.circle.fill",
            "green",
            &done,
            now,
        ));
    }

    let mut upcoming = Vec::new();
    for ahead in 1..=7 {
        let day = today + ahead;
        let mut tasks: Vec<&Task> = db
            .tasks
            .iter()
            .filter(|t| !t.done && due_day(t) == Some(day))
            .collect();
        if tasks.is_empty() {
            continue;
        }
        tasks.sort_by(|a, b| order(a, b));
        let title = format!(
            "{} · {}",
            text::day_label(lang, day, today),
            text::short_date(lang, day)
        );
        upcoming.push(section(
            db,
            &dates::key(day),
            &title,
            "calendar",
            "blue",
            &tasks,
            now,
        ));
    }
    let mut later: Vec<&Task> = db
        .tasks
        .iter()
        .filter(|t| !t.done && due_day(t).is_none_or(|d| d > today + 7))
        .collect();
    later.sort_by(|a, b| order(a, b));

    let due = due_now(db, now);
    let open = due.len();
    let done_today = done.len();
    let minutes: u32 = due.iter().map(|t| t.estimate).sum();
    let headline = if open == 0 && done_today > 0 {
        lang.pick("All clear", "Beres semua").to_string()
    } else if open == 0 {
        lang.pick("A clear day", "Hari yang lapang").to_string()
    } else {
        match lang {
            Lang::En => format!("{} for today", text::count_tasks(lang, open)),
            Lang::Id => format!("{} untuk hari ini", text::count_tasks(lang, open)),
        }
    };
    let subline = if minutes > 0 {
        match lang {
            Lang::En => format!("About {} of work", text::duration(lang, minutes)),
            Lang::Id => format!("Sekitar {} kerja", text::duration(lang, minutes)),
        }
    } else if open == 0 && done_today > 0 {
        match lang {
            Lang::En => format!(
                "{} done. Enjoy the quiet.",
                text::count_tasks(lang, done_today)
            ),
            Lang::Id => format!(
                "{} selesai. Nikmati tenangnya.",
                text::count_tasks(lang, done_today)
            ),
        }
    } else if done_today > 0 {
        match lang {
            Lang::En => format!("{done_today} done so far"),
            Lang::Id => format!("{done_today} sudah selesai"),
        }
    } else if open == 0 {
        lang.pick("Capture what's on your mind", "Tangkap isi pikiranmu")
            .to_string()
    } else {
        lang.pick("One at a time", "Satu per satu").to_string()
    };
    let top: Vec<&Task> = due.iter().take(3).copied().collect();
    let next = due.first().copied().or_else(|| later.first().copied());

    let mut open_tasks: Vec<&Task> = db.tasks.iter().filter(|t| !t.done).collect();
    open_tasks.sort_by(|a, b| order(a, b));
    let spotlight = open_tasks
        .iter()
        .take(60)
        .map(|t| {
            let r = row(db, t, now);
            SpotEntry {
                id: t.id.clone(),
                title: t.title.clone(),
                detail: [r.due_label, r.list_name]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" · "),
            }
        })
        .collect();

    let week: Vec<(i64, usize)> = (0..7)
        .map(|ahead| {
            let day = today + ahead;
            (
                day,
                db.tasks
                    .iter()
                    .filter(|t| !t.done && due_day(t) == Some(day))
                    .count(),
            )
        })
        .collect();
    let busiest = week.iter().map(|(_, n)| *n).max().unwrap_or(0).max(1);
    let week = week
        .into_iter()
        .map(|(day, n)| DayBar {
            key: dates::key(day),
            label: text::weekday_short(lang, day).to_string(),
            value: n as f64,
            ratio: n as f64 / busiest as f64,
            today: day == today,
        })
        .collect();
    let mut share = format!("Lumen · {}\n", text::long_date(lang, today));
    for t in current.iter().chain(overdue.iter()).take(20) {
        share.push_str(&format!("○ {}\n", t.title));
    }
    for t in done.iter().take(20) {
        share.push_str(&format!("● {}\n", t.title));
    }

    Board {
        ready: true,
        error: String::new(),
        greeting: greeting(profile, now),
        date_label: text::long_date(lang, today),
        headline,
        subline,
        today: sections,
        upcoming,
        later: rows(db, &later, now),
        lists: lists(db, now),
        stats: stats(db, profile, now),
        top: rows(db, &top, now),
        suggestions: suggestions(db, profile, now),
        open_count: open_tasks.len() as f64,
        next_id: next.map(|t| t.id.clone()).unwrap_or_default(),
        next_title: next.map(|t| t.title.clone()).unwrap_or_default(),
        spotlight,
        all_clear: open == 0 && done_today > 0,
        week,
        share,
    }
}

pub fn search(db: &Db, query: &str, filter: &str, now: &Now) -> Search {
    let needle = query.trim().to_lowercase();
    let mut tasks: Vec<&Task> = db
        .tasks
        .iter()
        .filter(|t| match filter {
            "open" => !t.done,
            "done" => t.done,
            "flagged" => t.flagged && !t.done,
            "high" => t.priority == 3 && !t.done,
            _ => true,
        })
        .filter(|t| {
            if needle.is_empty() {
                return true;
            }
            let list = db
                .list(&t.list)
                .map(|l| text::list_name(now.lang, &l.id, &l.name))
                .unwrap_or_default();
            t.title.to_lowercase().contains(&needle)
                || t.notes.to_lowercase().contains(&needle)
                || list.to_lowercase().contains(&needle)
                || t.subtasks
                    .iter()
                    .any(|s| s.title.to_lowercase().contains(&needle))
        })
        .collect();
    tasks.sort_by(|a, b| a.done.cmp(&b.done).then(order(a, b)));
    let count = tasks.len();
    tasks.truncate(100);
    Search {
        tasks: rows(db, &tasks, now),
        summary: match now.lang {
            Lang::En if count == 1 => "1 result".into(),
            Lang::En => format!("{count} results"),
            Lang::Id => format!("{count} hasil"),
        },
    }
}

pub fn list_page(db: &Db, id: &str, now: &Now) -> ListPage {
    let Some(list) = db.list(id) else {
        return ListPage::default();
    };
    let mut open: Vec<&Task> = db
        .tasks
        .iter()
        .filter(|t| t.list == id && !t.done)
        .collect();
    open.sort_by(|a, b| order(a, b));
    let mut done: Vec<&Task> = db.tasks.iter().filter(|t| t.list == id && t.done).collect();
    done.sort_by(|a, b| {
        b.completed_at
            .unwrap_or(0.0)
            .total_cmp(&a.completed_at.unwrap_or(0.0))
    });
    let finished = done.len();
    done.truncate(30);
    ListPage {
        found: true,
        id: id.into(),
        name: text::list_name(now.lang, &list.id, &list.name),
        symbol: list.symbol.clone(),
        color: list.color.clone(),
        summary: match now.lang {
            Lang::En => format!("{} open · {finished} done", open.len()),
            Lang::Id => format!("{} aktif · {finished} selesai", open.len()),
        },
        open: rows(db, &open, now),
        done: rows(db, &done, now),
        removable: id != "inbox",
    }
}

pub fn detail(db: &Db, id: &str, now: &Now) -> Detail {
    let Some(task) = db.task(id) else {
        return Detail::default();
    };
    let created = text::day_label(
        now.lang,
        dates::local(task.created_at, now.offset).day,
        now.today(),
    );
    Detail {
        found: true,
        task: row(db, task, now),
        subtasks: task
            .subtasks
            .iter()
            .map(|s| SubRow {
                id: s.id.clone(),
                title: s.title.clone(),
                done: s.done,
            })
            .collect(),
        created: match now.lang {
            Lang::En => format!("Added {created}"),
            Lang::Id => format!("Ditambahkan {created}"),
        },
    }
}

pub fn list_names(db: &Db) -> Vec<(String, Vec<String>)> {
    db.lists
        .iter()
        .map(|l| {
            let mut names = vec![
                text::list_name(Lang::En, &l.id, &l.name)
                    .to_lowercase()
                    .replace(' ', ""),
                text::list_name(Lang::Id, &l.id, &l.name)
                    .to_lowercase()
                    .replace(' ', ""),
            ];
            names.dedup();
            (l.id.clone(), names)
        })
        .collect()
}

pub fn draft(db: &Db, parsed: &Parsed, now: &Now) -> Draft {
    let list = db
        .list(&parsed.list)
        .or_else(|| db.list("inbox"))
        .cloned()
        .unwrap_or_default();
    let mut label = Vec::new();
    if let Some(day) = parsed.due {
        label.push(text::day_label(now.lang, day, now.today()));
    }
    if let Some(minute) = parsed.time {
        label.push(text::clock(now.lang, minute));
    }
    if parsed.repeat != "none" {
        label.push(text::repeat_label(now.lang, &parsed.repeat).into());
    }
    if parsed.estimate > 0 {
        label.push(text::duration(now.lang, parsed.estimate));
    }
    Draft {
        title: parsed.title.clone(),
        due: parsed.due.map(dates::key).unwrap_or_default(),
        time: parsed.time.map(dates::time_key).unwrap_or_default(),
        priority: f64::from(parsed.priority),
        list_name: text::list_name(now.lang, &list.id, &list.name),
        list_symbol: list.symbol.clone(),
        list_color: list.color.clone(),
        list_id: list.id,
        repeat: parsed.repeat.clone(),
        estimate: f64::from(parsed.estimate),
        label: label.join(" · "),
    }
}

fn preview_of(drafts: Vec<Draft>, now: &Now) -> Preview {
    let hint = match (now.lang, drafts.len()) {
        (_, 0) => String::new(),
        (Lang::En, 1) => "1 task ready".into(),
        (Lang::En, n) => format!("{n} tasks ready"),
        (Lang::Id, n) => format!("{n} tugas siap"),
    };
    Preview { drafts, hint }
}

pub fn preview(db: &Db, text: &str, dump: bool, default_list: &str, now: &Now) -> Preview {
    let names = list_names(db);
    let default = if db.list(default_list).is_some() {
        default_list
    } else {
        "inbox"
    };
    let settings = Settings {
        today: now.today(),
        minute: now.local.minute,
        lists: &names,
        default_list: default,
        dump,
    };
    let text = parse::truncate(text, 2000);
    preview_of(
        parse::parse(&text, &settings)
            .iter()
            .map(|p| draft(db, p, now))
            .collect(),
        now,
    )
}

/// The on-device model's tasks, held to what this app can keep: its words for a
/// time go through the same parser, so a day it invents in another form is dropped.
pub fn ai_preview(db: &Db, tasks: &[AiTask], default_list: &str, now: &Now) -> Preview {
    let names = list_names(db);
    let default = if db.list(default_list).is_some() {
        default_list
    } else {
        "inbox"
    };
    let settings = Settings {
        today: now.today(),
        minute: now.local.minute,
        lists: &names,
        default_list: default,
        dump: false,
    };
    let drafts = tasks
        .iter()
        .filter(|t| !t.title.trim().is_empty())
        .take(parse::MAX_DRAFTS)
        .map(|t| {
            let title = parse::truncate(t.title.trim(), parse::MAX_TITLE);
            let mut parsed = parse::parse_one(&title, &settings);
            let when = parse::parse_one(&format!("x {}", parse::truncate(&t.when, 80)), &settings);
            parsed.due = when.due.or(parsed.due);
            parsed.time = when.time.or(parsed.time);
            if when.repeat != "none" {
                parsed.repeat = when.repeat;
            }
            parsed.priority = match t.priority.to_lowercase().as_str() {
                "high" => 3,
                "medium" => 2,
                "low" => 1,
                _ => parsed.priority,
            };
            draft(db, &parsed, now)
        })
        .collect();
    preview_of(drafts, now)
}

fn peak_name(lang: Lang, peak: &str) -> &'static str {
    match (peak, lang) {
        ("early", Lang::En) => "early morning",
        ("early", Lang::Id) => "pagi buta",
        ("midday", Lang::En) => "midday",
        ("midday", Lang::Id) => "tengah hari",
        ("afternoon", Lang::En) => "afternoon",
        ("afternoon", Lang::Id) => "sore",
        ("evening", Lang::En) => "evening",
        ("evening", Lang::Id) => "petang",
        ("night", Lang::En) => "night",
        ("night", Lang::Id) => "malam",
        (_, Lang::En) => "morning",
        (_, Lang::Id) => "pagi",
    }
}

fn weekly_name(lang: Lang, weekly: &str) -> &'static str {
    match (weekly, lang) {
        ("sunday", Lang::En) => "Sunday",
        ("sunday", Lang::Id) => "Minggu",
        ("monday", Lang::En) => "Monday",
        ("monday", Lang::Id) => "Senin",
        (_, Lang::En) => "Friday",
        (_, Lang::Id) => "Jumat",
    }
}

fn plan_row(id: &str, symbol: &str, tint: &str, title: String, body: String) -> PlanRow {
    PlanRow {
        id: id.into(),
        symbol: symbol.into(),
        tint: tint.into(),
        title,
        body,
    }
}

/// The personal plan the onboarding answers add up to. Its numbers come from here,
/// never from the model.
pub fn plan(profile: &Profile, now: &Now) -> Plan {
    let lang = now.lang;
    let en = lang == Lang::En;
    let wake = dates::parse_time(&profile.wake).unwrap_or(7 * 60);
    let wind = dates::parse_time(&profile.wind).unwrap_or(22 * 60);
    let capacity = profile.capacity.clamp(1.0, 20.0) as u32;
    let focus = profile.focus.clamp(5.0, 180.0) as u32;
    let peak = peak_name(lang, &profile.peak);
    let name = parse::truncate(profile.name.trim(), 24);
    let title = match (en, name.is_empty()) {
        (true, true) => "Your plan is ready".to_string(),
        (true, false) => format!("{name}, your plan is ready"),
        (false, true) => "Rencanamu sudah siap".to_string(),
        (false, false) => format!("{name}, rencanamu sudah siap"),
    };
    let mut rows = vec![
        plan_row(
            "capacity",
            "gauge.with.needle.fill",
            "indigo",
            if en {
                format!("Up to {capacity} tasks a day")
            } else {
                format!("Maksimal {capacity} tugas sehari")
            },
            lang.pick(
                "Lumen warns you before a day overflows.",
                "Lumen mengingatkan sebelum harimu kepenuhan.",
            )
            .into(),
        ),
        plan_row(
            "focus",
            "bolt.fill",
            "orange",
            if en {
                format!("Focus in the {peak}")
            } else {
                format!("Fokus di waktu {peak}")
            },
            if en {
                format!("{focus} minute sessions when you are sharpest.")
            } else {
                format!("Sesi {focus} menit saat pikiranmu paling tajam.")
            },
        ),
        plan_row(
            "rhythm",
            "sunrise.fill",
            "pink",
            if en {
                format!("Morning plan at {}", text::clock(lang, wake))
            } else {
                format!("Rencana pagi pukul {}", text::clock(lang, wake))
            },
            if en {
                format!("Evening review at {}.", text::clock(lang, wind))
            } else {
                format!("Tinjauan malam pukul {}.", text::clock(lang, wind))
            },
        ),
    ];
    let (capture_title, capture_body, capture_symbol) = match profile.capture.as_str() {
        "speak" => (
            lang.pick("Voice first", "Suara dulu"),
            lang.pick("The mic is one tap away.", "Mikrofon cukup satu ketuk."),
            "waveform",
        ),
        "dump" => (
            lang.pick("Brain dump", "Curah pikiran"),
            lang.pick(
                "Pour it all in. Lumen sorts it into tasks.",
                "Tuang semuanya. Lumen memilahnya jadi tugas.",
            ),
            "sparkles",
        ),
        _ => (
            lang.pick("Type naturally", "Ketik biasa saja"),
            lang.pick(
                "“Call mom Friday 5pm” just works.",
                "“Telepon ibu Jumat jam 5” langsung jadi.",
            ),
            "keyboard.fill",
        ),
    };
    rows.push(plan_row(
        "capture",
        capture_symbol,
        "teal",
        capture_title.into(),
        capture_body.into(),
    ));
    let (r_title, r_body, r_symbol) = match profile.reminders.as_str() {
        "persistent" => (
            lang.pick("Persistent reminders", "Pengingat gigih"),
            lang.pick(
                "A second nudge 15 minutes later.",
                "Diingatkan lagi 15 menit kemudian.",
            ),
            "bell.badge.fill",
        ),
        "none" => (
            lang.pick("No reminders", "Tanpa pengingat"),
            lang.pick(
                "Quiet by choice. Change it any time.",
                "Tenang sesuai pilihanmu. Bisa diubah kapan saja.",
            ),
            "bell.slash.fill",
        ),
        _ => (
            lang.pick("Gentle reminders", "Pengingat lembut"),
            lang.pick("One nudge, right on time.", "Satu pengingat, tepat waktu."),
            "bell.fill",
        ),
    };
    rows.push(plan_row(
        "reminders",
        r_symbol,
        "red",
        r_title.into(),
        r_body.into(),
    ));
    for struggle in profile.struggles.iter().take(2) {
        let row = match struggle.as_str() {
            "forgetting" => Some((
                "tray.and.arrow.down.fill",
                lang.pick("For forgetting", "Untuk lupa"),
                lang.pick(
                    "Reminders on any task with a time.",
                    "Pengingat di setiap tugas berwaktu.",
                ),
            )),
            "overwhelm" => Some((
                "square.3.layers.3d.down.right",
                lang.pick("For overwhelm", "Untuk kewalahan"),
                lang.pick(
                    "A Top 3 card shows only what matters.",
                    "Kartu 3 Teratas hanya menampilkan yang penting.",
                ),
            )),
            "procrastination" => Some((
                "hourglass",
                lang.pick("For putting things off", "Untuk menunda"),
                lang.pick("A two minute start button.", "Tombol mulai dua menit."),
            )),
            "scattered" => Some((
                "square.and.arrow.down.on.square.fill",
                lang.pick("For scattered notes", "Untuk catatan tercecer"),
                lang.pick(
                    "One inbox, reachable from Siri and Control Center.",
                    "Satu kotak masuk, bisa dari Siri dan Control Center.",
                ),
            )),
            "time" => Some((
                "clock.badge.checkmark.fill",
                lang.pick("For time pressure", "Untuk waktu sempit"),
                lang.pick(
                    "Estimates add up before you commit.",
                    "Estimasi dijumlahkan sebelum kamu berkomitmen.",
                ),
            )),
            "starting" => Some((
                "wand.and.stars",
                lang.pick("For getting started", "Untuk memulai"),
                lang.pick(
                    "Lumen picks the next task for you.",
                    "Lumen memilihkan tugas berikutnya.",
                ),
            )),
            _ => None,
        };
        if let Some((symbol, title, body)) = row {
            rows.push(plan_row(
                struggle,
                symbol,
                "purple",
                title.into(),
                body.into(),
            ));
        }
    }
    rows.push(plan_row(
        "weekly",
        "calendar.badge.checkmark",
        "green",
        if en {
            format!("Weekly review on {}", weekly_name(lang, &profile.weekly))
        } else {
            format!(
                "Tinjauan mingguan hari {}",
                weekly_name(lang, &profile.weekly)
            )
        },
        lang.pick(
            "See what moved and reset.",
            "Lihat kemajuan dan mulai lagi.",
        )
        .into(),
    ));
    let summary = if en {
        format!(
            "Your day starts at {} with a short plan of up to {capacity} tasks. Your {peak} hours are for deep work in {focus} minute sessions, and at {} you close the day.",
            text::clock(lang, wake),
            text::clock(lang, wind)
        )
    } else {
        format!(
            "Harimu mulai pukul {} dengan rencana singkat maksimal {capacity} tugas. Waktu {peak} untuk kerja mendalam dalam sesi {focus} menit, dan pukul {} kamu menutup hari.",
            text::clock(lang, wake),
            text::clock(lang, wind)
        )
    };
    let prompt = format!(
        "Write two short warm sentences in {} that describe this daily plan to {}. Use only these facts and numbers: morning plan at {}, at most {capacity} tasks a day, focus in the {peak}, {focus} minute focus sessions, evening review at {}. No lists, no emoji, no medical or performance promises.",
        lang.pick("English", "Indonesian"),
        if name.is_empty() { "the person".to_string() } else { name },
        text::clock(Lang::En, wake),
        text::clock(Lang::En, wind),
    );
    let db = Db::fresh(&profile.areas);
    Plan {
        title,
        summary,
        prompt,
        rows,
        lists: lists(&db, now),
    }
}

fn numbers(text: &str) -> Vec<u32> {
    text.split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect()
}

/// The model's sentence only if it is short, finished and says no number the plan
/// does not have; else the plan's own words.
pub fn checked_summary(model: &str, profile: &Profile, now: &Now) -> String {
    let plan = plan(profile, now);
    let model = model.trim();
    let mut allowed = numbers(&plan.summary);
    allowed.extend(numbers(&plan.prompt));
    let length = model.chars().count();
    let ok = (40..=360).contains(&length)
        && model.ends_with(['.', '!'])
        && !model.contains(['\n', '*', '#', '{'])
        && numbers(model).iter().all(|n| allowed.contains(n));
    if ok {
        model.to_string()
    } else {
        plan.summary
    }
}

fn notice(tag: &str, title: &str, body: String, at: f64, navigate: &str) -> Notice {
    Notice {
        tag: tag.into(),
        title: title.into(),
        body,
        at,
        cancel: false,
        navigate: navigate.into(),
    }
}

pub fn cancel(tag: &str) -> Notice {
    Notice {
        tag: tag.into(),
        cancel: true,
        ..Notice::default()
    }
}

/// A task's reminders as they should stand now.
pub fn task_notices(db: &Db, task: &Task, profile: &Profile, now: &Now) -> Vec<Notice> {
    let tag = format!("task-{}", task.id);
    let again = format!("task-{}-again", task.id);
    let at = match (due_day(task), dates::parse_time(&task.time)) {
        (Some(day), Some(minute)) => Some(dates::instant(day, minute, now.offset)),
        _ => None,
    };
    match at {
        Some(at) if !task.done && task.reminder && profile.reminders != "none" && at > now.at => {
            let list = db
                .list(&task.list)
                .map(|l| text::list_name(now.lang, &l.id, &l.name))
                .unwrap_or_default();
            let body = format!("{} · {list}", now.lang.pick("Due now", "Waktunya sekarang"));
            let link = format!("/task/{}", task.id);
            let mut out = vec![notice(&tag, &task.title, body, at, &link)];
            if profile.reminders == "persistent" {
                out.push(notice(
                    &again,
                    &task.title,
                    now.lang
                        .pick("Still waiting for you", "Masih menunggumu")
                        .into(),
                    at + 15.0 * 60_000.0,
                    &link,
                ));
            } else {
                out.push(cancel(&again));
            }
            out
        }
        _ => vec![cancel(&tag), cancel(&again)],
    }
}

/// The next morning plan, evening review and weekly review.
pub fn nudges(db: &Db, profile: &Profile, now: &Now) -> Vec<Notice> {
    if profile.reminders == "none" {
        return vec![
            cancel("nudge-morning"),
            cancel("nudge-evening"),
            cancel("nudge-weekly"),
        ];
    }
    let next = |minute: u32| {
        let day = if minute > now.local.minute {
            now.today()
        } else {
            now.today() + 1
        };
        (day, dates::instant(day, minute, now.offset))
    };
    let (morning_day, morning) = next(dates::parse_time(&profile.wake).unwrap_or(7 * 60));
    let waiting = db
        .tasks
        .iter()
        .filter(|t| !t.done && due_day(t).is_some_and(|d| d <= morning_day))
        .count();
    let wind = dates::parse_time(&profile.wind).unwrap_or(22 * 60);
    let (_, evening) = next(wind);
    let weekday = weekly_day(&profile.weekly);
    let ahead = (i64::from(weekday) - i64::from(dates::weekday(now.today()))).rem_euclid(7);
    let review_minute = wind.saturating_sub(120);
    let ahead = if ahead == 0 && review_minute <= now.local.minute {
        7
    } else {
        ahead
    };
    let weekly = dates::instant(now.today() + ahead, review_minute, now.offset);
    let lang = now.lang;
    // Every upcoming task reminder too: iOS keeps 64 pending, so the soonest 40.
    let mut tasks: Vec<Notice> = db
        .tasks
        .iter()
        .flat_map(|t| task_notices(db, t, profile, now))
        .filter(|n| !n.cancel)
        .collect();
    tasks.sort_by(|a, b| a.at.total_cmp(&b.at));
    tasks.truncate(40);
    let mut out = vec![
        notice(
            "nudge-morning",
            lang.pick("Your plan for today", "Rencanamu hari ini"),
            if waiting == 0 {
                lang.pick(
                    "A clear day. Add what matters.",
                    "Hari lapang. Tambahkan yang penting.",
                )
                .into()
            } else {
                match lang {
                    Lang::En => format!(
                        "{} waiting. Pick your first one.",
                        text::count_tasks(lang, waiting)
                    ),
                    Lang::Id => format!(
                        "{} menunggu. Pilih yang pertama.",
                        text::count_tasks(lang, waiting)
                    ),
                }
            },
            morning,
            "/",
        ),
        notice(
            "nudge-evening",
            lang.pick("Close your day", "Tutup harimu"),
            lang.pick(
                "Review today and set up tomorrow.",
                "Tinjau hari ini dan siapkan besok.",
            )
            .into(),
            evening,
            "/",
        ),
        notice(
            "nudge-weekly",
            lang.pick("Weekly review", "Tinjauan mingguan"),
            lang.pick("See what moved this week.", "Lihat kemajuanmu minggu ini.")
                .into(),
            weekly,
            "/insights",
        ),
    ];
    out.extend(tasks);
    out
}
