use std::path::PathBuf;

use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(super)))]
pub(crate) enum Error {
    #[snafu(display("Failed to start command: {}", source))]
    CommandStart { source: std::io::Error },

    #[snafu(display("Failed to execute command: 'docker {:?}'", args))]
    DockerExecution { args: String },

    #[snafu(display("Input url is required"))]
    InputFile,

    #[snafu(display("Input file {:?} must be a file", path))]
    InputFileBad { path: PathBuf },

    #[snafu(display("Bad file url '{}': {}", url, source))]
    InputUrl {
        url: String,
        source: url::ParseError,
    },

    #[snafu(display("mod-dir is required"))]
    ModDir,

    #[snafu(display("output-dir is required"))]
    OutputDir,

    #[snafu(display("Missing environment variable '{}'", var))]
    Environment {
        var: String,
        source: std::env::VarError,
    },
}

pub(super) type Result<T> = std::result::Result<T, Error>;
