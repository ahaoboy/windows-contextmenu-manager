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
                    let _ = set_context_menu_style(false);
                }
                Some(Type::Win11) => {
                    let _ = set_context_menu_style(true);
                }
            }
            return;
        }
        Commands::Win10 { command } => match command {
            Win10Command::List => {
                let v = Type::Win10.list(None);
                for i in v {
                    let icon = if i.enabled { "✅" } else { "❌" };
                    println!("{icon} {} {}", i.id, i.name);
                }
            }
            Win10Command::Enable { id } => {
                let _ =  Type::Win10.enable(&id, None);
            }
            Win10Command::Disable { id } => {
            let _ =    Type::Win10.disable(&id, None);
            }
        },
        Commands::Win11 { command } => match command {
            Win11Command::Enable { scope, id } => {
                if scope == Scope::Machine && !is_admin::is_admin() {
                    panic!("You must run this command as an administrator.");
                }
             let _ =     Type::Win11.enable(&id, Some(scope));
            }
            Win11Command::Disable { scope, id } => {
                if scope == Scope::Machine && !is_admin::is_admin() {
                    panic!("You must run this command as an administrator.");
                }
             let _ =     Type::Win11.disable(&id, Some(scope));
            }
            Win11Command::List { scope } => {
                if scope == Scope::Machine && !is_admin::is_admin() {
                    panic!("You must run this command as an administrator.");
                }
                let v = Type::Win11.list(Some(scope));
                for i in v {
                    let icon = if i.enabled { "✅" } else { "❌" };
                    println!("{icon} {} {}", i.id, i.name,);
                }
            }
        },
        Commands::RestartExplorer => restart_explorer(),
    }
}
