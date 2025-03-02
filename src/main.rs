#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]

mod cache;
mod commands;
mod gpt;
mod parse;

use std::io;

use anyhow::Result;
use fstrings::*;
use lazy_static::lazy_static;
use owo_colors::OwoColorize;

lazy_static! {
    static ref SUPPORTED_LANGS_HELP: String = {
        let mut supported_langs = env!("LTCC_LANGS")
            .split(',')
            .collect::<Vec<&str>>()
            .join("\n  - ");

        let mut header = "SUPPORTED LANGUAGES:".to_string();

        if std::env::var("NO_COLOR").is_err() {
            header = "SUPPORTED LANGUAGES:".yellow().to_string();
            supported_langs = supported_langs.green().to_string();
        }

        return f!("{header}\n  - {supported_langs}");
    };
}

fn build_cli() -> clap::Command<'static> {
    return clap::Command::new("translate-comments-cpp")
        .about("Get source code comments to speak your language!")
        .after_help(SUPPORTED_LANGS_HELP.as_str())
        .version(env!("VERGEN_GIT_SEMVER"))
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            clap::Command::new("translate")
                .about("Parses source code comments from the provided file and passes them to GPT service, returning comments translations.")
                .arg(
                    clap::Arg::new("input")
                        .long("input")
                        .short('i')
                        .help("Path to input source code file.")
                        .value_hint(clap::ValueHint::FilePath)
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(
                    clap::Arg::new("output")
                        .long("output")
                        .short('o')
                        .help("Path to output source code file.")
                        .value_hint(clap::ValueHint::FilePath)
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(
                    clap::Arg::new("concurrency")
                        .long("concurrency")
                        .short('c')
                        .default_value("10")
                        .help("Maximum amount of requests to make to GPT service in parallel.")
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(
                    clap::Arg::new("language")
                        .long("language")
                        .short('l')
                        .default_value("en-US")
                        .help("New language of source code comment blocks, in form of language code, such as en-US, fr-FR or es-MX.")
                        .takes_value(true)
                        .multiple_values(false),
                ),

        )
        .subcommand(
            clap::Command::new("cache")
                .about("Translation results caching options.")
                .setting(clap::AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    clap::Command::new("path").about("Outputs the cache directories path")
                )
                .subcommand(
                    clap::Command::new("delete").about("Deletes the entire cache directory")
                )
        );
}

fn print_completions<G: clap_complete::Generator>(gen: G, app: &mut clap::Command) {
    clap_complete::generate(gen, app, app.get_name().to_string(), &mut io::stdout());
}

async fn parse_cli() -> Result<()> {
    let matches = build_cli().get_matches();
    match matches.subcommand() {
        Some(("translate", run_matches)) => {
            let input = run_matches.get_one::<String>("input").unwrap().to_string();
            let output = run_matches.get_one::<String>("output").map(|s| s.to_string());
            let language = run_matches
                .get_one::<String>("language")
                .unwrap()
                .to_string();
            let concurrency = run_matches
                .get_one::<String>("concurrency")
                .unwrap()
                .parse::<usize>()?;

            let res =
                commands::translate(input.clone(), concurrency, language).await?;

            if let Some(output_path) = output {
                // Read the input file
                let input_content = std::fs::read_to_string(&input)?;

                // Replace occurrences of QueryResult.text with QueryResult.text_translation
                let mut modified_content = input_content.clone();
                for query_result in &res {
                    modified_content = modified_content.replace(&query_result.text, &query_result.text_translation);
                }

                // Write the modified content to the output file
                std::fs::write(output_path, modified_content)?;
            } else {
                // Print JSON results if no output file is specified
                println!("{}", serde_json::to_string(&res)?);
            }
        }
        Some(("completion", run_matches)) => {
            if let Ok(generator) = run_matches.value_of_t::<clap_complete::Shell>("shell") {
                eprintln!("Generating completion file for {}...", generator);
                let mut app = build_cli();
                print_completions(generator, &mut app);
            }
        }
        Some(("cache", args)) => {
            match args.subcommand() {
                Some(("delete", _)) => {
                    cache::delete_cache().await?;
                }
                Some(("path", _)) => {
                    println!("{}", cache::get_dir_path().await?.to_str().unwrap());
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }

    return Ok(());
}

#[tokio::main]
async fn main() {
    parse_cli().await.unwrap();
}
