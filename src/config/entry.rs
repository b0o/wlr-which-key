use anyhow::{bail, Context};
use serde::Deserialize;

use crate::key::Key;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum Alias {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Deserialize)]
#[serde(try_from = "RawEntry")]
pub enum Entry {
    Cmd {
        key: Key,
        aliases: Vec<Key>,
        cmd: String,
        desc: String,
        keep_open: bool,
        hide: bool,
    },
    Recursive {
        key: Key,
        aliases: Vec<Key>,
        submenu: Vec<Self>,
        desc: String,
        hide: bool,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEntry {
    key: Key,
    desc: String,
    cmd: Option<String>,
    keep_open: Option<bool>,
    submenu: Option<Vec<Entry>>,
    hide: Option<bool>,
    alias: Option<Alias>,
}

impl TryFrom<RawEntry> for Entry {
    type Error = anyhow::Error;

    fn try_from(value: RawEntry) -> Result<Self, Self::Error> {
        let aliases = match value.alias {
            Some(Alias::Single(alias_str)) => {
                vec![alias_str.parse().map_err(|e| anyhow::anyhow!("Invalid alias key '{}': {}", alias_str, e))?]
            }
            Some(Alias::Multiple(alias_strs)) => {
                alias_strs
                    .into_iter()
                    .map(|s| s.parse().map_err(|e| anyhow::anyhow!("Invalid alias key '{}': {}", s, e)))
                    .collect::<Result<Vec<Key>, _>>()?
            }
            None => Vec::new(),
        };

        if let Some(submenu) = value.submenu {
            if value.cmd.is_some() {
                bail!("cannot have both 'submenu' and 'cmd'");
            }
            if value.keep_open.is_some() {
                bail!("cannot have both 'submenu' and 'keep_open'");
            }
            Ok(Self::Recursive {
                key: value.key,
                aliases,
                submenu,
                desc: value.desc,
                hide: value.hide.unwrap_or(false),
            })
        } else {
            Ok(Self::Cmd {
                key: value.key,
                aliases,
                cmd: value
                    .cmd
                    .context("either or 'submenu' or 'cmd' is required")?,
                desc: value.desc,
                keep_open: value.keep_open.unwrap_or(false),
                hide: value.hide.unwrap_or(false),
            })
        }
    }
}
