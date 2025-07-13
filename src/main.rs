use anyhow::Ok;
use clap::{Parser, Subcommand};
use std::str::FromStr;
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
    List { scope: Scope },
    Enable { id: String },
    Disable { id: String },
}

#[derive(Subcommand)]
enum Win11Command {
    List { scope: Scope },
    Enable { scope: Scope, id: String },
    Disable { scope: Scope, id: String },
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
            Win10Command::List { scope } => {
                if scope == Scope::Machine && !is_admin::is_admin() {
                    panic!("You must run this command as an administrator.");
                }
                let v = Type::Win10.list(scope);
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
                if scope == Scope::Machine && !is_admin::is_admin() {
                    panic!("You must run this command as an administrator.");
                }
                Type::Win11.enable(&id, scope);
            }
            Win11Command::Disable { scope, id } => {
                if scope == Scope::Machine && !is_admin::is_admin() {
                    panic!("You must run this command as an administrator.");
                }
                Type::Win11.disable(&id, scope);
            }
            Win11Command::List { scope } => {
                if scope == Scope::Machine && !is_admin::is_admin() {
                    panic!("You must run this command as an administrator.");
                }
                let v = Type::Win11.list(scope);
                for i in v {
                    let icon = if i.enabled { "✅" } else { "❌" };

                    println!("{icon} {} {}", i.id, i.name,);
                }
            }
        },
        Commands::RestartExplorer => restart_explorer(),
    }
}
