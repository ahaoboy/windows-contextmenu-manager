use serde::Serialize;
use std::collections::HashMap;
use strum::IntoEnumIterator;
use strum_macros::Display;
use strum_macros::EnumIter;
use strum_macros::EnumString;
use winreg::HKEY;
use winreg::RegKey;
use winreg::enums::*;

pub const APP_NAME: &str = "windows-contextmenu-manager";
pub const BACKUP_NAME: &str = "backup.json";
pub const WIN10_SKIP_REGKEY: [&str; 13] = [
    "ContextMenuHandlers",
    "CopyHookHandlers",
    "DragDropHandlers",
    "PropertySheetHandlers",
    "UpdateEncryptionSettings",
    "UpdateEncryptionSettingsWork",
    "DefaultIcon",
    "shell",
    "ShellFolder",
    "LibraryDescriptionHandler",
    "IconHandler",
    "SharingHandler",
    "removeproperties"
];

#[derive(
    Debug,
    Clone,
    Default,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Deserialize,
    Serialize,
    EnumIter,
    EnumString,
    Display,
)]
pub enum Scope {
    #[default]
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

pub trait Manager {
    fn list(&self, scope: Option<Scope>) -> Vec<MenuItem>;
    fn disable(&self, id: &str, scope: Option<Scope>) -> Result<(), anyhow::Error>;
    fn enable(&self, id: &str, scope: Option<Scope>) -> Result<(), anyhow::Error>;
}
#[derive(Debug, Clone, Default, PartialEq, Eq,   Deserialize, Serialize)]
pub struct MenuItem {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub info: Option<MenuItemInfo>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct TypeItem {
    pub id: String,
    pub ty: String,
    pub clsid: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq,   Deserialize, Serialize)]
pub struct MenuItemInfo {
    pub icon: Option<Vec<u8>>,
    pub publisher_display_name: String,
    pub description: String,
    pub types: Vec<TypeItem>,
    pub install_path: String,
    pub family_name: String,
    pub full_name: String,
    pub reg: Option<RegItem>
}

#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Type {
    Win10,
    #[default]
    Win11,
}

impl Manager for Type {
    fn list(&self, scope: Option<Scope>) -> Vec<MenuItem> {
        match self {
            Type::Win10 => crate::win10::list(),
            Type::Win11 => crate::win11::list(scope.unwrap_or_default()),
        }
    }

    fn disable(&self, id: &str, scope: Option<Scope>) -> Result<(), anyhow::Error> {
        match self {
            Type::Win10 => crate::win10::disable(id),
            Type::Win11 => crate::win11::disable(id, scope.unwrap_or_default()),
        }
    }

    fn enable(&self, id: &str, scope: Option<Scope>) -> Result<(), anyhow::Error> {
        match self {
            Type::Win10 => crate::win10::enable(id),
            Type::Win11 => crate::win11::enable(id, scope.unwrap_or_default()),
        }
    }
}

use serde::Deserialize;
use std::io::{self};

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
    std::thread::sleep(std::time::Duration::from_millis(1000));
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct RegItem {
    pub path: String,
    pub values: HashMap<String, String>,
    pub children: Vec<RegItem>,
}

impl RegItem {
    pub fn from_path(path: &str) -> io::Result<RegItem> {
        let reg_key = RegKey::predef(HKEY_CLASSES_ROOT).open_subkey(path)?;

        let mut values = HashMap::new();
        for value_name in reg_key.enum_values().map(|x| x.unwrap().0) {
            let value = reg_key.get_value::<String, _>(&value_name);
            if let Ok(val) = value {
                values.insert(value_name, val);
            }
        }

        let mut children: Vec<RegItem> = Vec::new();
        for subkey_name in reg_key.enum_keys().map(|x| x.unwrap()) {
            let subkey_path = format!("{path}\\{subkey_name}");
            let subkey_item = RegItem::from_path(&subkey_path)?;
            children.push(subkey_item);
        }

        Ok(RegItem {
            path: path.to_string(),
            values,
            children,
        })
    }

    fn is_safe(&self) -> bool {
        for i in Scene::iter().flat_map(|s| s.registry_path().to_vec()) {
            if self.path.starts_with(i) {
                return true;
            }
        }

        false
    }

    pub fn write(&self) {
        if !self.is_safe() {
            return;
        }
        let root = RegKey::predef(HKEY_CLASSES_ROOT);
        if let Ok((key, _disp)) = root.create_subkey(&self.path) {
            for (name, value) in &self.values {
                let _ = key.set_value(name.as_str(), value);
            }
            for child in &self.children {
                child.write();
            }
        }
    }

    pub fn delete(&self) -> io::Result<()> {
        if !self.is_safe() {
            return Ok(());
        }
        let reg_key = RegKey::predef(HKEY_CLASSES_ROOT)
            .open_subkey_with_flags(self.path.clone(), KEY_WRITE)?;
        for i in &self.children {
            let _ = i.delete();
        }
        reg_key.delete_subkey_with_flags("", KEY_WRITE)?;
        Ok(())
    }
}

#[derive(
    Debug,
    Clone,
    Default,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Deserialize,
    Serialize,
    EnumIter,
    EnumString,
    Display,
)]
pub enum Scene {
    #[default]
    File,
    Folder,
    Desktop,
    Directory,
    Background,
    Drive,
    AllObjects,
    Computer,
    RecycleBin,
    Library,
    LibraryBackground,
    User,
    Uwp,
    SystemFileAssociations,
    // Unknown,
}

impl Scene {
    pub fn registry_path(&self) -> &[&'static str] {
        match self {
            Scene::File => &[r"*\shell", r"*\ShellEx", r"*\OpenWithList"],
            Scene::Folder => &[r"Folder\shell", r"Folder\ShellEx"],
            Scene::Background => &[
                r"Directory\Background\Shell",
                r"Directory\Background\ShellEx",
            ],
            Scene::Directory => &[r"Directory\Shell", r"Directory\ShellEx"],
            Scene::Desktop => &[r"DesktopBackground\Shell", r"DesktopBackground\ShellEx"],
            Scene::Drive => &[r"Drive\Shell", r"Drive\ShellEx"],
            Scene::AllObjects => &[
                r"AllFilesystemObjects\Shell",
                r"AllFilesystemObjects\ShellEx",
            ],
            Scene::Computer => &[r"CLSID\{20D04FE0-3AEA-1069-A2D8-08002B30309D}"],
            Scene::RecycleBin => &[
                r"CLSID\{645FF040-5081-101B-9F08-00AA002F954E}\Shell",
                r"CLSID\{645FF040-5081-101B-9F08-00AA002F954E}\ShellEx",
            ],
            Scene::Library => &[r"LibraryFolder\Shell", r"LibraryFolder\ShellEx"],
            Scene::LibraryBackground => &[
                r"LibraryFolder\Background\Shell",
                r"LibraryFolder\Background\ShellEx",
            ],
            Scene::User => &[r"UserLibraryFolder\Shell", r"UserLibraryFolder\ShellEx"],
            Scene::Uwp => &[
                r"Launcher.ImmersiveApplication\Shell",
                r"Launcher.ImmersiveApplication\ShellEx",
            ],
            Scene::SystemFileAssociations => &[r"SystemFileAssociations"],
            // Scene::Unknown => &[r"Unknown"],
        }
    }
}
