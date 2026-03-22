use std::path::PathBuf;

use clap::Parser;

use crate::{default_repo_path, ConnectionMode};

#[derive(Debug, Clone, Parser, PartialEq, Eq)]
#[command(name = "tui")]
#[command(about = "PasswordsKeeper terminal UI")]
pub struct CliArgs {
    #[arg(
        long,
        value_enum,
        default_value_t = ConnectionMode::File,
        help = "Choose how the TUI connects to a repository."
    )]
    pub connection: ConnectionMode,

    #[arg(
        long,
        value_name = "PATH",
        default_value_os_t = default_repo_path(),
        help = "Use PATH as the repository file when --connection=file."
    )]
    pub repo_file: PathBuf,

    #[arg(
        long,
        value_name = "PATH",
        help = "Load remote server connection settings from PATH when --connection=remote."
    )]
    pub remote_config: Option<PathBuf>,
}

impl CliArgs {
    pub fn validate(self) -> Result<Self, clap::Error> {
        match (&self.connection, &self.remote_config) {
            (ConnectionMode::Remote, None) => Err(clap::Error::raw(
                clap::error::ErrorKind::MissingRequiredArgument,
                "--remote-config is required when --connection=remote",
            )),
            _ => Ok(self),
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use crate::ConnectionMode;

    use super::CliArgs;

    #[test]
    fn test_parse_no_args() {
        let args = CliArgs::parse_from(["tui"])
            .validate()
            .expect("args should validate");
        assert_eq!(args.connection, ConnectionMode::File);
        assert!(args.repo_file.ends_with("passwords_keeper_tui_data/repo"));
        assert_eq!(args.remote_config, None);
    }

    #[test]
    fn test_parse_repo_file_arg() {
        let args = CliArgs::parse_from(["tui", "--repo-file", "/tmp/custom-repo"])
            .validate()
            .expect("args should validate");
        assert_eq!(args.repo_file, std::path::Path::new("/tmp/custom-repo"));
    }

    #[test]
    fn test_parse_missing_repo_file_value_fails() {
        let err = CliArgs::try_parse_from(["tui", "--repo-file"]).expect_err("parse must fail");
        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidValue);
    }

    #[test]
    fn test_remote_mode_requires_remote_config() {
        let err = CliArgs::parse_from(["tui", "--connection", "remote"])
            .validate()
            .expect_err("validation must fail");
        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn test_parse_remote_config_arg() {
        let args = CliArgs::parse_from([
            "tui",
            "--connection",
            "remote",
            "--remote-config",
            "/tmp/remote.toml",
        ])
        .validate()
        .expect("args should validate");
        assert_eq!(args.connection, ConnectionMode::Remote);
        assert_eq!(
            args.remote_config,
            Some(std::path::PathBuf::from("/tmp/remote.toml"))
        );
    }

    #[test]
    fn test_help_includes_repo_file_description() {
        let mut command = CliArgs::command();
        let mut help = Vec::new();
        command.write_long_help(&mut help).expect("write help");
        let help = String::from_utf8(help).expect("utf8 help");

        assert!(help.contains("--connection <CONNECTION>"));
        assert!(help.contains("--repo-file <PATH>"));
        assert!(help.contains("--remote-config <PATH>"));
        assert!(help.contains("Use PATH as the repository file when --connection=file."));
    }
}
