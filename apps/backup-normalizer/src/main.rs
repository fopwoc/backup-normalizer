mod application;
mod backup_file;
mod cli;
mod error;
mod period;
mod policy;
mod scanner;

use std::process::ExitCode;

use clap::Parser;
use tracing::error;
use tracing_subscriber::EnvFilter;

use crate::application::Application;
use crate::cli::Cli;

fn main() -> ExitCode {
    init_logging();

    let config = match Cli::parse().into_config() {
        Ok(config) => config,
        Err(error) => {
            error!(%error, "invalid configuration");
            return ExitCode::FAILURE;
        }
    };

    match Application::new(config).run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            error!(%error, "backup normalizer stopped");
            ExitCode::FAILURE
        }
    }
}

fn init_logging() {
    let filter = EnvFilter::try_from_env("LOG_LEVEL").unwrap_or_else(|_| EnvFilter::new("error"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
