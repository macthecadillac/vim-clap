use super::filer::read_dir_entries;
use super::{write_response, Message};
use anyhow::Result;
use serde_json::json;
use std::io::BufRead;

pub struct OnInitHandler {
    pub msg_id: u64,
    pub provider_id: String,
    pub cwd: String,
    pub source_cmd: Option<String>,
}

impl From<Message> for OnInitHandler {
    fn from(msg: Message) -> Self {
        let msg_id = msg.get_message_id();
        let provider_id = msg.get_provider_id();

        let cwd = String::from(
            msg.params
                .get("cwd")
                .and_then(|x| x.as_str())
                .unwrap_or("Missing cwd when deserializing into FilerParams"),
        );

        let source_cmd = msg
            .params
            .get("source_cmd")
            .and_then(|x| x.as_str().map(Into::into));

        Self {
            msg_id,
            provider_id,
            cwd,
            source_cmd,
        }
    }
}

impl OnInitHandler {
    pub fn handle(&self) -> Result<()> {
        match self.provider_id.as_str() {
            "filer" => self.handle_filer(),
            "files" => self.handle_files(),
            _ => Ok(()),
        }
    }

    fn handle_filer(&self) -> Result<()> {
        let enable_icon = super::env::global().enable_icon;
        let result = match read_dir_entries(&self.cwd, enable_icon, None) {
            Ok(entries) => json!({
            "id": self.msg_id,
            "provider_id": "filer",
            "result": {
              "dir": self.cwd,
              "total": entries.len(),
              "event": "on_init",
              "entries": entries,
            }}),
            Err(err) => json!({
            "id": self.msg_id,
            "provider_id": "filer",
            "error": {
              "message": format!("{}", err),
              "dir": self.cwd
            }}),
        };

        write_response(result);

        Ok(())
    }

    fn handle_files(&self) -> Result<()> {
        // TODO: check 2 seconds later to see if it's finished?
        let stdout_stream = fuzzy_filter::subprocess::Exec::shell(self.source_cmd.clone().unwrap())
            .cwd(&self.cwd)
            .stream_stdout()?;
        let lines = std::io::BufReader::new(stdout_stream)
            .lines()
            .filter_map(|x| x.ok().map(|line| icon::IconPainter::File.paint(&line)))
            .collect::<Vec<String>>();

        log::debug!(
            "sending msg_id:{}, provider_id:{}",
            self.msg_id,
            self.provider_id
        );

        write_response(json!({
        "id": self.msg_id,
        "provider_id": self.provider_id,
        "result": {
          "event": "on_init",
          "lines": lines,
        }}));

        Ok(())
    }
}
