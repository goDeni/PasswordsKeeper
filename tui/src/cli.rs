use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone, Parser, PartialEq, Eq)]
#[command(name = "tui")]
#[command(about = "PasswordsKeeper terminal UI")]
pub struct CliArgs {
    #[arg(long, value_name = "PATH")]
    pub data_dir: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::CliArgs;

    #[test]
    fn test_parse_no_args() {
        let args = CliArgs::parse_from(["tui"]);
        assert_eq!(args.data_dir, None);
    }

    #[test]
    fn test_parse_data_dir_arg() {
        let args = CliArgs::parse_from(["tui", "--data-dir", "/tmp/pk"]);
        assert_eq!(
            args.data_dir.as_deref(),
            Some(std::path::Path::new("/tmp/pk"))
        );
    }

    #[test]
    fn test_parse_missing_data_dir_value_fails() {
        let err = CliArgs::try_parse_from(["tui", "--data-dir"]).expect_err("parse must fail");
        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidValue);
    }
}
