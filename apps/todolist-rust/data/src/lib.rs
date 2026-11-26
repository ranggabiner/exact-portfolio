//! The app's data: what app.contract's sources answer. `query` answers now, with no I/O, so the build bakes it
//! into the plan for the first frame; `answer` and `parse` add storage and requests
//! (docs/reference.md, "Rust data sources").

use exact_runner::{DataError, DataSource, Value};

#[allow(dead_code, clippy::all)]
mod shapes {
    include!(concat!(env!("OUT_DIR"), "/shapes.rs"));
}
use shapes::{ContractValue, Greeting};

/// The app's data source.
#[derive(Default)]
pub struct TodoList;

impl DataSource for TodoList {
    fn query(&mut self, source: &str, args: &[Value]) -> Result<Value, DataError> {
        match source {
            "greeting" => {
                let name = args
                    .first()
                    .and_then(Value::as_str)
                    .ok_or_else(|| DataError::BadArguments(source.into()))?;
                Ok(Greeting {
                    text: format!("Hello from {name}."),
                }
                .to_value())
            }
            _ => Err(DataError::UnknownSource(source.into())),
        }
    }
}
