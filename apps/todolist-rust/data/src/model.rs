//! What Lumen keeps: one JSON document under `app:/data/`.

use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Db {
    pub version: u32,
    pub seq: u64,
    pub lists: Vec<List>,
    pub tasks: Vec<Task>,
    pub sessions: Vec<Session>,
    /// The last deleted task and where it sat, for Undo.
    pub trash: Option<(usize, Task)>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct List {
    pub id: String,
    /// Empty for a built-in list, whose name follows the app's language.
    pub name: String,
    pub symbol: String,
    pub color: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub notes: String,
    pub list: String,
    /// `YYYY-MM-DD`, empty for no date.
    pub due: String,
    /// `HH:MM`, empty for no time.
    pub time: String,
    /// 0 none, 1 low, 2 medium, 3 high.
    pub priority: u8,
    pub flagged: bool,
    pub done: bool,
    pub completed_at: Option<f64>,
    pub created_at: f64,
    /// `none`, `daily`, `weekdays`, `weekly` or `monthly`.
    pub repeat: String,
    pub subtasks: Vec<Sub>,
    pub reminder: bool,
    /// Minutes; 0 when unknown.
    pub estimate: u32,
    /// The next occurrence a repeating task made when it was completed.
    pub spawned: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Sub {
    pub id: String,
    pub title: String,
    pub done: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Session {
    pub task: String,
    pub minutes: u32,
    pub ended_at: f64,
}

/// Built-in lists: id, SF Symbol, colour role.
pub const BUILTIN: [(&str, &str, &str); 8] = [
    ("inbox", "tray.fill", "blue"),
    ("work", "briefcase.fill", "indigo"),
    ("personal", "person.fill", "pink"),
    ("study", "graduationcap.fill", "orange"),
    ("health", "heart.fill", "red"),
    ("home", "house.fill", "teal"),
    ("money", "creditcard.fill", "green"),
    ("family", "figure.2.and.child.holdinghands", "purple"),
];

impl Db {
    pub fn next_id(&mut self, prefix: &str) -> String {
        self.seq += 1;
        format!("{prefix}{}", self.seq)
    }

    /// A new document holding Inbox and the areas of life the person chose.
    pub fn fresh(areas: &[String]) -> Db {
        let mut db = Db {
            version: VERSION,
            ..Db::default()
        };
        db.ensure_lists(areas);
        db
    }

    pub fn ensure_lists(&mut self, areas: &[String]) {
        for (id, symbol, color) in BUILTIN {
            let wanted = id == "inbox" || areas.iter().any(|a| a == id);
            if wanted && !self.lists.iter().any(|l| l.id == id) {
                self.lists.push(List {
                    id: id.into(),
                    name: String::new(),
                    symbol: symbol.into(),
                    color: color.into(),
                });
            }
        }
    }

    pub fn list(&self, id: &str) -> Option<&List> {
        self.lists.iter().find(|l| l.id == id)
    }

    pub fn task(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn task_mut(&mut self, id: &str) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }
}
