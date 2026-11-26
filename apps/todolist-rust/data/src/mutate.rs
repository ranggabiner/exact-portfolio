//! Every change to the document. Each answers a `Reply`: whether it happened,
//! what to tell the person, and which reminders to post or withdraw.

use crate::board::{self, Now};
use crate::dates;
use crate::model::{Db, List, Session, Sub, Task, BUILTIN};
use crate::parse;
use crate::shapes::*;
use crate::text::{self, Lang};

pub const MAX_TASKS: usize = 5_000;
const MAX_NOTES: usize = 4_000;
const MAX_SUBTASKS: usize = 50;
const MAX_LISTS: usize = 30;
const COLORS: [&str; 8] = [
    "blue", "indigo", "purple", "pink", "red", "orange", "green", "teal",
];
const SYMBOLS: [&str; 12] = [
    "list.bullet",
    "star.fill",
    "bookmark.fill",
    "airplane",
    "cart.fill",
    "dumbbell.fill",
    "book.fill",
    "pawprint.fill",
    "leaf.fill",
    "gift.fill",
    "lightbulb.fill",
    "music.note",
];

pub fn refuse(message: &str) -> Reply {
    Reply {
        ok: false,
        message: message.into(),
        ..Reply::default()
    }
}

fn done(message: impl Into<String>, id: &str) -> Reply {
    Reply {
        ok: true,
        message: message.into(),
        id: id.into(),
        ..Reply::default()
    }
}

fn total_done(db: &Db) -> f64 {
    db.tasks.iter().filter(|t| t.done).count() as f64
}

fn missing(lang: Lang) -> Reply {
    refuse(lang.pick("That task is no longer here.", "Tugas itu sudah tidak ada."))
}

fn from_draft(db: &mut Db, draft: &Draft, now: &Now) -> Option<Task> {
    let title = parse::truncate(draft.title.trim(), parse::MAX_TITLE);
    if title.is_empty() || db.tasks.len() >= MAX_TASKS {
        return None;
    }
    let list = if db.list(&draft.list_id).is_some() {
        draft.list_id.clone()
    } else {
        "inbox".into()
    };
    let due = dates::parse_key(&draft.due)
        .map(dates::key)
        .unwrap_or_default();
    let time = if due.is_empty() {
        String::new()
    } else {
        dates::parse_time(&draft.time)
            .map(dates::time_key)
            .unwrap_or_default()
    };
    let repeat = match draft.repeat.as_str() {
        r @ ("daily" | "weekdays" | "weekly" | "monthly") if !due.is_empty() => r,
        _ => "none",
    };
    Some(Task {
        id: db.next_id("t"),
        title,
        list,
        reminder: !time.is_empty(),
        due,
        time,
        priority: draft.priority.clamp(0.0, 3.0) as u8,
        created_at: now.at,
        repeat: repeat.into(),
        estimate: draft.estimate.clamp(0.0, 720.0) as u32,
        ..Task::default()
    })
}

pub fn add_drafts(db: &mut Db, drafts: &[Draft], profile: &Profile, now: &Now) -> Reply {
    let mut notices = Vec::new();
    let mut added = 0;
    let mut last = String::new();
    for draft in drafts.iter().take(parse::MAX_DRAFTS) {
        if let Some(task) = from_draft(db, draft, now) {
            if !task.time.is_empty() {
                notices.extend(board::task_notices(db, &task, profile, now));
            }
            last = task.id.clone();
            db.tasks.push(task);
            added += 1;
        }
    }
    if added == 0 {
        return refuse(now.lang.pick(
            "Write what you need to do first.",
            "Tulis dulu apa yang perlu dikerjakan.",
        ));
    }
    let message = match (now.lang, added) {
        (Lang::En, 1) => "Task added".to_string(),
        (Lang::En, n) => format!("{n} tasks added"),
        (Lang::Id, n) => format!("{n} tugas ditambahkan"),
    };
    Reply {
        count: f64::from(added),
        notices,
        ..done(message, &last)
    }
}

pub fn quick_add(db: &mut Db, text: &str, list: &str, profile: &Profile, now: &Now) -> Reply {
    let drafts = board::preview(db, text, false, list, now).drafts;
    add_drafts(db, &drafts, profile, now)
}

