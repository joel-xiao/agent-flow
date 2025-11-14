use std::fs;
use std::path::PathBuf;

use agentflow::{PluginKind, PluginManifest, load_plugin_manifests, schema_exports};
use clap::{Parser, Subcommand};
use serde_json::json;

#[derive(Parser)]
#[command(name = "agentflow", version, about = "AgentFlow CLI", author)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Plugins {
        #[command(subcommand)]
        command: PluginCommand,
    },
    Schema {
        #[command(subcommand)]
        command: SchemaCommand,
    },
    Flow {
        #[command(subcommand)]
        command: FlowCommand,
    },
}

#[derive(Subcommand)]
enum PluginCommand {
    List {
        #[arg(long, default_value = "plugins")]
        dir: PathBuf,
    },
}

#[derive(Subcommand)]
enum SchemaCommand {
    Export {
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long, default_value_t = true)]
        pretty: bool,
    },
}

#[derive(Subcommand)]
enum FlowCommand {
    Trace {
        id: String,
    },
}


fn main() -> anyhow::Result<()> {
    tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .try_init()
        .ok();

    let cli = Cli::parse();
    match cli.command {
        Command::Plugins { command } => match command {
            PluginCommand::List { dir } => handle_plugins_list(dir)?,
        },
        Command::Schema { command } => match command {
            SchemaCommand::Export {
                output,
                pretty,
            } => handle_schema_export(output, pretty)?,
        },
        Command::Flow { command } => match command {
            FlowCommand::Trace { id } => handle_flow_trace(id)?,
        },
    }
    Ok(())
}

fn handle_plugins_list(dir: PathBuf) -> anyhow::Result<()> {
    let manifests = load_plugin_manifests(&dir)?;
    if manifests.is_empty() {
        println!("No plugins found in directory `{}`", dir.display());
    } else {
        render_plugin_table(&manifests);
    }
    Ok(())
}

fn render_plugin_table(manifests: &[PluginManifest]) {
    println!(
        "{:<32} {:<10} {:<10} {}",
        "Name", "Version", "Kind", "Description"
    );
    for manifest in manifests {
        let description = manifest.description.clone().unwrap_or_default();
        println!(
            "{:<32} {:<10} {:<10} {}",
            manifest.name,
            manifest.version,
            render_kind(manifest.kind.clone()),
            description
        );
    }
}

fn render_kind(kind: PluginKind) -> String {
    match kind {
        PluginKind::Agent => "agent".to_string(),
        PluginKind::Tool => "tool".to_string(),
        PluginKind::Schema => "schema".to_string(),
        PluginKind::Other => "other".to_string(),
    }
}

fn handle_schema_export(
    output: Option<PathBuf>,
    pretty: bool,
) -> anyhow::Result<()> {
    let entries = schema_exports();
    let value = json!(entries);

    let content = if pretty {
        serde_json::to_string_pretty(&value)?
    } else {
        serde_json::to_string(&value)?
    };

    if let Some(path) = output {
        fs::write(&path, content)?;
        println!("Schema exported to `{}`", path.display());
    } else {
        println!("{content}");
    }
    Ok(())
}

fn handle_flow_trace(id: String) -> anyhow::Result<()> {
    println!("Flow trace `{}` is not persisted yet. Please enable event storage before querying.", id);
    Ok(())
}
