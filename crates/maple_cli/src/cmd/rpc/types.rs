use super::Message;
use anyhow::{anyhow, Context};
use pattern::{extract_blines_lnum, extract_buf_tags_lnum, extract_proj_tags};
use serde_json::value::Value;
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
    pub provider: Provider,
}

pub enum Provider {
    Files(PathBuf),
    Grep(GrepPreviewEntry),
    Filer { path: PathBuf },
    ProjTags { path: PathBuf, lnum: usize },
    BufferTags { path: PathBuf, lnum: usize },
    BLines { path: PathBuf, lnum: usize },
}

#[derive(Debug, Clone)]
pub struct GlobalEnv {
    pub is_nvim: bool,
    pub enable_icon: bool,
    pub preview_size: Value,
}

impl GlobalEnv {
    pub fn new(is_nvim: bool, enable_icon: bool, preview_size: Value) -> Self {
        Self {
            is_nvim,
            enable_icon,
            preview_size,
        }
    }

    pub fn preview_size_of(&self, provider_id: &str) -> usize {
        match self.preview_size {
            serde_json::value::Value::Number(ref number) => number.as_u64().unwrap() as usize,
            serde_json::value::Value::Object(ref obj) => {
                let get_size = |key: &str| {
                    obj.get(key)
                        .and_then(|x| x.as_u64().map(|i| i as usize))
                        .unwrap()
                };
                if obj.contains_key(provider_id) {
                    get_size(provider_id)
                } else if obj.contains_key("*") {
                    get_size("*")
                } else {
                    5usize
                }
            }
            _ => unreachable!("clap_preview_size has to be either Number or Object"),
        }
    }
}

fn has_icon_support(provider_id: &str) -> bool {
    provider_id != "proj_tags" && provider_id != "blines"
}

fn should_skip_leading_icon(provider_id: &str) -> bool {
    super::env().enable_icon && has_icon_support(provider_id)
}

impl TryFrom<Message> for PreviewEnv {
    type Error = anyhow::Error;
    fn try_from(msg: Message) -> std::result::Result<Self, Self::Error> {
        let provider_id = msg
            .params
            .get("provider_id")
            .and_then(|x| x.as_str())
            .unwrap_or("Unknown provider id");

        let cwd = String::from(
            msg.params
                .get("cwd")
                .and_then(|x| x.as_str())
                .unwrap_or("Missing cwd when deserializing into FilerParams"),
        );

        let display_curline = String::from(
            msg.params
                .get("curline")
                .and_then(|x| x.as_str())
                .unwrap_or("Missing fname when deserializing into FilerParams"),
        );

        let curline = if should_skip_leading_icon(provider_id) {
            display_curline.chars().skip(2).collect()
        } else {
            display_curline
        };

        let provider = match provider_id {
            "files" => {
                let mut fpath: PathBuf = cwd.into();
                fpath.push(&curline);
                Provider::Files(fpath)
            }
            "blines" => {
                let lnum = extract_blines_lnum(&curline).context("Couldn't extract buffer lnum")?;
                let path = msg
                    .params
                    .get("source_fpath")
                    .and_then(|x| x.as_str().map(Into::into))
                    .context("Missing fname when deserializing into FilerParams")?;
                Provider::BLines { path, lnum }
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
                Provider::Filer { path }
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

        Ok(Self { provider })
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