fn starters(role: &str, lang: Lang) -> [(&'static str, &'static str); 2] {
    let pick = |en: &'static str, id: &'static str| match lang {
        Lang::En => en,
        Lang::Id => id,
    };
    match role {
        "student" => [
            (
                "study",
                pick(
                    "Review today's lecture notes",
                    "Baca ulang catatan kuliah hari ini",
                ),
            ),
            (
                "study",
                pick("List this week's deadlines", "Catat tenggat minggu ini"),
            ),
        ],
        "founder" => [
            (
                "work",
                pick(
                    "Name the one thing that moves the product",
                    "Tentukan satu hal yang memajukan produk",
                ),
            ),
            (
                "work",
                pick(
                    "Reply to two people waiting on you",
                    "Balas dua orang yang menunggumu",
                ),
            ),
        ],
        "creator" => [
            (
                "work",
                pick(
                    "Draft one idea for ten minutes",
                    "Buat draf satu ide selama sepuluh menit",
                ),
            ),
            (
                "personal",
                pick(
                    "Save one reference that inspires you",
                    "Simpan satu referensi yang menginspirasi",
                ),
            ),
        ],
        "caregiver" => [
            (
                "family",
                pick("Plan this week's meals", "Rencanakan menu minggu ini"),
            ),
            (
                "personal",
                pick(
                    "Take ten quiet minutes for yourself",
                    "Ambil sepuluh menit tenang untukmu",
                ),
            ),
        ],
        _ => [
            (
                "work",
                pick(
                    "Pick today's most important task",
                    "Pilih tugas terpenting hari ini",
                ),
            ),
            (
                "personal",
                pick(
                    "Clear one small thing you have postponed",
                    "Bereskan satu hal kecil yang tertunda",
                ),
            ),
        ],
    }
}

/// The first launch: the person's lists, the tasks they tried in onboarding, and
/// the one thing they said was on their mind.
pub fn complete_onboarding(
    db: &mut Db,
    profile: &Profile,
    drafts: &[Draft],
    first: &str,
    now: &Now,
) -> Reply {
    db.ensure_lists(&profile.areas);
    // Finishing onboarding twice (a relaunch mid-write) adds nothing twice.
    if !db.tasks.is_empty() {
        return done(now.lang.pick("Welcome back", "Selamat datang kembali"), "");
    }
    let mut all: Vec<Draft> = Vec::new();
    if !first.trim().is_empty() {
        let mut preview = board::preview(db, first, false, "inbox", now).drafts;
        for draft in &mut preview {
            if draft.due.is_empty() {
                draft.due = dates::key(now.today());
            }
        }
        all.extend(preview);
    }
    all.extend(
        drafts
            .iter()
            .filter(|d| !all.iter().any(|a| a.title.eq_ignore_ascii_case(&d.title)))
            .cloned()
            .collect::<Vec<_>>(),
    );
    if all.len() < 3 {
        for (list, title) in starters(&profile.role, now.lang)
            .into_iter()
            .take(3 - all.len())
        {
            all.push(Draft {
                title: title.into(),
                due: dates::key(now.today()),
                list_id: list.into(),
                estimate: profile.focus.clamp(5.0, 60.0),
                repeat: "none".into(),
                ..Draft::default()
            });
        }
    }
    add_drafts(db, &all, profile, now)
}

fn next_due(repeat: &str, from: i64) -> i64 {
    match repeat {
        "daily" => from + 1,
        "weekdays" => {
            let mut day = from + 1;
            while dates::weekday(day) >= 5 {
                day += 1;
            }
            day
        }
        "weekly" => from + 7,
        _ => dates::add_month(from),
    }
}

