//! Lumen's data module: what app.contract's sources answer.
//!
//! The document lives in `app:/data/lumen.json`. A source reads it once, keeps it
//! in memory, and writes it after every change. Pure sources (the capture
//! preview, the onboarding plan) answer at once, with no storage.

mod board;
mod dates;
mod model;
mod mutate;
mod parse;
mod text;
mod widget;

use exact_runner::{Answer, DataError, DataSource, Outcome, Request, Store, Value};
use model::Db;
use serde_json::{json, Value as Json};
use std::collections::BTreeMap;

#[allow(dead_code, clippy::all)]
mod shapes {
    include!(concat!(env!("OUT_DIR"), "/shapes.rs"));
}
use shapes::*;

const PATH: &str = "app:/data/lumen.json";
const GRANTS: &str = "fs.read app:/data\nfs.write app:/data\nsecret.keep lumen.widget\ndevice.microphone purpose.microphone\ndevice.speech-recognition purpose.speech\ndevice.notifications purpose.notifications";

struct Write {
    reply: Reply,
    before: Box<Db>,
    lang: String,
}

/// The app's data source.
#[derive(Default)]
pub struct TodoList {
    db: Option<Db>,
    writes: BTreeMap<String, Write>,
}

fn arg<T: ContractValue>(source: &str, args: &[Value], i: usize) -> Result<T, DataError> {
    args.get(i)
        .and_then(T::from_value)
        .ok_or_else(|| DataError::BadArguments(source.into()))
}

fn storage(op: &str, args: Json) -> Request {
    Request::storage(
        serde_json::to_vec(&json!({"version":1,"op":op,"args":args})).unwrap_or_default(),
    )
}

fn response(outcome: Outcome) -> Result<Json, String> {
    match outcome {
        Outcome::Storage(bytes) => {
            let value: Json = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            match value.get("error").and_then(Json::as_str) {
                Some(error) => Err(error.into()),
                None => Ok(value),
            }
        }
        Outcome::Failed { message, .. } => Err(message),
        _ => Err("expected a storage result".into()),
    }
}

fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let (mut bits, mut have) = (0u32, 0u32);
    for c in text.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b'\n' | b'\r' => continue,
            _ => return None,
        };
        bits = (bits << 6) | u32::from(v);
        have += 6;
        if have >= 8 {
            have -= 8;
            out.push((bits >> have) as u8);
        }
    }
    Some(out)
}

/// Whether a storage failure only says the file is not there yet. Each host
/// words it its own way.
fn is_missing(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    ["enoent", "no such file", "notfounderror", "os error 2"]
        .iter()
        .any(|words| message.contains(words))
}

/// The stored document, `None` for a first launch. A document that cannot be read
/// is an error, never an empty list that the next write would save over it.
fn load(outcome: Outcome) -> Result<Option<Db>, String> {
    let value = match response(outcome) {
        Ok(value) => value,
        Err(message) if is_missing(&message) => return Ok(None),
        Err(message) => return Err(message),
    };
    let bytes = value
        .get("base64")
        .and_then(Json::as_str)
        .and_then(decode_base64)
        .or_else(|| value.as_str().map(|s| s.as_bytes().to_vec()))
        .ok_or("unreadable storage reply")?;
    let db: Db = serde_json::from_slice(&bytes).map_err(|e| format!("damaged document: {e}"))?;
    if db.version > model::VERSION {
        return Err("this list was saved by a newer version of Lumen".into());
    }
    Ok(Some(db))
}

fn is_pure(source: &str) -> bool {
    matches!(
        source,
        "parseCapture" | "parseAi" | "planFor" | "checkedSummary"
    )
}

