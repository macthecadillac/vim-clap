use super::filer::{read_dir_entries, FilerParams};
use super::{write_response, Message};
use anyhow::Result;
use log::debug;
use serde_json::json;

// TODO: generic on_init handler
pub(super) fn handle_message(msg: Message) -> Result<()> {
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
