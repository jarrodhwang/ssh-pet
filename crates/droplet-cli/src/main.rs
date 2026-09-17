use clap::{Parser, Subcommand};
use droplet_core::{
    protocol::{MainRequest, PetReply, PetRequest},
    Core,
};
use std::{
    io::{self, BufRead, Write},
    path::PathBuf,
};

#[derive(Parser)]
#[command(
    name = "droplet",
    about = "Inspect Droplet connections and run bounded, unauthenticated diagnostics"
)]
struct Args {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    List,
    Doctor {
        connection: String,
        #[arg(long)]
        network: bool,
    },
    ExportTypes {
        directory: PathBuf,
    },
    Preview {
        directory: PathBuf,
    },
    /// Isolated development harness. Never loads your real connections or launches SSH.
    #[command(hide = true)]
    TestIpc,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
async fn sample() -> Result<(tempfile::TempDir, std::sync::Arc<Core>), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let core = Core::open(temp.path().join("data"), temp.path().into());
    core.execute(
        MainRequest::Import {
            name: "Zbook Studio".into(),
            command: "ssh you@studio.local".into(),
        },
        false,
    )
    .await?;
    Ok((temp, core))
}
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    match Args::parse().command {
        Command::ExportTypes { directory } => {
            droplet_core::export_types(&directory)?;
        }
        Command::Preview { directory } => {
            let (_temp, core) = sample().await?;
            std::fs::create_dir_all(&directory)?;
            std::fs::write(
                directory.join("preview.json"),
                serde_json::to_string_pretty(&core.view("")?)?,
            )?;
            std::fs::write(
                directory.join("pet-preview.json"),
                serde_json::to_string_pretty(&core.pet()?)?,
            )?;
        }
        Command::TestIpc => {
            let (_temp, core) = sample().await?;
            for line in io::stdin().lock().lines() {
                let line = line?;
                if line.len() > 65536 {
                    return Err("Input exceeds the development transport limit.".into());
                }
                let result: droplet_core::Result<serde_json::Value> = match serde_json::from_str::<
                    serde_json::Value,
                >(&line)
                {
                    Ok(envelope) => {
                        if envelope["surface"] == "pet" {
                            match serde_json::from_value::<PetRequest>(envelope["request"].clone())
                            {
                                Ok(PetRequest::View) => core.pet().and_then(|v| {
                                    serde_json::to_value(PetReply::View(v))
                                        .map_err(|e| e.to_string().into())
                                }),
                                _ => Err(droplet_core::AppError::new(
                                    droplet_core::ErrorCode::Unsupported,
                                    "This action requires the desktop app.",
                                )),
                            }
                        } else {
                            match serde_json::from_value::<MainRequest>(envelope["request"].clone())
                            {
                                Ok(request) => core.execute(request, false).await.and_then(|r| {
                                    serde_json::to_value(r).map_err(|e| e.to_string().into())
                                }),
                                Err(e) => Err(e.to_string().into()),
                            }
                        }
                    }
                    Err(e) => Err(e.to_string().into()),
                };
                let output = match result {
                    Ok(value) => serde_json::json!({"ok":true,"value":value}),
                    Err(error) => serde_json::json!({"ok":false,"error":error}),
                };
                println!("{output}");
                io::stdout().flush()?;
            }
        }
        command => {
            let home = dirs::home_dir().ok_or("Home directory unavailable")?;
            let path = dirs::config_dir()
                .ok_or("Data directory unavailable")?
                .join("com.jarrod.droplet/connections.json");
            let config = droplet_core::storage::load(&path)?.unwrap_or_default();
            match command {
                Command::List => {
                    for c in config.connections {
                        println!("{}\t{}\t{}", c.id, c.name, c.destination());
                    }
                }
                Command::Doctor {
                    connection,
                    network,
                } => {
                    let c = config
                        .connections
                        .iter()
                        .find(|c| c.id == connection || c.name == connection)
                        .ok_or("Connection not found; run droplet list first.")?;
                    let report = droplet_core::diagnostics::run(c, &home, network).await;
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                _ => unreachable!(),
            }
        }
    }
    Ok(())
}
