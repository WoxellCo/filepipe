use crate::{
    caller::ClientState,
    config::{Binding, Config, ConfigError, init_config},
};
use clap::{Parser, Subcommand};
use filepipe::filepipe::StreamType;
use std::{format, path::PathBuf, println, process::exit, sync::Arc};

mod caller;
mod config;

#[derive(Parser)]
struct Cli {
    #[arg(long, default_value = ".")]
    root: PathBuf,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    #[command(alias = "u")]
    Up(ActionArgs),
    #[command(alias = "d")]
    Down(ActionArgs),
    //Help
}

#[derive(clap::Args)]
struct ActionArgs {
    binding: Option<String>,

    #[arg(long, short)]
    user: Option<String>,
}

const VERSION: (u32, u32, u32) = (0, 1, 0);

macro_rules! version {
    () => {
        format!("{}.{}.{}", VERSION.0, VERSION.1, VERSION.2)
    };
}

fn display_help() {
    println!("FilePipe v{} - Woxell", version!());
    println!();
    println!("[usage]");
    println!("filepipe <up | down> <binding-id> [flags]");
    println!();
    println!("[flags]");
    println!("--user (s): specify which local user to authenticate");
}

fn log_config_errors(errors: Vec<ConfigError>) {
    for error in errors {
        println!("err!");
    }
}

fn extract_config_and_command_action_or_exit_err(
    config: Result<Config, Vec<ConfigError>>,
    action: &ActionArgs,
) -> (Config, Arc<Binding>, Option<String>) {
    let config = config.unwrap_or_else(|errors| {
        log_config_errors(errors);
        exit(1);
    });

    let binding = match &action.binding {
        Some(id) => match config.bindings.get(id) {
            Some(binding) => binding.clone(),
            None => {
                println!("specified binding could not be found");
                exit(1);
            }
        },
        None => {
            if config.bindings.len() != 1 {
                println!("no binding specified");
                exit(1);
            }

            config
                .bindings
                .iter()
                .next()
                .unwrap_or_else(|| {
                    println!("couldn't load the default binding");
                    exit(1);
                })
                .1
                .clone()
        }
    };

    (config, binding, action.user.clone())
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    let config_result = init_config(&".fp/config.lua".to_string());

    let stream_type;
    let (config, binding, username);

    match &args.command {
        Some(Command::Up(action)) => {
            stream_type = StreamType::UpStream;
            (config, binding, username) =
                extract_config_and_command_action_or_exit_err(config_result, action);
        }
        Some(Command::Down(action)) => {
            stream_type = StreamType::DownStream;
            (config, binding, username) =
                extract_config_and_command_action_or_exit_err(config_result, action);
        }
        /*Some(Command::Help) | */
        None => {
            display_help();
            exit(0);
        }
    }

    let state = ClientState {
        client: reqwest::Client::new(),
        config: config.clone(),
        current_binding: binding,
    };

    let key = match state.authenticate(username).await {
        Ok(key) => key,
        Err(error) => {
            println!("err: {:?}", error);
            exit(1);
        }
    };

    let key = match state.send_open_stream_request(stream_type, key).await {
        Ok(key) => key,
        Err(error) => {
            println!("{:?}", error);
            exit(1);
        }
    };

    println!("client!! 😭");
}
