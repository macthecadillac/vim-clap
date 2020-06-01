use super::filer::read_dir_entries;
use super::*;
use anyhow::Result;

pub struct OnTypedHandler {
    pub msg_id: u64,
    pub provider_id: String,
    pub cwd: String,
    pub source_cmd: Option<String>,
}

impl From<Message> for OnTypedHandler {
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

impl OnTypedHandler {
    pub fn handle(&self) -> Result<()> {
        match self.provider_id.as_str() {
            "filer" => self.handle_filer(),
            _ => Ok(()),
        }
    }

    fn handle_filer(&self) -> Result<()> {
        let enable_icon = super::env::global().enable_icon;
        let result = match read_dir_entries(&self.cwd, enable_icon, None) {
            Ok(entries) => {
                let result = json!({
                "entries": entries,
                "dir": self.cwd,
                "total": entries.len(),
                });
                json!({ "id": self.msg_id, "provider_id": self.provider_id, "event": "on_init", "result": result })
            }
            Err(err) => {
                let error = json!({"message": format!("{}", err), "dir": self.cwd});
                json!({ "id": self.msg_id, "provider_id": self.provider_id, "error": error })
            }
        };

        write_response(result);

        Ok(())
    }
}
