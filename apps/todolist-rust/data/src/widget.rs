//! The widget reads a small snapshot the app keeps in the shared store: a
//! widget's process has no way into the app's files.

use crate::board;
use crate::dates;
use crate::model::Db;
use crate::shapes::*;
use crate::text::{self, Lang};
use serde::{Deserialize, Serialize};

pub const KEY: &str = "lumen.widget";
const MAX_ROWS: usize = 24;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Snapshot {
    pub lang: String,
    pub offset: f64,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Row {
    pub title: String,
    pub due: String,
    pub time: String,
    pub done_on: String,
}

/// What the widget needs for today and tomorrow.
pub fn snapshot(db: &Db, now: &board::Now) -> String {
    let today = now.today();
    let mut open: Vec<_> = db
        .tasks
        .iter()
        .filter(|t| !t.done && board::due_day(t).is_some_and(|d| d <= today + 1))
        .collect();
    open.sort_by(|a, b| board::order(a, b));
    let finished = db
        .tasks
        .iter()
        .filter(|t| t.done && board::completed_day(t, now.offset) == Some(today));
    let rows = open
        .into_iter()
        .map(|t| Row {
            title: t.title.clone(),
            due: t.due.clone(),
            time: t.time.clone(),
            done_on: String::new(),
        })
        .chain(finished.map(|t| Row {
            title: t.title.clone(),
            due: t.due.clone(),
            time: t.time.clone(),
            done_on: dates::key(today),
        }))
        .take(MAX_ROWS)
        .collect();
    let snapshot = Snapshot {
        lang: now.lang.pick("en", "id").into(),
        offset: now.offset,
        rows,
    };
    serde_json::to_string(&snapshot).unwrap_or_default()
}

fn props(snapshot: &Snapshot, day: i64) -> WidgetProps {
    let lang = Lang::from_tag(&snapshot.lang);
    let key = dates::key(day);
    let done = snapshot.rows.iter().filter(|r| r.done_on == key).count();
    let open: Vec<&Row> = snapshot
        .rows
        .iter()
        .filter(|r| r.done_on.is_empty() && dates::parse_key(&r.due).is_some_and(|d| d <= day))
        .collect();
    let total = done + open.len();
    let rows = open
        .iter()
        .take(3)
        .map(|r| WidgetRow {
            title: r.title.clone(),
            time: dates::parse_time(&r.time)
                .map(|m| text::clock(lang, m))
                .unwrap_or_default(),
            done: false,
        })
        .collect::<Vec<_>>();
    let headline = match (open.len(), lang) {
        (0, _) if done > 0 => lang.pick("All clear", "Beres semua").to_string(),
        (0, _) => lang.pick("A clear day", "Hari lapang").to_string(),
        (n, Lang::En) => format!("{n} to do"),
        (n, Lang::Id) => format!("{n} tugas"),
    };
    WidgetProps {
        day: text::weekday_name(lang, day).to_string(),
        headline,
        done: done as f64,
        total: total as f64,
        ratio: if total == 0 {
            0.0
        } else {
            done as f64 / total as f64
        },
        more: open.len().saturating_sub(rows.len()) as f64,
        rows,
        empty: lang
            .pick("Nothing due. Enjoy it.", "Tidak ada tenggat. Nikmati.")
            .into(),
    }
}

/// Now, and again just after midnight so the day turns over with the app closed.
pub fn timeline(stored: Option<&str>, at: f64) -> Vec<WidgetEntry> {
    let snapshot: Snapshot = stored
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let today = dates::local(at, snapshot.offset).day;
    let midnight = dates::instant(today + 1, 0, snapshot.offset) + 1_000.0;
    vec![
        WidgetEntry {
            date: at,
            props: props(&snapshot, today),
        },
        WidgetEntry {
            date: midnight.max(at + 1.0),
            props: props(&snapshot, today + 1),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mutate;

    #[test]
    fn the_timeline_turns_over_at_midnight() {
        let day = dates::parse_key("2026-11-27").unwrap();
        let now = board::Now::new(&Context {
            now: dates::instant(day, 600, 420.0),
            offset: 420.0,
            lang: "en".into(),
        });
        let mut db = Db::fresh(&[]);
        let profile = Profile {
            capacity: 5.0,
            ..Profile::default()
        };
        mutate::quick_add(&mut db, "a today; b tomorrow", "inbox", &profile, &now);
        let stored = snapshot(&db, &now);
        let entries = timeline(Some(&stored), now.at);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].props.headline, "1 to do");
        assert_eq!(entries[1].props.headline, "2 to do");
        assert!(entries[1].date > entries[0].date);
        assert_eq!(
            timeline(Some("not json"), now.at)[0].props.headline,
            "A clear day"
        );
    }
}