fn is_write(source: &str) -> bool {
    matches!(
        source,
        "completeOnboarding"
            | "addDrafts"
            | "quickAdd"
            | "toggleTask"
            | "deleteTask"
            | "undoDelete"
            | "snoozeTask"
            | "saveTask"
            | "flagTask"
            | "addSubtask"
            | "toggleSubtask"
            | "removeSubtask"
            | "logFocus"
            | "createList"
            | "removeList"
            | "balanceDay"
            | "catchUp"
            | "eraseAll"
    )
}

impl TodoList {
    /// A pure source: from the view's arguments, and the lists once they are loaded.
    fn pure(&self, source: &str, args: &[Value]) -> Result<Value, DataError> {
        match source {
            "parseCapture" => {
                let text: String = arg(source, args, 0)?;
                let dump: bool = arg(source, args, 1)?;
                let list: String = arg(source, args, 2)?;
                let profile: Profile = arg(source, args, 3)?;
                let context: Context = arg(source, args, 4)?;
                let now = board::Now::new(&context);
                Ok(board::preview(&self.lists_for(&profile), &text, dump, &list, &now).to_value())
            }
            "parseAi" => {
                let tasks: Vec<AiTask> = arg(source, args, 0)?;
                let list: String = arg(source, args, 1)?;
                let profile: Profile = arg(source, args, 2)?;
                let context: Context = arg(source, args, 3)?;
                let now = board::Now::new(&context);
                Ok(board::ai_preview(&self.lists_for(&profile), &tasks, &list, &now).to_value())
            }
            "planFor" => {
                let profile: Profile = arg(source, args, 0)?;
                let context: Context = arg(source, args, 1)?;
                Ok(board::plan(&profile, &board::Now::new(&context)).to_value())
            }
            "checkedSummary" => {
                let model: String = arg(source, args, 0)?;
                let profile: Profile = arg(source, args, 1)?;
                let context: Context = arg(source, args, 2)?;
                Ok(Value::str(&board::checked_summary(
                    &model,
                    &profile,
                    &board::Now::new(&context),
                )))
            }
            _ => Err(DataError::UnknownSource(source.into())),
        }
    }

    fn lists_for(&self, profile: &Profile) -> Db {
        let mut db = self.db.clone().unwrap_or_else(|| Db::fresh(&profile.areas));
        db.ensure_lists(&profile.areas);
        db
    }

    fn read(&self, source: &str, args: &[Value]) -> Result<Value, DataError> {
        let db = self
            .db
            .as_ref()
            .ok_or_else(|| DataError::Unavailable("the list is not loaded".into()))?;
        match source {
            "board" => {
                let profile: Profile = arg(source, args, 0)?;
                let context: Context = arg(source, args, 1)?;
                Ok(board::board(db, &profile, &board::Now::new(&context)).to_value())
            }
            "search" => {
                let query: String = arg(source, args, 0)?;
                let filter: String = arg(source, args, 1)?;
                let context: Context = arg(source, args, 2)?;
                Ok(board::search(db, &query, &filter, &board::Now::new(&context)).to_value())
            }
            "listPage" => {
                let id: String = arg(source, args, 0)?;
                let context: Context = arg(source, args, 1)?;
                Ok(board::list_page(db, &id, &board::Now::new(&context)).to_value())
            }
            "taskDetail" => {
                let id: String = arg(source, args, 0)?;
                let context: Context = arg(source, args, 1)?;
                Ok(board::detail(db, &id, &board::Now::new(&context)).to_value())
            }
            "nudges" => {
                let profile: Profile = arg(source, args, 0)?;
                let context: Context = arg(source, args, 1)?;
                Ok(board::nudges(db, &profile, &board::Now::new(&context)).to_value())
            }
            _ => Err(DataError::UnknownSource(source.into())),
        }
    }

