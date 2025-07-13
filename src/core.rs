use serde::Serialize;
use std::fmt;
use std::str::FromStr;
use winreg::HKEY;
use winreg::enums::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    User,
    Machine,
}

impl Scope {
    pub fn to_hive(self) -> HKEY {
        match self {
            Scope::User => HKEY_CURRENT_USER,
            Scope::Machine => HKEY_LOCAL_MACHINE,
        }
    }
}
impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Scope::User => write!(f, "user"),
            Scope::Machine => write!(f, "machine"),
        }
    }
}

impl FromStr for Scope {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "user" => Ok(Scope::User),
            "machine" => Ok(Scope::Machine),
            _ => Err("Invalid scope string, expected 'user' or 'machine'"),
        }
    }
}

pub trait Manager {
    fn list(&self, scope: Scope) -> Vec<MenuItem>;
    fn disable(&self, id: &str, scope: Scope) -> Result<(), anyhow::Error>;
    fn enable(&self, id: &str, scope: Scope) -> Result<(), anyhow::Error>;
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct MenuItem {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub info: Option<MenuItemInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct MenuItemInfo {
    pub icon: Option<Vec<u8>>,
    pub publisher_display_name: String,
    pub description: String,
    pub types: Vec<String>,
}

#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Type {
    Win10,
    Win11,
}

impl Manager for Type {
    fn list(&self, scope: Scope) -> Vec<MenuItem> {
        match self {
            Type::Win10 => crate::win10::list(),
            Type::Win11 => crate::win11::list(scope),
        }
    }

    fn disable(&self, id: &str, scope: Scope) -> Result<(), anyhow::Error> {
        match self {
            Type::Win10 => crate::win10::disable(id),
            Type::Win11 => crate::win11::disable(id, scope),
        }
    }

    fn enable(&self, id: &str, scope: Scope) -> Result<(), anyhow::Error> {
        match self {
            Type::Win10 => crate::win10::enable(id),
            Type::Win11 => crate::win11::enable(id, scope),
        }
    }
}

use serde::Deserialize;
use std::io::{self};
use winreg::RegKey;

const CLSID_PATH: &str =
    r"SOFTWARE\Classes\CLSID\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}\InprocServer32";

pub fn set_context_menu_style(is_win11_style: bool) -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if is_win11_style {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu.open_subkey_with_flags(CLSID_PATH, KEY_SET_VALUE)?;
        key.delete_value("")?;
    } else {
        let clsid_path = r"Software\Classes\CLSID\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}";
        let (clsid_key, _) = hkcu.create_subkey(clsid_path)?;
        let (inproc_key, _) = clsid_key.create_subkey("InprocServer32")?;
        inproc_key.set_value("", &"")?;
    }
    Ok(())
}

pub fn get_context_menu_style() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.open_subkey(CLSID_PATH).is_ok()
}

use std::process::Command;
use std::process::Stdio;

pub fn restart_explorer() {
    let _ = Command::new("taskkill")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .arg("/f")
        .arg("/im")
        .arg("explorer.exe")
        .spawn();
    std::thread::sleep(std::time::Duration::from_millis(2000));
    let _ = Command::new("explorer.exe").spawn();
}

impl Type {
    pub fn enable_classic_menu() -> io::Result<()> {
        set_context_menu_style(false)
    }

    pub fn disable_classic_menu() -> io::Result<()> {
        set_context_menu_style(true)
    }

    pub fn menu_type() -> Type {
        if get_context_menu_style() {
            Type::Win11
        } else {
            Type::Win10
        }
    }
}
