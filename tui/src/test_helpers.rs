use std::ffi::OsString;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use tempfile::TempDir;

static TUI_DATA_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn acquire_env_lock() -> MutexGuard<'static, ()> {
    TUI_DATA_ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(crate) struct ScopedTuiDataDir {
    _env_lock: MutexGuard<'static, ()>,
    previous_env: Option<OsString>,
    pub temp_dir: TempDir,
}

impl ScopedTuiDataDir {
    pub(crate) fn new() -> Self {
        Self::new_with_lock(acquire_env_lock())
    }

    fn new_with_lock(env_lock: MutexGuard<'static, ()>) -> Self {
        let previous_env = std::env::var_os("PASSWORDS_KEEPER_TUI_DATA");
        let temp_dir = TempDir::new().expect("failed to create test temp dir");
        crate::repo::clear_configured_data_dir();

        // SAFETY: tests serialize env mutations via TUI_DATA_ENV_LOCK.
        unsafe {
            std::env::set_var("PASSWORDS_KEEPER_TUI_DATA", temp_dir.path());
        }

        Self {
            _env_lock: env_lock,
            previous_env,
            temp_dir,
        }
    }
}

impl Drop for ScopedTuiDataDir {
    fn drop(&mut self) {
        crate::repo::clear_configured_data_dir();
        if let Some(value) = self.previous_env.as_ref() {
            // SAFETY: tests serialize env mutations via TUI_DATA_ENV_LOCK.
            unsafe {
                std::env::set_var("PASSWORDS_KEEPER_TUI_DATA", value);
            }
        } else {
            // SAFETY: tests serialize env mutations via TUI_DATA_ENV_LOCK.
            unsafe {
                std::env::remove_var("PASSWORDS_KEEPER_TUI_DATA");
            }
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
    use super::{acquire_env_lock, test_password, ScopedTuiDataDir};

    #[test]
    fn test_scoped_tui_data_dir_sets_env_to_temp_path() {
        let scope = ScopedTuiDataDir::new();
        let current =
            std::env::var_os("PASSWORDS_KEEPER_TUI_DATA").expect("env var should be set by scope");
        assert_eq!(current, scope.temp_dir.path().as_os_str());
    }

    #[test]
    fn test_scoped_tui_data_dir_restores_previous_env_value() {
        let env_lock = acquire_env_lock();

        // SAFETY: this test mutates process env in a controlled, single-threaded scope.
        unsafe {
            std::env::set_var("PASSWORDS_KEEPER_TUI_DATA", "/tmp/original-tui-data");
        }

        {
            let _scope = ScopedTuiDataDir::new_with_lock(env_lock);
            let current = std::env::var("PASSWORDS_KEEPER_TUI_DATA")
                .expect("env var should be set inside scope");
            assert_ne!(current, "/tmp/original-tui-data");
        }

        let restored =
            std::env::var("PASSWORDS_KEEPER_TUI_DATA").expect("env var should be restored");
        assert_eq!(restored, "/tmp/original-tui-data");
    }

    #[test]
    fn test_password_returns_non_empty_value() {
        assert!(!test_password().is_empty());
    }
}
