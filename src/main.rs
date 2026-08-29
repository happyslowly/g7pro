use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "g7pro", about = "GameSir G7 Pro 8K tools")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Battery {
        #[arg(long)]
        watch: bool,

        #[arg(long)]
        raw: bool,
    },
    Rumble,
    Buttons,
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Battery { watch, raw } => g7pro::commands::battery(watch, raw),
        Command::Rumble => g7pro::commands::rumble_test(),
        Command::Buttons => g7pro::commands::button_test(),
    };

    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,

        Err(err) => {
            eprintln!("error: {err}");

            std::process::ExitCode::FAILURE
        }
    }
}