pub fn toggle(db: &mut Db, id: &str, profile: &Profile, now: &Now) -> Reply {
    let Some(task) = db.task(id).cloned() else {
        return missing(now.lang);
    };
    let mut notices = Vec::new();
    if task.done {
        // Undo the occurrence this completion made, unless the person touched it.
        if let Some(spawned) = &task.spawned {
            db.tasks.retain(|t| {
                !(&t.id == spawned && !t.done && t.notes == task.notes && t.title == task.title)
            });
            notices.push(board::cancel(&format!("task-{spawned}")));
        }
        if let Some(t) = db.task_mut(id) {
            t.done = false;
            t.completed_at = None;
            t.spawned = None;
        }
        if let Some(t) = db.task(id) {
            notices.extend(board::task_notices(db, t, profile, now));
        }
        return Reply {
            notices,
            count: total_done(db),
            ..done(now.lang.pick("Back on your list", "Kembali ke daftar"), id)
        };
    }
    let mut spawned = None;
    if task.repeat != "none" && db.tasks.len() < MAX_TASKS {
        let from = board::due_day(&task).unwrap_or(now.today());
        let mut due = next_due(&task.repeat, from);
        // A long-missed repeat resumes from today rather than replaying the past.
        let mut steps = 0;
        while due <= now.today() && steps < 4000 {
            due = next_due(&task.repeat, due);
            steps += 1;
        }
        let mut next = task.clone();
        next.id = db.next_id("t");
        next.due = dates::key(due);
        next.done = false;
        next.completed_at = None;
        next.created_at = now.at;
        next.spawned = None;
        for s in &mut next.subtasks {
            s.done = false;
        }
        spawned = Some(next.id.clone());
        notices.extend(board::task_notices(db, &next, profile, now));
        db.tasks.push(next);
    }
    if let Some(t) = db.task_mut(id) {
        t.done = true;
        t.completed_at = Some(now.at);
        t.spawned = spawned;
    }
    if let Some(t) = db.task(id) {
        notices.extend(board::task_notices(db, t, profile, now));
    }
    let clear = board::due_now(db, now).is_empty()
        && board::due_day(&task).is_some_and(|d| d <= now.today());
    let message = if clear {
        now.lang
            .pick("Everything for today is done", "Semua tugas hari ini beres")
    } else {
        now.lang.pick("Done", "Selesai")
    };
    Reply {
        notices,
        completed: true,
        celebrate: clear,
        undoable: false,
        count: total_done(db),
        ..done(message, id)
    }
}

pub fn remove(db: &mut Db, id: &str, now: &Now) -> Reply {
    let Some(index) = db.tasks.iter().position(|t| t.id == id) else {
        return missing(now.lang);
    };
    let task = db.tasks.remove(index);
    db.trash = Some((index, task));
    Reply {
        notices: vec![
            board::cancel(&format!("task-{id}")),
            board::cancel(&format!("task-{id}-again")),
        ],
        undoable: true,
        ..done(now.lang.pick("Task deleted", "Tugas dihapus"), id)
    }
}

pub fn undo(db: &mut Db, profile: &Profile, now: &Now) -> Reply {
    let Some((index, task)) = db.trash.take() else {
        return refuse(
            now.lang
                .pick("Nothing to undo.", "Tidak ada yang bisa diurungkan."),
        );
    };
    let id = task.id.clone();
    let notices = board::task_notices(db, &task, profile, now);
    let index = index.min(db.tasks.len());
    db.tasks.insert(index, task);
    Reply {
        notices,
        ..done(now.lang.pick("Restored", "Dipulihkan"), &id)
    }
}

pub fn snooze(db: &mut Db, id: &str, preset: &str, profile: &Profile, now: &Now) -> Reply {
    let today = now.today();
    let minute = now.local.minute;
    let Some(task) = db.task_mut(id) else {
        return missing(now.lang);
    };
    match preset {
        "hour" => {
            let later = ((minute + 60) / 5) * 5;
            task.due = dates::key(today + i64::from(later / 1440));
            task.time = dates::time_key(later % 1440);
            task.reminder = true;
        }
        "tonight" => {
            task.due = dates::key(today);
            task.time = "20:00".into();
            task.reminder = true;
        }
        "tomorrow" => task.due = dates::key(today + 1),
        "nextweek" => task.due = dates::key(today + 7 - i64::from(dates::weekday(today))),
        "today" => task.due = dates::key(today),
        "nodate" => {
            task.due.clear();
            task.time.clear();
            task.repeat = "none".into();
        }
        _ => return refuse("?"),
    }
    let task = task.clone();
    let label = if task.due.is_empty() {
        now.lang.pick("No date", "Tanpa tanggal").to_string()
    } else {
        text::day_label(
            now.lang,
            dates::parse_key(&task.due).unwrap_or(today),
            today,
        )
    };
    let message = match now.lang {
        Lang::En => format!("Moved to {label}"),
        Lang::Id => format!("Dipindah ke {label}"),
    };
    Reply {
        notices: board::task_notices(db, &task, profile, now),
        ..done(message, id)
    }
}

