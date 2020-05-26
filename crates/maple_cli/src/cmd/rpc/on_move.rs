use super::types::{PreviewEnv, Provider};
use super::*;
use anyhow::Result;
use log::error;
use std::convert::TryInto;
use std::path::Path;

#[inline]
fn canonicalize_and_as_str<P: AsRef<Path>>(path: P) -> String {
    std::fs::canonicalize(path)
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap()
}

fn preview_file_at<P: AsRef<Path>>(
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

fn preview_file<P: AsRef<Path>>(path: P, size: usize, msg_id: u64, provider_id: &str) {
    let abs_path = canonicalize_and_as_str(path.as_ref());
    let lines_iter = match crate::utils::read_first_lines(path.as_ref(), size) {
        Ok(i) => i,
        Err(e) => {
            error!(
                "[{}]Couldn't read first lines of {}, error: {:?}",
                provider_id,
                path.as_ref().display(),
                e
            );
            return;
        }
    };
    let lines = std::iter::once(abs_path.clone())
        .chain(lines_iter)
        .collect::<Vec<_>>();
    write_response(
        json!({ "id": msg_id, "provider_id": provider_id, "event": "on_move", "lines": lines, "fname": abs_path }),
    );
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

pub(super) fn handle_message_on_move(msg: Message) -> Result<()> {
    let msg_id = msg.id;

    let PreviewEnv { size, provider } = msg.try_into()?;

    match provider {
        Provider::Grep(preview_entry) => {
            preview_file_at(
                &preview_entry.fpath,
                preview_entry.lnum,
                size,
                msg_id,
                "grep",
            );
        }
        Provider::ProjTags { path, lnum } => {
            preview_file_at(&path, lnum, size, msg_id, "proj_tags");
        }
        Provider::BufferTags { path, lnum } => {
            preview_file_at(&path, lnum, size, msg_id, "tags");
        }
        Provider::Filer { path, enable_icon } => {
            if path.is_dir() {
                preview_directory(&path, 2 * size, enable_icon, msg_id, "filer")?;
            } else {
                preview_file(&path, 2 * size, msg_id, "filer");
            }
        }
        Provider::Files(fpath) => {
            preview_file(&fpath, 2 * size, msg_id, "files");
        }
    }

    Ok(())
}
