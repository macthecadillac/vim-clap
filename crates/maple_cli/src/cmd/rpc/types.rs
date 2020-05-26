use super::Message;
use anyhow::anyhow;
use anyhow::Context;
use pattern::{extract_buf_tags_lnum, extract_proj_tags};
use std::convert::{TryFrom, TryInto};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GrepPreviewEntry {
    pub fpath: PathBuf,
    pub lnum: usize,
    pub col: usize,
}

impl TryFrom<String> for GrepPreviewEntry {
    type Error = anyhow::Error;
    fn try_from(line: String) -> std::result::Result<Self, Self::Error> {
        let (fpath, lnum, col) =
            pattern::extract_grep_position(&line).context("Couldn't extract grep position")?;
        Ok(Self { fpath, lnum, col })
    }
}

/// Preview environment on Vim CursorMoved event.
pub struct PreviewEnv {
    /// Number of lines to preview.
    pub size: usize,
    pub provider: Provider,
}

pub enum Provider {
    Files(PathBuf),
    Grep(GrepPreviewEntry),
    Filer { path: PathBuf, enable_icon: bool },
    ProjTags { path: PathBuf, lnum: usize },
    BufferTags { path: PathBuf, lnum: usize },
}

impl TryFrom<Message> for PreviewEnv {
    type Error = anyhow::Error;
    fn try_from(msg: Message) -> std::result::Result<Self, Self::Error> {
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

        let fname_with_icon = String::from(
            msg.params
                .get("curline")
                .and_then(|x| x.as_str())
                .unwrap_or("Missing fname when deserializing into FilerParams"),
        );

        let curline = if enable_icon && provider_id != "proj_tags" {
            fname_with_icon.chars().skip(2).collect()
        } else {
            fname_with_icon
        };

        let size = msg
            .params
            .get("preview_size")
            .and_then(|x| x.as_u64().map(|x| x as usize))
            .unwrap_or(5);

        let provider = match provider_id {
            "files" => {
                let mut fpath: PathBuf = cwd.into();
                fpath.push(&curline);
                Provider::Files(fpath)
            }
            "tags" => {
                let lnum =
                    extract_buf_tags_lnum(&curline).context("Couldn't extract buffer tags")?;
                let path = msg
                    .params
                    .get("source_fpath")
                    .and_then(|x| x.as_str().map(Into::into))
                    .context("Missing fname when deserializing into FilerParams")?;
                Provider::BufferTags { path, lnum }
            }

            "proj_tags" => {
                let (lnum, p) =
                    extract_proj_tags(&curline).context("Couldn't extract proj tags")?;
                let mut path: PathBuf = cwd.into();
                path.push(&p);
                Provider::ProjTags { path, lnum }
            }
            "filer" => {
                let mut path: PathBuf = cwd.into();
                path.push(&curline);
                let enable_icon = msg
                    .params
                    .get("enable_icon")
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false);
                Provider::Filer { path, enable_icon }
            }
            "grep" | "grep2" => {
                let mut preview_entry: GrepPreviewEntry = curline.try_into()?;
                let mut with_cwd: PathBuf = cwd.into();
                with_cwd.push(&preview_entry.fpath);
                preview_entry.fpath = with_cwd;
                Provider::Grep(preview_entry)
            }
            _ => {
                return Err(anyhow!(
                    "Couldn't into PreviewEnv from Message: {:?}, unknown provider_id: {}",
                    msg,
                    provider_id
                ))
            }
        };

        Ok(Self { size, provider })
    }
}

#[test]
fn test_grep_regex() {
    use std::convert::TryInto;
    let line = "install.sh:1:5:#!/usr/bin/env bash";
    let e: GrepPreviewEntry = String::from(line).try_into().unwrap();
    assert_eq!(
        e,
        GrepPreviewEntry {
            fpath: "install.sh".into(),
            lnum: 1,
            col: 5
        }
    );
}
