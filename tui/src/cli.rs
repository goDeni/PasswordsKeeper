use std::path::PathBuf;

use clap::Parser;

use crate::default_repo_path;

#[derive(Debug, Clone, Parser, PartialEq, Eq)]
#[command(name = "tui")]
#[command(about = "PasswordsKeeper terminal UI")]
pub struct CliArgs {
    #[arg(
        long,
        value_name = "PATH",
        default_value_os_t = default_repo_path(),
        help = "Use PATH as the repository file."
    )]
    pub repo_file: PathBuf,
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::CliArgs;

    #[test]
    fn test_parse_no_args() {
        let args = CliArgs::parse_from(["tui"]);
        assert!(args.repo_file.ends_with("passwords_keeper_tui_data/repo"));
    }

    #[test]
    fn test_parse_repo_file_arg() {
        let args = CliArgs::parse_from(["tui", "--repo-file", "/tmp/custom-repo"]);
        assert_eq!(args.repo_file, std::path::Path::new("/tmp/custom-repo"));
    }

    #[test]
    fn test_parse_missing_repo_file_value_fails() {
        let err = CliArgs::try_parse_from(["tui", "--repo-file"]).expect_err("parse must fail");
        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidValue);
    }

    #[test]
    fn test_help_includes_repo_file_description() {
        let mut command = CliArgs::command();
        let mut help = Vec::new();
        command.write_long_help(&mut help).expect("write help");
        let help = String::from_utf8(help).expect("utf8 help");

        assert!(help.contains("--repo-file <PATH>"));
        assert!(help.contains("Use PATH as the repository file."));
    }
}