pub fn save(db: &mut Db, edit: &TaskEdit, profile: &Profile, now: &Now) -> Reply {
    let title = parse::truncate(edit.title.trim(), parse::MAX_TITLE);
    if title.is_empty() {
        return refuse(
            now.lang
                .pick("Give the task a name.", "Beri nama tugasnya."),
        );
    }
    let due = if edit.due.is_empty() {
        String::new()
    } else {
        match dates::parse_key(&edit.due) {
            Some(day) => dates::key(day),
            None => {
                return refuse(
                    now.lang
                        .pick("That date isn't valid.", "Tanggal itu tidak valid."),
                )
            }
        }
    };
    let time = if due.is_empty() || edit.time.is_empty() {
        String::new()
    } else {
        match dates::parse_time(&edit.time) {
            Some(m) => dates::time_key(m),
            None => {
                return refuse(
                    now.lang
                        .pick("That time isn't valid.", "Waktu itu tidak valid."),
                )
            }
        }
    };
    if db.list(&edit.list_id).is_none() {
        return refuse(
            now.lang
                .pick("That list was removed.", "Daftar itu sudah dihapus."),
        );
    }
    let Some(task) = db.task_mut(&edit.id) else {
        return missing(now.lang);
    };
    task.title = title;
    task.notes = parse::truncate(edit.notes.trim_end(), MAX_NOTES);
    task.list = edit.list_id.clone();
    task.priority = edit.priority.clamp(0.0, 3.0) as u8;
    task.repeat = match edit.repeat.as_str() {
        r @ ("daily" | "weekdays" | "weekly" | "monthly") if !due.is_empty() => r.into(),
        _ => "none".into(),
    };
    task.reminder = edit.reminder && !time.is_empty();
    task.estimate = edit.estimate.clamp(0.0, 720.0) as u32;
    task.due = due;
    task.time = time;
    let task = task.clone();
    Reply {
        notices: board::task_notices(db, &task, profile, now),
        ..done(now.lang.pick("Saved", "Tersimpan"), &edit.id)
    }
}

pub fn flag(db: &mut Db, id: &str, now: &Now) -> Reply {
    let Some(task) = db.task_mut(id) else {
        return missing(now.lang);
    };
    task.flagged = !task.flagged;
    let message = if task.flagged {
        now.lang.pick("Flagged", "Ditandai")
    } else {
        now.lang.pick("Flag removed", "Tanda dihapus")
    };
    done(message, id)
}

pub fn add_sub(db: &mut Db, id: &str, title: &str, now: &Now) -> Reply {
    let title = parse::truncate(title.trim(), parse::MAX_TITLE);
    if title.is_empty() {
        return refuse(
            now.lang
                .pick("Write the step first.", "Tulis langkahnya dulu."),
        );
    }
    let sub = db.next_id("s");
    let Some(task) = db.task_mut(id) else {
        return missing(now.lang);
    };
    if task.subtasks.len() >= MAX_SUBTASKS {
        return refuse(now.lang.pick(
            "That's enough steps for one task.",
            "Langkah untuk satu tugas sudah cukup.",
        ));
    }
    task.subtasks.push(Sub {
        id: sub,
        title,
        done: false,
    });
    done(now.lang.pick("Step added", "Langkah ditambahkan"), id)
}

pub fn toggle_sub(db: &mut Db, id: &str, sub: &str, now: &Now) -> Reply {
    let Some(item) = db
        .task_mut(id)
        .and_then(|t| t.subtasks.iter_mut().find(|s| s.id == sub))
    else {
        return missing(now.lang);
    };
    item.done = !item.done;
    done("", id)
}

pub fn remove_sub(db: &mut Db, id: &str, sub: &str, now: &Now) -> Reply {
    let Some(task) = db.task_mut(id) else {
        return missing(now.lang);
    };
    task.subtasks.retain(|s| s.id != sub);
    done("", id)
}

