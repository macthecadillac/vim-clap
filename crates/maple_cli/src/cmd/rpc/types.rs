use super::Message;
use anyhow::{anyhow, Context};
use pattern::{
    extract_blines_lnum, extract_buf_tags_lnum, extract_grep_position, extract_proj_tags,
};
use serde_json::value::Value;
use std::convert::TryFrom;
use std::path::PathBuf;

/// Preview environment on Vim CursorMoved event.
#[allow(dead_code)]
pub enum ProviderExtended {
    Files(PathBuf),
    Filer(PathBuf),
    Grep { path: PathBuf, lnum: usize },
    BLines { path: PathBuf, lnum: usize },
    ProjTags { path: PathBuf, lnum: usize },
    BufferTags { path: PathBuf, lnum: usize },
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
    super::env::global().enable_icon && has_icon_support(provider_id)
}

impl TryFrom<Message> for ProviderExtended {
    type Error = anyhow::Error;
    fn try_from(msg: Message) -> std::result::Result<Self, Self::Error> {
        let provider_id = msg
            .params
            .get("provider_id")
            .and_then(|x| x.as_str())
            .unwrap_or("Unknown provider id");

        let cwd = msg
            .params
            .get("cwd")
            .and_then(|x| x.as_str())
            .unwrap_or("Missing cwd when deserializing into FilerParams");

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

        let get_source_fpath = || {
            msg.params
                .get("source_fpath")
                .and_then(|x| x.as_str().map(Into::into))
                .context("Missing source_fpath")
        };

        // Rebuild the absolute path using cwd and relative path.
        let rebuild_abs_path = || {
            let mut path: PathBuf = cwd.into();
            path.push(&curline);
            path
        };

        log::debug!("curline: {}", curline);
        let provider_ext = match provider_id {
            "files" => Self::Files(rebuild_abs_path()),
            "filer" => Self::Filer(rebuild_abs_path()),
            "blines" => {
                let lnum = extract_blines_lnum(&curline).context("Couldn't extract buffer lnum")?;
                let path = get_source_fpath()?;
                Self::BLines { path, lnum }
            }
            "tags" => {
                let lnum =
                    extract_buf_tags_lnum(&curline).context("Couldn't extract buffer tags")?;
                let path = get_source_fpath()?;
                Self::BufferTags { path, lnum }
            }

            "proj_tags" => {
                let (lnum, p) =
                    extract_proj_tags(&curline).context("Couldn't extract proj tags")?;
                let mut path: PathBuf = cwd.into();
                path.push(&p);
                Self::ProjTags { path, lnum }
            }
            "grep" | "grep2" => {
                let (fpath, lnum, _col) =
                    extract_grep_position(&curline).context("Couldn't extract grep position")?;
                let mut path: PathBuf = cwd.into();
                path.push(&fpath);
                Self::Grep { path, lnum }
            }
            _ => {
                return Err(anyhow!(
                    "Couldn't into PreviewEnv from Message: {:?}, unknown provider_id: {}",
                    msg,
                    provider_id
                ))
            }
        };

        Ok(provider_ext)
    }
}
