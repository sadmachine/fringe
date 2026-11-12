mod render;
mod terminal;

use fringe::{app::App, generator::Difficulty};
use std::{
    env,
    io::{self, IsTerminal},
    process::ExitCode,
};

#[derive(Debug, PartialEq, Eq)]
struct Options {
    seed: Option<u64>,
    difficulty: Difficulty,
    help: bool,
}

fn parse(mut args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut options = Options {
        seed: None,
        difficulty: Difficulty::Normal,
        help: false,
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => options.help = true,
            "--seed" => {
                let value = args
                    .next()
                    .ok_or("--seed needs an unsigned 64-bit number")?;
                options.seed = Some(
                    value
                        .parse()
                        .map_err(|_| "--seed needs an unsigned 64-bit number")?,
                );
            }
            "--difficulty" => {
                options.difficulty = args
                    .next()
                    .ok_or("--difficulty needs easy, normal, or hard")?
                    .parse()?
            }
            _ => return Err(format!("Unknown argument: {arg}. Use --help for usage.")),
        }
    }
    Ok(options)
}

fn run() -> Result<(), String> {
    let options = parse(env::args().skip(1))?;
    if options.help {
        println!("Fringe - procedural slide mazes\n\nUsage: fringe [--seed NUMBER] [--difficulty easy|normal|hard]\n\nChoose a difficulty, then press Enter. Slide with arrows, WASD, or HJKL.\nP pause  R retry  N new seed  M menu  Q quit\n\nRequires an interactive terminal of at least 80 columns by 24 rows.");
        return Ok(());
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(
            "Fringe needs an interactive terminal. Run it directly in a terminal, or use --help."
                .into(),
        );
    }
    let mut app = App::new(
        options.seed.unwrap_or_else(rand::random),
        options.difficulty,
    );
    terminal::run(&mut app).map_err(|e| format!("Terminal error: {e}"))
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(values: &[&str]) -> Result<Options, String> {
        parse(values.iter().map(|s| s.to_string()))
    }
    #[test]
    fn parses_replay_options_and_help_without_a_terminal() {
        assert_eq!(
            args(&["--seed", "18446744073709551615", "--difficulty", "hard"]).unwrap(),
            Options {
                seed: Some(u64::MAX),
                difficulty: Difficulty::Hard,
                help: false
            }
        );
        assert!(args(&["--help"]).unwrap().help);
    }
    #[test]
    fn rejects_invalid_seeds_difficulties_and_missing_arguments() {
        for values in [
            &["--seed", "-1"][..],
            &["--seed", "18446744073709551616"],
            &["--seed"],
            &["--difficulty"],
            &["--difficulty", "insane"],
            &["--unknown"],
        ] {
            assert!(args(values).is_err(), "{values:?}");
        }
    }
}