pub fn log_focus(db: &mut Db, id: &str, minutes: f64, now: &Now) -> Reply {
    let minutes = if minutes.is_finite() {
        minutes.clamp(0.0, 600.0) as u32
    } else {
        0
    };
    if minutes == 0 {
        return done(now.lang.pick("Session ended", "Sesi berakhir"), id);
    }
    db.sessions.push(Session {
        task: id.into(),
        minutes,
        ended_at: now.at,
    });
    if db.sessions.len() > 2_000 {
        db.sessions.remove(0);
    }
    let message = match now.lang {
        Lang::En => format!("{minutes} focused minutes logged"),
        Lang::Id => format!("{minutes} menit fokus tercatat"),
    };
    done(message, id)
}

pub fn create_list(db: &mut Db, name: &str, symbol: &str, color: &str, now: &Now) -> Reply {
    let name = parse::truncate(name.trim(), 30);
    if name.is_empty() {
        return refuse(
            now.lang
                .pick("Name your list first.", "Beri nama daftarmu dulu."),
        );
    }
    let taken = db
        .lists
        .iter()
        .any(|l| text::list_name(now.lang, &l.id, &l.name).eq_ignore_ascii_case(&name));
    if taken {
        return refuse(now.lang.pick(
            "You already have a list with that name.",
            "Sudah ada daftar dengan nama itu.",
        ));
    }
    if db.lists.len() >= MAX_LISTS {
        return refuse(now.lang.pick(
            "That's the most lists Lumen keeps.",
            "Jumlah daftar sudah maksimal.",
        ));
    }
    let id = db.next_id("l");
    db.lists.push(List {
        id: id.clone(),
        name,
        symbol: if SYMBOLS.contains(&symbol) {
            symbol.into()
        } else {
            SYMBOLS[0].into()
        },
        color: if COLORS.contains(&color) {
            color.into()
        } else {
            COLORS[0].into()
        },
    });
    done(now.lang.pick("List created", "Daftar dibuat"), &id)
}

pub fn remove_list(db: &mut Db, id: &str, now: &Now) -> Reply {
    if id == "inbox" || db.list(id).is_none() {
        return refuse(now.lang.pick(
            "That list can't be removed.",
            "Daftar itu tidak bisa dihapus.",
        ));
    }
    for task in db.tasks.iter_mut().filter(|t| t.list == id) {
        task.list = "inbox".into();
    }
    db.lists.retain(|l| l.id != id);
    done(
        now.lang.pick(
            "List removed. Its tasks moved to Inbox.",
            "Daftar dihapus. Tugasnya pindah ke Kotak Masuk.",
        ),
        id,
    )
}

/// Move what is over the day's capacity to tomorrow: the least urgent first.
pub fn balance(db: &mut Db, profile: &Profile, now: &Now) -> Reply {
    let capacity = profile.capacity.clamp(1.0, 20.0) as usize;
    let moving: Vec<String> = board::due_now(db, now)
        .iter()
        .skip(capacity)
        .map(|t| t.id.clone())
        .collect();
    if moving.is_empty() {
        return done(
            now.lang.pick("Today already fits", "Hari ini sudah pas"),
            "",
        );
    }
    let tomorrow = dates::key(now.today() + 1);
    let mut notices = Vec::new();
    for id in &moving {
        if let Some(task) = db.task_mut(id) {
            task.due = tomorrow.clone();
            let task = task.clone();
            notices.extend(board::task_notices(db, &task, profile, now));
        }
    }
    let message = match now.lang {
        Lang::En => format!(
            "Moved {} to tomorrow",
            text::count_tasks(now.lang, moving.len())
        ),
        Lang::Id => format!(
            "{} dipindah ke besok",
            text::count_tasks(now.lang, moving.len())
        ),
    };
    Reply {
        notices,
        count: moving.len() as f64,
        ..done(message, "")
    }
}