    /// Apply a change. The last argument is always the instant and language.
    fn apply(&mut self, source: &str, args: &[Value]) -> Result<(Reply, Context), DataError> {
        let db = self
            .db
            .as_mut()
            .ok_or_else(|| DataError::Unavailable("the list is not loaded".into()))?;
        let context: Context = arg(source, args, args.len().saturating_sub(1))?;
        let now = board::Now::new(&context);
        let profile = |i: usize| arg::<Profile>(source, args, i);
        let text = |i: usize| arg::<String>(source, args, i);
        let reply = match source {
            "completeOnboarding" => {
                let p = profile(0)?;
                let drafts: Vec<Draft> = arg(source, args, 1)?;
                mutate::complete_onboarding(db, &p, &drafts, &text(2)?, &now)
            }
            "addDrafts" => {
                let drafts: Vec<Draft> = arg(source, args, 0)?;
                mutate::add_drafts(db, &drafts, &profile(1)?, &now)
            }
            "quickAdd" => mutate::quick_add(db, &text(0)?, &text(1)?, &profile(2)?, &now),
            "toggleTask" => mutate::toggle(db, &text(0)?, &profile(1)?, &now),
            "deleteTask" => mutate::remove(db, &text(0)?, &now),
            "undoDelete" => mutate::undo(db, &profile(0)?, &now),
            "snoozeTask" => mutate::snooze(db, &text(0)?, &text(1)?, &profile(2)?, &now),
            "saveTask" => {
                let edit: TaskEdit = arg(source, args, 0)?;
                mutate::save(db, &edit, &profile(1)?, &now)
            }
            "flagTask" => mutate::flag(db, &text(0)?, &now),
            "addSubtask" => mutate::add_sub(db, &text(0)?, &text(1)?, &now),
            "toggleSubtask" => mutate::toggle_sub(db, &text(0)?, &text(1)?, &now),
            "removeSubtask" => mutate::remove_sub(db, &text(0)?, &text(1)?, &now),
            "logFocus" => mutate::log_focus(db, &text(0)?, arg(source, args, 1)?, &now),
            "createList" => mutate::create_list(db, &text(0)?, &text(1)?, &text(2)?, &now),
            "removeList" => mutate::remove_list(db, &text(0)?, &now),
            "balanceDay" => mutate::balance(db, &profile(0)?, &now),
            "catchUp" => mutate::catch_up(db, &profile(0)?, &now),
            "eraseAll" => mutate::erase(db, &profile(0)?, &now),
            _ => return Err(DataError::UnknownSource(source.into())),
        };
        Ok((reply, context))
    }

    /// Answer from the loaded document: a read now, a change after it is written.
    fn run(
        &mut self,
        store: &mut Store,
        source: &str,
        args: &[Value],
    ) -> Result<Answer, DataError> {
        if !is_write(source) {
            return Ok(Answer::Now(self.read(source, args)?));
        }
        let before = Box::new(self.db.clone().unwrap_or_default());
        let (reply, context) = self.apply(source, args)?;
        if !reply.ok {
            self.db = Some(*before);
            return Ok(Answer::Now(reply.to_value()));
        }
        let Some(db) = self.db.as_ref() else {
            return Err(DataError::Unavailable("the list is not loaded".into()));
        };
        let document =
            serde_json::to_string(db).map_err(|e| DataError::Unavailable(e.to_string()))?;
        // The widget's copy; a platform without a shared store just has none.
        let _ = store.set(
            widget::KEY,
            &widget::snapshot(db, &board::Now::new(&context)),
        );
        self.writes.insert(
            key(source, args),
            Write {
                reply,
                before,
                lang: context.lang,
            },
        );
        Ok(Answer::Later(storage(
            "fs.atomicWriteFile",
            json!({"path":PATH,"text":document}),
        )))
    }
}

fn key(source: &str, args: &[Value]) -> String {
    format!("{source}:{args:?}")
}

impl DataSource for TodoList {
    fn grants(&self) -> &str {
        GRANTS
    }

