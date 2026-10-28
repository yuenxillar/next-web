use std::path::PathBuf;

use async_std::task;
use clap::{Parser, Subcommand};

use crate::error::Result;
use crate::generator::{Generator, initialize_workspace};

/// CLI entry definition for the code generator.
#[derive(Debug, Parser)]
#[command(name = "next-web-code-generator")]
#[command(about = "Generate Rust mapping code from database metadata and templates")]
pub struct Cli {
    /// Command to execute.
    #[command(subcommand)]
    pub command: CliCommand,
}

/// Supported CLI commands.
#[derive(Debug, Subcommand)]
pub enum CliCommand {
    /// Create an example `generator.toml` and default templates directory.
    Init {
        /// Output configuration file path.
        #[arg(short, long, default_value = "generator.toml")]
        path: PathBuf,
        /// Overwrite an existing config file and templates.
        #[arg(long, default_value_t = false)]
        force: bool,
    },
    /// Print discovered table metadata from the configured datasource.
    Inspect {
        /// Generator configuration file path.
        #[arg(short, long, default_value = "generator.toml")]
        config: PathBuf,
        /// Datasource name. Uses the first datasource when omitted.
        #[arg(short, long)]
        datasource: Option<String>,
        /// Print output as JSON.
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Render templates and write generated code to the output directory.
    Generate {
        /// Generator configuration file path.
        #[arg(short, long, default_value = "generator.toml")]
        config: PathBuf,
        /// Datasource name. Uses the first datasource when omitted.
        #[arg(short, long)]
        datasource: Option<String>,
        /// Show the generation plan without writing files.
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
}

/// Execute a CLI command.
pub fn run_cli(cli: Cli) -> Result<()> {
    match cli.command {
        CliCommand::Init { path, force } => {
            initialize_workspace(&path, force)?;
            println!("Initialized config: {}", path.display());
        }
        CliCommand::Inspect {
            config,
            datasource,
            json,
        } => {
            let generator = Generator::from_path(&config)?;
            let tables = task::block_on(generator.inspect(datasource.as_deref()))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&tables)?);
            } else {
                print_table_summary(&tables);
            }
        }
        CliCommand::Generate {
            config,
            datasource,
            dry_run,
        } => {
            let generator = Generator::from_path(&config)?;
            if dry_run {
                let files = task::block_on(generator.plan(datasource.as_deref()))?;
                for file in files {
                    println!(
                        "[{}] {}",
                        file.template_name,
                        file.relative_path.to_string_lossy()
                    );
                }
            } else {
                let files = task::block_on(generator.generate(datasource.as_deref()))?;
                for file in files {
                    println!("generated {}", file.display());
                }
            }
        }
    }

    Ok(())
}

fn print_table_summary(tables: &[crate::model::TableInfo]) {
    for table in tables {
        println!("table: {}", table.name);
        if let Some(comment) = &table.comment {
            println!("  comment: {}", comment);
        }
        println!("  columns:");
        for column in &table.columns {
            println!(
                "    - {} -> {} ({}){}{}",
                column.name,
                column.rust_name,
                column.rust_type,
                if column.nullable { " nullable" } else { "" },
                if column.primary_key {
                    " primary-key"
                } else {
                    ""
                }
            );
        }
    }
}
