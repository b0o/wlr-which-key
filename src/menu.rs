use std::str::FromStr;

use anyhow::{bail, Result};
use pangocairo::{cairo, pango};
use wayrs_utils::keyboard::xkb;

use crate::color::Color;
use crate::config::{self, Config};
use crate::key::{Key, ModifierState};
use crate::text::{self, ComputedText};
use crate::DEBUG_LAYOUT;

pub struct Menu {
    pages: Vec<MenuPage>,
    cur_page: usize,
    separator: ComputedText,
}

struct MenuPage {
    item_height: f64,
    columns: Vec<MenuColumn>,
    key_mappings: Vec<(Key, Action)>,
    parent: Option<usize>,
}

struct MenuColumn {
    key_col_width: f64,
    val_col_width: f64,
    items: Vec<MenuItem>,
}

struct MenuItem {
    key_comp: ComputedText,
    val_comp: ComputedText,
}

#[derive(Clone)]
pub enum Action {
    Quit,
    Exec { cmd: String, keep_open: bool },
    Submenu(usize),
}

impl Menu {
    pub fn new(config: &Config) -> Result<Self> {
        let context = pango::Context::new();
        let fontmap = pangocairo::FontMap::new();
        context.set_font_map(Some(&fontmap));

        let mut this = Self {
            pages: Vec::new(),
            cur_page: 0,
            separator: ComputedText::new(&config.separator, &context, &config.font.0),
        };

        this.push_page(&context, &config.menu, config, None)?;

        Ok(this)
    }

    fn push_page(
        &mut self,
        context: &pango::Context,
        entries: &[config::Entry],
        config: &Config,
        parent: Option<usize>,
    ) -> Result<usize> {
        if entries.is_empty() {
            bail!("Empty menu pages are not allowed");
        }

        let cur_page = self.pages.len();

        self.pages.push(MenuPage {
            item_height: self.separator.height,
            columns: Vec::new(),
            key_mappings: Vec::new(),
            parent,
        });

        let mut visible_entry_i = 0;
        for entry in entries.iter() {
            let (key, action, item, is_hidden) = match entry {
                config::Entry::Cmd {
                    key,
                    cmd,
                    desc,
                    keep_open,
                    hide,
                } => {
                    let action = Action::Exec {
                        cmd: cmd.into(),
                        keep_open: *keep_open,
                    };
                    let item = MenuItem {
                        key_comp: ComputedText::new(key.to_string(), context, &config.font.0),
                        val_comp: ComputedText::new(desc, context, &config.font.0),
                    };
                    (key.clone(), action, item, *hide)
                }
                config::Entry::Recursive {
                    key,
                    submenu: entries,
                    desc,
                    hide,
                } => {
                    let new_page = self.push_page(context, entries, config, Some(cur_page))?;
                    let action = Action::Submenu(new_page);
                    let item = MenuItem {
                        key_comp: ComputedText::new(key.to_string(), context, &config.font.0),
                        val_comp: ComputedText::new(format!("+{desc}"), context, &config.font.0),
                    };
                    (key.clone(), action, item, *hide)
                }
            };

            // Store key mapping for input handling (all items)
            self.pages[cur_page].key_mappings.push((key, action));

            // Skip hidden items for layout/rendering
            if is_hidden {
                continue;
            }

            let height = f64::max(item.key_comp.height, item.val_comp.height);
            if height > self.pages[cur_page].item_height {
                self.pages[cur_page].item_height = height;
            }

            let col_i = config
                .rows_per_column
                .map_or(0, |rows_per_column| visible_entry_i / rows_per_column);

            if col_i == self.pages[cur_page].columns.len() {
                self.pages[cur_page].columns.push(MenuColumn {
                    key_col_width: item.key_comp.width,
                    val_col_width: item.val_comp.width,
                    items: vec![item],
                });
            } else {
                let col = &mut self.pages[cur_page].columns[col_i];
                col.key_col_width = col.key_col_width.max(item.key_comp.width);
                col.val_col_width = col.val_col_width.max(item.val_comp.width);
                col.items.push(item);
            }

            visible_entry_i += 1;
        }

        Ok(cur_page)
    }

    pub fn width(&self, config: &Config) -> f64 {
        let page = &self.pages[self.cur_page];
        if page.columns.is_empty() {
            return (config.padding() + config.border_width) * 2.0;
        }
        page.columns
            .iter()
            .map(|col| col.key_col_width + col.val_col_width + self.separator.width)
            .sum::<f64>()
            + (page.columns.len() - 1) as f64 * config.column_padding()
            + (config.padding() + config.border_width) * 2.0
    }

