use std::path::PathBuf;

pub struct Config {
    pub session_limit: usize,
    db_path_override: Option<PathBuf>,
}

impl Config {
    pub fn load() -> Self {
        let mut session_limit = 50usize;
        let mut db_path_override = None;

        if let Some(cfg_dir) = dirs::config_dir() {
            let rc = cfg_dir.join("zhli").join("config");
            if let Ok(contents) = std::fs::read_to_string(&rc) {
                for line in contents.lines() {
                    let line = line.trim();
                    if line.starts_with('#') || line.is_empty() {
                        continue;
                    }
                    if let Some(rest) = line.strip_prefix("db_path") {
                        let val = rest.trim_start_matches(|c: char| c == '=' || c.is_whitespace());
                        if !val.is_empty() {
                            db_path_override = Some(expand_tilde(val));
                        }
                    } else if let Some(rest) = line.strip_prefix("session_limit") {
                        let val = rest.trim_start_matches(|c: char| c == '=' || c.is_whitespace());
                        if let Ok(n) = val.parse::<usize>() {
                            if n > 0 {
                                session_limit = n;
                            }
                        }
                    }
                }
            }
        }

        Config { session_limit, db_path_override }
    }

    pub fn db_path(&self) -> PathBuf {
        if let Some(p) = &self.db_path_override {
            return p.clone();
        }
        let mut p = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        p.push("zhli");
        p.push("data.db");
        p
    }
}

fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(s)
}
