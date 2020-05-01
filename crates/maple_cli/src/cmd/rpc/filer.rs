use std::{fs, io};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{write_response, Message};
use icon::prepend_filer_icon;

fn into_string(entry: std::fs::DirEntry, enable_icon: bool) -> String {
    let path = entry.path();
    let path_str = if path.is_dir() {
        format!(
            "{}/",
            path.file_name().and_then(std::ffi::OsStr::to_str).unwrap()
        )
    } else {
        path.file_name()
            .and_then(std::ffi::OsStr::to_str)
            .map(Into::into)
            .unwrap()
    };

    if enable_icon {
        prepend_filer_icon(&entry.path(), &path_str)
    } else {
        path_str
    }
}

fn read_dir_entries(dir: &str, enable_icon: bool) -> Result<Vec<String>> {
    let mut entries = fs::read_dir(dir)?
        .map(|res| res.map(|x| into_string(x, enable_icon)))
        .collect::<Result<Vec<_>, io::Error>>()?;

    entries.sort();

    Ok(entries)
}

#[derive(Serialize, Deserialize)]
struct FilerParams {
    cwd: String,
    enable_icon: bool,
}

impl From<serde_json::Map<String, serde_json::Value>> for FilerParams {
    fn from(serde_map: serde_json::Map<String, serde_json::Value>) -> Self {
        Self {
            cwd: String::from(
                serde_map
                    .get("cwd")
                    .and_then(|x| x.as_str())
                    .unwrap_or("Missing cwd when deserializing into FilerParams"),
            ),
            enable_icon: serde_map
                .get("enable_icon")
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
        }
    }
}

pub(super) fn handle_message(msg: Message) {
    let FilerParams { cwd, enable_icon } = msg.params.into();

    let result = match read_dir_entries(&cwd, enable_icon) {
        Ok(entries) => {
            let result = json!({
            "entries": entries,
            "dir": cwd,
            "total": entries.len(),
            });
            json!({ "result": result, "id": msg.id })
        }
        Err(err) => {
            let error = json!({"message": format!("{}", err), "dir": cwd});
            json!({ "error": error, "id": msg.id })
        }
    };

    write_response(result);
}

#[test]
fn test_dir() {
    let entries = read_dir_entries(
        &std::env::current_dir()
            .unwrap()
            .into_os_string()
            .into_string()
            .unwrap(),
        false,
    )
    .unwrap();
    println!("entry: {:?}", entries);
}