    pub fn height(&self, config: &Config) -> f64 {
        let page = &self.pages[self.cur_page];
        if page.columns.is_empty() {
            return (config.padding() + config.border_width) * 2.0;
        }
        page.columns
            .iter()
            .map(|col| page.item_height * col.items.len() as f64)
            .max_by(f64::total_cmp)
            .unwrap()
            + (config.padding() + config.border_width) * 2.0
    }

    pub fn render(&self, config: &config::Config, cairo_ctx: &cairo::Context) -> Result<()> {
        let mut dx = config.padding() + config.border_width;
        let dy = config.padding() + config.border_width;
        let page = &self.pages[self.cur_page];
        for col in &page.columns {
            self.render_column(config, cairo_ctx, dx, dy, page, col)?;
            dx += col.key_col_width
                + col.val_col_width
                + self.separator.width
                + config.column_padding();
        }
        Ok(())
    }

    fn render_column(
        &self,
        config: &Config,
        cairo_ctx: &cairo::Context,
        dx: f64,
        dy: f64,
        page: &MenuPage,
        column: &MenuColumn,
    ) -> Result<()> {
        for (i, comp) in column.items.iter().enumerate() {
            comp.key_comp.render(
                cairo_ctx,
                text::RenderOptions {
                    x: dx + column.key_col_width - comp.key_comp.width,
                    y: dy + page.item_height * (i as f64),
                    fg_color: config.color,
                    height: page.item_height,
                },
            )?;
            self.separator.render(
                cairo_ctx,
                text::RenderOptions {
                    x: dx + column.key_col_width,
                    y: dy + page.item_height * (i as f64),
                    fg_color: config.color,
                    height: page.item_height,
                },
            )?;
            comp.val_comp.render(
                cairo_ctx,
                text::RenderOptions {
                    x: dx + column.key_col_width + self.separator.width,
                    y: dy + page.item_height * (i as f64),
                    fg_color: config.color,
                    height: page.item_height,
                },
            )?;
        }

        if *DEBUG_LAYOUT {
            Color::from_rgba(0, 0, 255, 255).apply(cairo_ctx);
            cairo_ctx.rectangle(
                dx,
                dy,
                column.key_col_width + column.val_col_width + self.separator.width,
                column.items.len() as f64 * page.item_height,
            );
            cairo_ctx.set_line_width(1.0);
            cairo_ctx.stroke().unwrap();
        }

        Ok(())
    }

    pub fn get_action(&self, xkb: &xkb::State, sym: xkb::Keysym) -> Option<Action> {
        let page = &self.pages[self.cur_page];
        let modifiers = ModifierState::from_xkb_state(xkb);

        let action = page
            .key_mappings
            .iter()
            .find_map(|(key, action)| key.matches(sym, modifiers).then(|| action.clone()));
        if action.is_some() {
            return action;
        }

        match sym {
            xkb::Keysym::Escape => {
                return Some(Action::Quit);
            }
            xkb::Keysym::bracketleft | xkb::Keysym::g if modifiers.mod_ctrl => {
                return Some(Action::Quit);
            }
            xkb::Keysym::BackSpace => {
                if let Some(parent) = page.parent {
                    return Some(Action::Submenu(parent));
                }
            }
            _ => (),
        }

        None
    }

    pub fn set_page(&mut self, page: usize) {
        self.cur_page = page;
    }

    pub fn navigate_to_key_sequence(&mut self, key_sequence: &str) -> Result<Option<Action>> {
        let keys: Vec<&str> = key_sequence.split_whitespace().collect();
        if keys.is_empty() {
            return Ok(None);
        }

        let mut current_page = 0;

        for (i, key_str) in keys.iter().enumerate() {
            let key = Key::from_str(key_str)
                .map_err(|_| anyhow::anyhow!("Invalid key: '{}'", key_str))?;

            let page = &self.pages[current_page];
            let mut found = false;

            for mapping in page.key_mappings.iter() {
                let (mapped_key, action) = mapping;
                if mapped_key.to_string() == key.to_string() {
                    match action {
                        Action::Submenu(submenu_page) => {
                            current_page = *submenu_page;
                            found = true;
                            break;
                        }
                        action @ (Action::Exec { .. } | Action::Quit) => {
                            if i == keys.len() - 1 {
                                // This is the final key, execute the action
                                return Ok(Some(action.clone()));
                            } else {
                                bail!(
                                    "Key '{}' leads to a command, but more keys follow in sequence",
                                    key_str
                                );
                            }
                        }
                    }
                }
                if found {
                    break;
                }
            }

            if !found {
                bail!("Key '{}' not found in current menu", key_str);
            }
        }

        self.cur_page = current_page;
        Ok(None)
    }
}
