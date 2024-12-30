use std::collections::HashSet;
use std::fmt::Display;
use std::fs::File;
use std::io::prelude::Read;
use std::path::Path;
use teloxide::types::UserId;

pub struct Whitelist {
    allowed_ids: HashSet<UserId>,
}

impl Display for Whitelist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.allowed_ids.is_empty() {
            write!(f, "[all]")
        } else {
            write!(
                f,
                "{}",
                self.allowed_ids
                    .iter()
                    .map(|v| v.to_string())
                    .reduce(|a, b| format!("'{}', '{}'", a, b))
                    .map_or("[]".to_string(), |v| format!("[{}]", v))
            )
        }
    }
}

impl Default for Whitelist {
    fn default() -> Self {
        Self::new()
    }
}

impl Whitelist {
    pub fn new() -> Self {
        Self {
            allowed_ids: HashSet::new(),
        }
    }

    pub fn read<P: AsRef<Path>>(file: P) -> std::io::Result<Self> {
        let mut data = String::new();
        File::open(file)?.read_to_string(&mut data)?;

        let allowed_ids = HashSet::from_iter(
            data.lines()
                .map(|line| line.trim())
                .filter(|line| line.len().gt(&0))
                .filter_map(|line| {
                    line.parse::<u64>().map_or_else(
                        |err| {
                            log::warn!("Failed line parse \"{}\": {}", line, err);
                            None
                        },
                        |num| Some(UserId(num)),
                    )
                }),
        );

        Ok(Whitelist { allowed_ids })
    }

    pub fn check_allowed(&self, user_id: &UserId) -> bool {
        self.allowed_ids
            .is_empty()
            .then_some(true)
            .unwrap_or_else(|| self.allowed_ids.contains(user_id))
    }
}
