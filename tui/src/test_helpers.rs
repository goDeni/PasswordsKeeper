use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use tempfile::TempDir;

static TUI_REPO_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn acquire_repo_lock() -> MutexGuard<'static, ()> {
    TUI_REPO_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(crate) struct ScopedTuiDataDir {
    _repo_lock: MutexGuard<'static, ()>,
    pub temp_dir: TempDir,
}

impl ScopedTuiDataDir {
    pub(crate) fn new() -> Self {
        let repo_lock = acquire_repo_lock();
        let temp_dir = TempDir::new().expect("failed to create test temp dir");

        Self {
            _repo_lock: repo_lock,
            temp_dir,
        }
    }
}

pub(crate) fn test_password() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after unix epoch")
        .as_nanos();
    format!("tui-secret-{}-{nanos}", std::process::id())
}

#[cfg(test)]
mod tests {
    use super::{test_password, ScopedTuiDataDir};

    #[test]
    fn test_scoped_tui_data_dir_points_repo_into_temp_dir() {
        let scope = ScopedTuiDataDir::new();
        let repo_path = crate::repo::resolve_repo_path(scope.temp_dir.path().join("repo"));
        assert_eq!(repo_path, scope.temp_dir.path().join("repo"));
    }

    #[test]
    fn test_password_returns_non_empty_value() {
        assert!(!test_password().is_empty());
    }
}