    fn query(&mut self, source: &str, args: &[Value]) -> Result<Value, DataError> {
        match source {
            "widgetTimeline" => {
                let at: f64 = arg(source, args, 0)?;
                Ok(widget::timeline(None, at).to_value())
            }
            s if is_pure(s) => self.pure(s, args),
            // Storage is not there at build time: every host asks again at launch.
            "board" | "search" | "listPage" | "taskDetail" | "nudges" => {
                if self.db.is_some() {
                    self.read(source, args)
                } else {
                    Err(DataError::Unavailable("storage is unavailable here".into()))
                }
            }
            _ => Err(DataError::UnknownSource(source.into())),
        }
    }

    fn answer(
        &mut self,
        store: &mut Store,
        source: &str,
        args: &[Value],
    ) -> Result<Answer, DataError> {
        if source == "widgetTimeline" {
            let at: f64 = arg(source, args, 0)?;
            return Ok(Answer::Now(
                widget::timeline(store.get(widget::KEY), at).to_value(),
            ));
        }
        if is_pure(source) {
            return Ok(Answer::Now(self.pure(source, args)?));
        }
        // Erasing is the way out of a damaged document, so it reads nothing first.
        if source == "eraseAll" && self.db.is_none() {
            self.db = Some(Db::default());
        }
        if self.db.is_some() {
            return self.run(store, source, args);
        }
        store.observe_external_read();
        Ok(Answer::Later(storage("fs.readFile", json!({"path":PATH}))))
    }

    fn parse(
        &mut self,
        store: &mut Store,
        source: &str,
        args: &[Value],
        outcome: Outcome,
    ) -> Result<Answer, DataError> {
        if let Some(write) = self.writes.remove(&key(source, args)) {
            return Ok(Answer::Now(match response(outcome) {
                Ok(_) => write.reply.to_value(),
                Err(_) => {
                    self.db = Some(*write.before);
                    let lang = text::Lang::from_tag(&write.lang);
                    mutate::refuse(lang.pick(
                        "Couldn't save that. Please try again.",
                        "Gagal menyimpan. Coba lagi.",
                    ))
                    .to_value()
                }
            }));
        }
        // The answer to the first read; another may have loaded it meanwhile.
        if self.db.is_none() {
            let loaded = load(outcome).map_err(DataError::Unavailable)?;
            self.db = Some(loaded.unwrap_or_else(|| Db::fresh(&[])));
        }
        self.run(store, source, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> Context {
        Context {
            now: 1_795_748_400_000.0,
            offset: 420.0,
            lang: "en".into(),
        }
    }

    fn missing() -> Outcome {
        Outcome::Storage(br#"{"error":"fs.readFile: ENOENT"}"#.to_vec())
    }

    #[test]
    fn a_first_launch_loads_an_empty_document() {
        assert_eq!(load(missing()), Ok(None));
        assert_eq!(
            load(Outcome::Storage(
                br#"{"error":"filesystem: no such file or directory: app:/data/lumen.json"}"#
                    .to_vec()
            )),
            Ok(None)
        );
        assert!(load(Outcome::Storage(br#"{"error":"EACCES"}"#.to_vec())).is_err());
        assert!(load(Outcome::Storage(br#"{"base64":"bm90IGpzb24="}"#.to_vec())).is_err());
    }

    #[test]
    fn base64_decodes() {
        assert_eq!(decode_base64("aGVsbG8="), Some(b"hello".to_vec()));
        assert_eq!(decode_base64("!!"), None);
    }

    #[test]
    fn a_pure_source_answers_without_storage() {
        let mut data = TodoList::default();
        let args = [
            Value::str("buy milk tomorrow"),
            Value::Bool(false),
            Value::str("inbox"),
            Profile {
                areas: vec!["home".into()],
                ..Profile::default()
            }
            .to_value(),
            context().to_value(),
        ];
        let preview = Preview::from_value(&data.query("parseCapture", &args).unwrap()).unwrap();
        assert_eq!(preview.drafts[0].list_id, "home");
        assert!(data.query("board", &[]).is_err());
    }
}
