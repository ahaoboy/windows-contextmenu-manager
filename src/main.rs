use anyhow::Ok;
use clap::{Parser, Subcommand};
use wcm::*;

#[derive(Parser)]
#[clap(name = "wcm", about = "Windows ContextMenu Manager")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Switch {
        #[clap(value_enum)]
        r#type: Option<Type>,
    },
    Win10 {
        #[clap(subcommand)]
        command: Win10Command,
    },
    Win11 {
        #[clap(subcommand)]
        command: Win11Command,
    },
    RestartExplorer,
}

#[derive(Subcommand)]
enum Win10Command {
    List,
    Enable { id: String },
    Disable { id: String },
}

#[derive(Subcommand)]
enum Win11Command {
    List,
    Enable { scope: String, id: String },
    Disable { scope: String, id: String },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Switch { r#type: ty } => {
            match ty {
                None => {
                    println!(
                        "{}",
                        if get_context_menu_style() {
                            "win11"
                        } else {
                            "win10"
                        }
                    )
                }
                Some(Type::Win10) => {
                    set_context_menu_style(false);
                }
                Some(Type::Win11) => {
                    set_context_menu_style(true);
                }
            }
            return;
        }
        Commands::Win10 { command } => match command {
            Win10Command::List => {
                let v = Type::Win10.list();
                for i in v {
                    let icon = if i.enabled { "✅" } else { "❌" };
                    println!("{icon} {} {}", i.id, i.name);
                }
            }
            Win10Command::Enable { id } => println!("Enabling Windows 10 feature with ID: {}", id),
            Win10Command::Disable { id } => {
                println!("Disabling Windows 10 feature with ID: {}", id)
            }
        },
        Commands::Win11 { command } => match command {
            Win11Command::Enable { scope, id } => {
                let scope = match scope.to_lowercase().as_str() {
                    "user" => Scope::User,
                    "machine" => Scope::Machine,
                    _ => panic!("scope: user|machine"),
                };

                Type::Win11.enable(&id, scope);
            }
            Win11Command::Disable { scope, id } => {
                let scope = match scope.to_lowercase().as_str() {
                    "user" => Scope::User,
                    "machine" => Scope::Machine,
                    _ => panic!("scope: user|machine"),
                };

                Type::Win11.disable(&id, scope);
            }
            Win11Command::List => {
                let v = Type::Win11.list();
                for i in v {
                    let icon = if i.enabled { "✅" } else { "❌" };
                    println!("{icon} {} {}", i.id, i.name);
                }
            }
        },
        Commands::RestartExplorer => restart_explorer(),
    }
}
