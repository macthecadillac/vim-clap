use super::types::{OnMove, OnMove::*};
use super::*;
use anyhow::{anyhow, Result};
use log::{debug, error};
use std::convert::TryInto;
use std::path::Path;

#[inline]
fn as_absolute_path<P: AsRef<Path>>(path: P) -> Result<String> {
    std::fs::canonicalize(path.as_ref())?
        .into_os_string()
        .into_string()
        .map_err(|e| anyhow!("{:?}, path:{}", e, path.as_ref().display()))
}

fn apply_preview_file_at<P: AsRef<Path>>(
    path: P,
    lnum: usize,
    size: usize,
    msg_id: u64,
    provider_id: &str,
) {
    match crate::utils::read_preview_lines(path.as_ref(), lnum, size) {
        Ok((lines_iter, hi_lnum)) => {
            let fname = format!("{}", path.as_ref().display());
            let lines = std::iter::once(fname.clone())
                .chain(lines_iter)
                .collect::<Vec<_>>();
            debug!("sending msg_id:{}, provider_id:{}", msg_id, provider_id);
            write_response(
                json!({ "id": msg_id, "provider_id": provider_id, "event": "on_move", "lines": lines, "fname": fname, "hi_lnum": hi_lnum }),
            );
        }
        Err(err) => {
            error!(
                "[{}]Couldn't read first lines of {}, error: {:?}",
                provider_id,
                path.as_ref().display(),
                err
            );
        }
    }
}

fn apply_preview_file<P: AsRef<Path>>(
    path: P,
    size: usize,
    msg_id: u64,
    provider_id: &str,
) -> Result<()> {
    let abs_path = as_absolute_path(path.as_ref())?;
    let lines_iter = crate::utils::read_first_lines(path.as_ref(), size)?;
    let lines = std::iter::once(abs_path.clone())
        .chain(lines_iter)
        .collect::<Vec<_>>();
    write_response(
        json!({ "id": msg_id, "provider_id": provider_id, "event": "on_move", "lines": lines, "fname": abs_path }),
    );
    Ok(())
}

fn preview_directory<P: AsRef<Path>>(
    path: P,
    size: usize,
    enable_icon: bool,
    msg_id: u64,
    provider_id: &str,
) -> Result<()> {
    let lines = super::filer::read_dir_entries(&path, enable_icon, Some(size))?;
    write_response(
        json!({ "id": msg_id, "provider_id": provider_id, "event": "on_move", "lines": lines, "is_dir": true }),
    );
    Ok(())
}

pub struct OnMoveHandler {
    pub msg_id: u64,
    pub provider_id: String,
    pub size: usize,
    pub which_move: OnMove,
}

impl From<Message> for OnMoveHandler {
    fn from(msg: Message) -> Self {
        let msg_id = msg.get_message_id();
        let provider_id = msg.get_provider_id();
        let size = super::env::preview_size_of(&provider_id);
        let which_move: OnMove = msg.try_into().expect("Couldn't into OnMove");
        Self {
            msg_id,
            provider_id,
            size,
            which_move,
        }
    }
}

impl OnMoveHandler {
    pub fn handle(&self) -> Result<()> {
        let preview_file =
            |path: &Path| apply_preview_file(path, 2 * self.size, self.msg_id, &self.provider_id);

        let preview_file_at = |path: &Path, lnum: usize| {
            apply_preview_file_at(path, lnum, self.size, self.msg_id, &self.provider_id)
        };

        match self.which_move.clone() {
            BLines { path, lnum }
            | Grep { path, lnum }
            | ProjTags { path, lnum }
            | BufferTags { path, lnum } => {
                debug!("path:{}, lnum:{}", path.display(), lnum);
                preview_file_at(&path, lnum);
            }
            Filer(path) if path.is_dir() => {
                preview_directory(
                    &path,
                    2 * self.size,
                    super::env::global().enable_icon,
                    self.msg_id,
                    "filer",
                )?;
            }
            Files(path) | Filer(path) => {
                preview_file(&path)?;
            }
        }

        Ok(())
    }
}
