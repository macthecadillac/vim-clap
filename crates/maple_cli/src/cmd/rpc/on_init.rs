use super::filer::{read_dir_entries, FilerParams};
use super::{write_response, Message};
use anyhow::Result;
use log::debug;
use serde_json::json;
use std::io::BufRead;

fn handle_filer(msg: Message) -> Result<()> {
    let FilerParams { cwd, enable_icon } = msg.params.into();
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

// TODO: generic on_init handler
pub(super) fn handle_message(msg: Message) -> Result<()> {
    let msg_id = msg.id;

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

    let source_cmd = String::from(
        msg.params
            .get("source_cmd")
            .and_then(|x| x.as_str())
            .unwrap_or("Missing source_cmd when deserializing into FilerParams"),
    );

    let size = msg
        .params
        .get("preview_size")
        .and_then(|x| x.as_u64().map(|x| x as usize))
        .unwrap_or(5);

    match provider_id {
        "filer" => {
            debug!(
                "Recv filer params: cwd:{}, enable_icon:{}",
                cwd, enable_icon
            );
            handle_filer(msg)?;
        }
        "files" => {
            let stdout_stream = fuzzy_filter::subprocess::Exec::shell(source_cmd)
                .cwd(cwd)
                .stream_stdout()?;
            let lines = std::io::BufReader::new(stdout_stream)
                .lines()
                .filter_map(|x| {
                    x.ok()
                        .and_then(|line| Some(icon::IconPainter::File.paint(&line)))
                })
                // .flatten()
                // .collect::<std::result::Result<Vec<String>, std::io::Error>>()?;
                .collect::<Vec<String>>();

            log::debug!("sending msg_id:{}, provider_id:{}", msg_id, provider_id);
            write_response(
                json!({ "id": msg_id, "provider_id": provider_id, "event": "on_init", "lines": lines, }),
            );
        }
        _ => {}
    }

    Ok(())
}
