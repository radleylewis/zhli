use std::path::PathBuf;

pub struct Config {
    pub session_limit: usize,
    pub warnings: Vec<String>,
    db_path_override: Option<PathBuf>,
}

impl Config {
    pub fn load() -> Self {
        let mut session_limit = 50usize;
        let mut db_path_override = None;
        let mut warnings = Vec::new();

        if let Some(cfg_dir) = dirs::config_dir() {
            let rc = cfg_dir.join("zhli").join("config");
            if let Ok(contents) = std::fs::read_to_string(&rc) {
                let parsed = parse_config(&contents);
                session_limit = parsed.0;
                db_path_override = parsed.1;
                warnings = parsed.2;
            }
        }

        Config {
            session_limit,
            db_path_override,
            warnings,
        }
    }

    pub fn data_dir(&self) -> PathBuf {
        self.db_path()
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
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

/// Parse the contents of a config file. Returns (session_limit, db_path_override, warnings).
/// Extracted for testability.
fn parse_config(contents: &str) -> (usize, Option<PathBuf>, Vec<String>) {
    let mut session_limit = 50usize;
    let mut db_path_override = None;
    let mut warnings = Vec::new();

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
            if val.is_empty() {
                // key present but no value — leave default
            } else if let Ok(n) = val.parse::<usize>() {
                if n > 0 {
                    session_limit = n;
                } else {
                    warnings.push("session_limit must be > 0; using default".to_string());
                }
            } else {
                warnings.push(format!("invalid value for 'session_limit': '{val}'"));
            }
        } else {
            let key = line
                .split(|c: char| c == '=' || c.is_whitespace())
                .next()
                .unwrap_or(line)
                .trim();
            if !key.is_empty() {
                warnings.push(format!("unknown config key '{key}'"));
            }
        }
    }

    (session_limit, db_path_override, warnings)
}

fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_config ──────────────────────────────────────────────────────────

    #[test]
    fn defaults_when_empty() {
        let (limit, path, warns) = parse_config("");
        assert_eq!(limit, 50);
        assert!(path.is_none());
        assert!(warns.is_empty());
    }

    #[test]
    fn comments_and_blank_lines_ignored() {
        let input = "# this is a comment\n\n# another\n";
        let (limit, path, warns) = parse_config(input);
        assert_eq!(limit, 50);
        assert!(path.is_none());
        assert!(warns.is_empty());
    }

    #[test]
    fn session_limit_parsed() {
        let (limit, _, warns) = parse_config("session_limit = 100");
        assert_eq!(limit, 100);
        assert!(warns.is_empty());
    }

    #[test]
    fn session_limit_zero_warns_and_keeps_default() {
        let (limit, _, warns) = parse_config("session_limit = 0");
        assert_eq!(limit, 50);
        assert!(!warns.is_empty());
    }

    #[test]
    fn session_limit_non_numeric_warns() {
        let (limit, _, warns) = parse_config("session_limit = abc");
        assert_eq!(limit, 50);
        assert!(warns.iter().any(|w| w.contains("session_limit")));
    }

    #[test]
    fn db_path_absolute_parsed() {
        let (_, path, warns) = parse_config("db_path = /tmp/test.db");
        assert_eq!(path, Some(PathBuf::from("/tmp/test.db")));
        assert!(warns.is_empty());
    }

    #[test]
    fn db_path_tilde_expanded() {
        if let Some(home) = dirs::home_dir() {
            let (_, path, warns) = parse_config("db_path = ~/mydb.db");
            assert_eq!(path, Some(home.join("mydb.db")));
            assert!(warns.is_empty());
        }
    }

    #[test]
    fn unknown_key_generates_warning() {
        let (_, _, warns) = parse_config("font_size = 14");
        assert_eq!(warns.len(), 1);
        assert!(warns[0].contains("font_size"));
    }

    #[test]
    fn multiple_unknown_keys_each_warn() {
        let input = "foo = 1\nbar = 2\n";
        let (_, _, warns) = parse_config(input);
        assert_eq!(warns.len(), 2);
    }

    #[test]
    fn known_and_unknown_keys_mixed() {
        let input = "session_limit = 20\nbad_key = x\ndb_path = /tmp/x.db\n";
        let (limit, path, warns) = parse_config(input);
        assert_eq!(limit, 20);
        assert!(path.is_some());
        assert_eq!(warns.len(), 1);
        assert!(warns[0].contains("bad_key"));
    }

    // ── expand_tilde ──────────────────────────────────────────────────────────

    #[test]
    fn expand_tilde_absolute_path_unchanged() {
        assert_eq!(expand_tilde("/tmp/foo"), PathBuf::from("/tmp/foo"));
    }

    #[test]
    fn expand_tilde_relative_path_unchanged() {
        assert_eq!(
            expand_tilde("relative/path"),
            PathBuf::from("relative/path")
        );
    }

    #[test]
    fn expand_tilde_home_prefix_expanded() {
        if let Some(home) = dirs::home_dir() {
            assert_eq!(expand_tilde("~/docs/file.db"), home.join("docs/file.db"));
        }
    }
}
