use crate::replay::constants::{REPLAY_DIRECTORY, REPLAY_LIST_ROWS};
use anyhow::Result;
use std::fs;

pub struct ReplayListView {
    replay_files: Vec<String>,
    page: usize,
    row: usize,
}

impl ReplayListView {
    pub fn new() -> Result<Self> {
        Ok(Self {
            replay_files: Self::load_replay_files()?,
            page: 0,
            row: 0,
        })
    }

    pub fn get_current_page_items_pretty(&self) -> Vec<String> {
        let start = self.page * REPLAY_LIST_ROWS;

        (start..start + REPLAY_LIST_ROWS)
            .filter_map(|i| {
                let f = self.replay_files.get(i)?;

                let f = f.strip_prefix(REPLAY_DIRECTORY)?;
                let f = f.strip_suffix(".prp")?;

                Some(f.to_string())
            })
            .collect()
    }

    pub fn get_selected_row_index(&self) -> usize {
        self.row
    }

    pub fn current_page(&self) -> usize {
        self.page
    }

    pub fn down(&mut self) {
        let max_row = if self.page == self.total_pages() - 1 {
            self.items_on_last_page()
        } else {
            REPLAY_LIST_ROWS
        };

        if self.row + 1 < max_row {
            self.row += 1;
        } else {
            let old_page = self.page;
            self.right();

            if old_page != self.page {
                self.row = 0;
            }
        }
    }

    pub fn up(&mut self) {
        if self.row > 0 {
            self.row -= 1;
        } else {
            let old_page = self.page;
            self.left();
            if old_page != self.page {
                self.row = REPLAY_LIST_ROWS - 1;
            }
        }
    }

    pub fn left(&mut self) {
        if self.page > 0 {
            self.page -= 1;
        }
    }

    pub fn right(&mut self) {
        if self.page < self.total_pages() - 1 {
            self.page += 1;
        }

        if self.page == self.total_pages() - 1 && self.row > self.items_on_last_page() {
            self.row = self.items_on_last_page() - 1;
        }
    }

    pub fn selected(&self) -> Option<String> {
        let index = self.page * REPLAY_LIST_ROWS + self.row;
        self.replay_files.get(index).cloned()
    }

    pub fn total_pages(&self) -> usize {
        self.replay_files.len().div_ceil(REPLAY_LIST_ROWS)
    }

    fn items_on_last_page(&self) -> usize {
        let length = self.replay_files.len();
        let rem = length % REPLAY_LIST_ROWS;
        if rem == 0 && length != 0 {
            REPLAY_LIST_ROWS
        } else {
            rem
        }
    }

    fn load_replay_files() -> Result<Vec<String>> {
        let mut files = Vec::new();

        for entry in fs::read_dir(REPLAY_DIRECTORY)? {
            let path = entry?.path();

            if path.is_file()
                && let Some(ext) = path.extension()
                && ext == "prp"
                && let Some(path_str) = path.to_str()
            {
                files.push(path_str.to_string());
            }
        }

        Ok(files)
    }
}
