use super::filer::{read_dir_entries, FilerParams};
use super::{write_response, Message};
use anyhow::Result;
use log::debug;
use serde_json::json;

// TODO: generic on_init handler
pub(super) fn handle_message(msg: Message) -> Result<()> {
    let provider_id = msg
        .params
        .get("provider_id")
        .and_then(|x| x.as_str())
        .unwrap_or("Unknown provider id");

    let enable_icon = msg
        .params
        .get("enable_icon")
        .and_then(|x| x.as_bool())
        .unwrap_or(false);

    let cwd = String::from(
        msg.params
            .get("cwd")
            .and_then(|x| x.as_str())
            .unwrap_or("Missing cwd when deserializing into FilerParams"),
    );

    let size = msg
        .params
        .get("preview_size")
        .and_then(|x| x.as_u64().map(|x| x as usize))
        .unwrap_or(5);

    let FilerParams { cwd, enable_icon } = msg.params.into();
    debug!(
        "Recv filer params: cwd:{}, enable_icon:{}",
        cwd, enable_icon
    );

    let result = match read_dir_entries(&cwd, enable_icon, None) {
        Ok(entries) => {
            let result = json!({
            "entries": entries,
            "dir": cwd,
            "total": entries.len(),
            });
            json!({ "id": msg.id, "provider_id": "filer", "event": "on_init", "result": result })
        }
        Err(err) => {
            let error = json!({"message": format!("{}", err), "dir": cwd});
            json!({ "id": msg.id, "provider_id": "filer", "error": error })
        }
    };

    write_response(result);

    Ok(())
}