pub fn catch_up(db: &mut Db, profile: &Profile, now: &Now) -> Reply {
    let today = dates::key(now.today());
    let ids: Vec<String> = db
        .tasks
        .iter()
        .filter(|t| !t.done && board::due_day(t).is_some_and(|d| d < now.today()))
        .map(|t| t.id.clone())
        .collect();
    let mut notices = Vec::new();
    for id in &ids {
        if let Some(task) = db.task_mut(id) {
            task.due = today.clone();
            let task = task.clone();
            notices.extend(board::task_notices(db, &task, profile, now));
        }
    }
    let message = match now.lang {
        Lang::En => format!("{} moved to today", text::count_tasks(now.lang, ids.len())),
        Lang::Id => format!(
            "{} dipindah ke hari ini",
            text::count_tasks(now.lang, ids.len())
        ),
    };
    Reply {
        notices,
        count: ids.len() as f64,
        ..done(message, "")
    }
}

pub fn erase(db: &mut Db, profile: &Profile, now: &Now) -> Reply {
    let mut notices: Vec<Notice> = db
        .tasks
        .iter()
        .flat_map(|t| {
            [
                board::cancel(&format!("task-{}", t.id)),
                board::cancel(&format!("task-{}-again", t.id)),
            ]
        })
        .take(60)
        .collect();
    notices.extend([
        board::cancel("nudge-morning"),
        board::cancel("nudge-evening"),
        board::cancel("nudge-weekly"),
    ]);
    let mut fresh = Db::fresh(&profile.areas);
    fresh.seq = db.seq;
    *db = fresh;
    // The built-ins are all that survive.
    debug_assert!(db
        .lists
        .iter()
        .all(|l| BUILTIN.iter().any(|(id, ..)| *id == l.id)));
    Reply {
        notices,
        ..done(
            now.lang.pick("Everything was erased", "Semua data dihapus"),
            "",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> Profile {
        Profile {
            areas: vec!["work".into(), "home".into()],
            capacity: 3.0,
            focus: 25.0,
            wake: "07:00".into(),
            wind: "22:00".into(),
            reminders: "persistent".into(),
            peak: "morning".into(),
            weekly: "friday".into(),
            role: "professional".into(),
            ..Profile::default()
        }
    }

    /// Friday 27 November 2026, 10:00 in Jakarta.
    fn now() -> Now {
        let day = dates::parse_key("2026-11-27").unwrap();
        Now::new(&Context {
            now: dates::instant(day, 600, 420.0),
            offset: 420.0,
            lang: "en".into(),
        })
    }

    fn db() -> Db {
        Db::fresh(&profile().areas)
    }

    #[test]
    fn onboarding_seeds_lists_and_first_tasks_once() {
        let mut db = Db::default();
        let reply = complete_onboarding(&mut db, &profile(), &[], "Send the deck at 3pm", &now());
        assert!(reply.ok);
        assert_eq!(db.tasks.len(), 3);
        assert_eq!(db.tasks[0].title, "Send the deck");
        assert_eq!(db.tasks[0].time, "15:00");
        assert!(db.list("inbox").is_some() && db.list("work").is_some());
        assert!(
            reply
                .notices
                .iter()
                .any(|n| !n.cancel && n.tag.ends_with("again")),
            "persistent follows up"
        );
        complete_onboarding(&mut db, &profile(), &[], "Send the deck at 3pm", &now());
        assert_eq!(db.tasks.len(), 3, "a second finish adds nothing");
    }

    #[test]
    fn completing_a_repeat_makes_the_next_and_undo_removes_it() {
        let mut db = db();
        quick_add(
            &mut db,
            "Water plants every day",
            "home",
            &profile(),
            &now(),
        );
        let id = db.tasks[0].id.clone();
        let reply = toggle(&mut db, &id, &profile(), &now());
        assert!(reply.completed && reply.celebrate);
        assert_eq!(db.tasks.len(), 2);
        assert_eq!(db.tasks[1].due, "2026-11-28");
        toggle(&mut db, &id, &profile(), &now());
        assert_eq!(db.tasks.len(), 1);
        assert!(!db.tasks[0].done);
    }

    #[test]
    fn delete_restores_in_place() {
        let mut db = db();
        quick_add(&mut db, "one; two; three", "inbox", &profile(), &now());
        let id = db.tasks[1].id.clone();
        assert!(remove(&mut db, &id, &now()).undoable);
        assert_eq!(db.tasks.len(), 2);
        assert!(undo(&mut db, &profile(), &now()).ok);
        assert_eq!(db.tasks[1].id, id);
        assert!(!undo(&mut db, &profile(), &now()).ok);
    }

    #[test]
    fn balancing_moves_only_the_overflow() {
        let mut db = db();
        quick_add(
            &mut db,
            "a today !high; b today; c today; d today; e today",
            "inbox",
            &profile(),
            &now(),
        );
        let reply = balance(&mut db, &profile(), &now());
        assert_eq!(reply.count, 2.0);
        assert_eq!(board::due_now(&db, &now()).len(), 3);
        assert!(
            board::due_now(&db, &now()).iter().any(|t| t.priority == 3),
            "the urgent one stays"
        );
    }

    #[test]
    fn refusals_leave_the_document_alone() {
        let mut db = db();
        assert!(!quick_add(&mut db, "   ", "inbox", &profile(), &now()).ok);
        assert!(!create_list(&mut db, " ", "star.fill", "blue", &now()).ok);
        assert!(!create_list(&mut db, "Work", "star.fill", "blue", &now()).ok);
        assert!(!remove_list(&mut db, "inbox", &now()).ok);
        quick_add(&mut db, "x", "inbox", &profile(), &now());
        let id = db.tasks[0].id.clone();
        let edit = TaskEdit {
            id: id.clone(),
            title: "".into(),
            list_id: "inbox".into(),
            ..TaskEdit::default()
        };
        assert!(!save(&mut db, &edit, &profile(), &now()).ok);
        let edit = TaskEdit {
            id,
            title: "ok".into(),
            due: "2026-02-31".into(),
            list_id: "inbox".into(),
            ..TaskEdit::default()
        };
        assert!(!save(&mut db, &edit, &profile(), &now()).ok);
        assert_eq!(db.tasks[0].title, "X");
    }

    #[test]
    fn removing_a_list_keeps_its_tasks() {
        let mut db = db();
        quick_add(&mut db, "Report #work", "inbox", &profile(), &now());
        assert!(remove_list(&mut db, "work", &now()).ok);
        assert_eq!(db.tasks[0].list, "inbox");
    }

    #[test]
    fn the_board_greets_sorts_and_suggests() {
        let mut db = db();
        let mut p = profile();
        p.name = "Rangga".into();
        quick_add(&mut db, "low today; high today !high", "inbox", &p, &now());
        let board = board::board(&db, &p, &now());
        assert_eq!(board.greeting, "Good morning, Rangga");
        assert_eq!(board.top[0].title, "High");
        assert_eq!(board.headline, "2 tasks for today");
        assert!(board.suggestions.iter().any(|s| s.kind == "focus"));
        assert!(board.suggestions.len() <= 2);
        toggle(&mut db, &board.top[0].id.clone(), &p, &now());
        let board = board::board(&db, &p, &now());
        assert_eq!(board.stats.done_today, 1.0);
        assert_eq!(board.stats.streak, 1.0);
        assert_eq!(board.stats.bars.len(), 7);
    }

    #[test]
    fn nudges_carry_upcoming_reminders_and_respect_none() {
        let mut db = db();
        let p = profile();
        quick_add(&mut db, "call mom tomorrow 5pm", "inbox", &p, &now());
        let list = board::nudges(&db, &p, &now());
        assert!(list
            .iter()
            .any(|n| n.tag == "nudge-morning" && n.body.starts_with("1 task")));
        assert!(list
            .iter()
            .any(|n| n.tag.starts_with("task-") && !n.cancel && n.at > now().at));
        let quiet = Profile {
            reminders: "none".into(),
            ..p
        };
        assert!(board::nudges(&db, &quiet, &now()).iter().all(|n| n.cancel));
    }

    #[test]
    fn the_model_summary_is_refused_when_it_invents_numbers() {
        let p = profile();
        let template = board::plan(&p, &now()).summary;
        let invented = "You will finish 40 tasks every single day with this wonderful plan.";
        assert_eq!(board::checked_summary(invented, &p, &now()), template);
        let good = "Each morning starts calmly at 7:00 AM with at most 3 tasks, then a quiet evening review.";
        assert_eq!(board::checked_summary(good, &p, &now()), good);
    }
}
