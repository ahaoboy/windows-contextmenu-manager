use crate::MenuItem;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;
use winreg::RegKey;
use winreg::enums::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MenuItemType {
    Shell,
    ShellEx,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Win10MenuItem {
    pub id: String,
    pub name: String,
    pub registry_path: String,
    pub item_type: MenuItemType,
    pub enabled: bool,
    pub command: Option<String>,
    pub guid: Option<Uuid>,
    pub icon: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum Scene {
    File,
    Folder,
    Directory,
    Background,
    DesktopBackground,
    Drive,
    AllObjects,
    Computer,
    RecycleBin,
    Library,
    LibraryBackground,
    User,
    Uwp,
    SystemFileAssociations,
    Unknown,
}

impl fmt::Display for Scene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Scene::File => "File",
            Scene::Folder => "Folder",
            Scene::Directory => "Directory",
            Scene::Background => "Background",
            Scene::DesktopBackground => "DesktopBackground",
            Scene::Drive => "Drive",
            Scene::AllObjects => "AllObjects",
            Scene::Computer => "Computer",
            Scene::RecycleBin => "RecycleBin",
            Scene::Library => "Library",
            Scene::LibraryBackground => "LibraryBackground",
            Scene::User => "User",
            Scene::Uwp => "Uwp",
            Scene::SystemFileAssociations => "SystemFileAssociations",
            Scene::Unknown => "Unknown",
        };

        write!(f, "{s}")
    }
}

impl FromStr for Scene {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "File" => Ok(Scene::File),
            "Folder" => Ok(Scene::Folder),
            "Directory" => Ok(Scene::Directory),
            "Background" => Ok(Scene::Background),
            "DesktopBackground" => Ok(Scene::DesktopBackground),
            "Drive" => Ok(Scene::Drive),
            "AllObjects" => Ok(Scene::AllObjects),
            "Computer" => Ok(Scene::Computer),
            "RecycleBin" => Ok(Scene::RecycleBin),
            "Library" => Ok(Scene::Library),
            "LibraryBackground" => Ok(Scene::LibraryBackground),
            "User" => Ok(Scene::User),
            "Uwp" => Ok(Scene::Uwp),
            "SystemFileAssociations" => Ok(Scene::SystemFileAssociations),
            "Unknown" => Ok(Scene::Unknown),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegItem {
    pub id: String,
    pub data: HashMap<String, String>,
}

impl Scene {
    pub fn registry_path(&self) -> &'static str {
        match self {
            Scene::File => r"HKEY_CLASSES_ROOT\*",
            Scene::Folder => r"HKEY_CLASSES_ROOT\Folder",
            Scene::Directory => r"HKEY_CLASSES_ROOT\Directory",
            Scene::Background => r"HKEY_CLASSES_ROOT\Directory\Background",
            Scene::DesktopBackground => r"HKEY_CLASSES_ROOT\DesktopBackground",
            Scene::Drive => r"HKEY_CLASSES_ROOT\Drive",
            Scene::AllObjects => r"HKEY_CLASSES_ROOT\AllFilesystemObjects",
            Scene::Computer => r"HKEY_CLASSES_ROOT\CLSID\{20D04FE0-3AEA-1069-A2D8-08002B30309D}",
            Scene::RecycleBin => r"HKEY_CLASSES_ROOT\CLSID\{645FF040-5081-101B-9F08-00AA002F954E}",
            Scene::Library => r"HKEY_CLASSES_ROOT\LibraryFolder",
            Scene::LibraryBackground => r"HKEY_CLASSES_ROOT\LibraryFolder\Background",
            Scene::User => r"HKEY_CLASSES_ROOT\UserLibraryFolder",
            Scene::Uwp => r"HKEY_CLASSES_ROOT\Launcher.ImmersiveApplication",
            Scene::Unknown => r"HKEY_CLASSES_ROOT\Unknown",
            Scene::SystemFileAssociations => r"HKEY_CLASSES_ROOT\SystemFileAssociations",
        }
    }
}

// fn get_backup_path() -> String {
//     let d = dirs::config_dir().expect("Failed to get config directory");
//     d.join("wcm_backup.json")
//         .to_str()
//         .unwrap()
//         .to_string()
// }

fn get_backup() -> Vec<String> {
    return vec![];
}

#[derive(Debug, Clone)]
pub struct MenuItemCollection {
    pub items: HashMap<String, Win10MenuItem>,
}

impl MenuItemCollection {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    pub fn add_item(&mut self, item: Win10MenuItem) {
        self.items.insert(item.id.clone(), item);
    }

    pub fn get_item(&self, id: &str) -> Option<&Win10MenuItem> {
        self.items.get(id)
    }

    pub fn remove_item(&mut self, id: &str) -> Option<Win10MenuItem> {
        self.items.remove(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Win10MenuItem> {
        self.items.values()
    }
}

pub struct RegistryManager;

impl RegistryManager {
    pub fn get_all_menu_items() -> anyhow::Result<MenuItemCollection, anyhow::Error> {
        let mut collection = MenuItemCollection::new();

        let scenes = [
            Scene::File,
            Scene::Folder,
            Scene::Directory,
            Scene::Background,
            Scene::DesktopBackground,
            Scene::Drive,
            Scene::AllObjects,
            Scene::Computer,
            Scene::RecycleBin,
            Scene::Library,
            Scene::LibraryBackground,
            Scene::User,
            Scene::Uwp,
            Scene::SystemFileAssociations,
            Scene::Unknown,
        ];

        for scene in scenes {
            Self::load_scene_items(&mut collection, scene);
        }

        Ok(collection)
    }

    fn load_scene_items(
        collection: &mut MenuItemCollection,
        scene: Scene,
    ) -> Result<(), anyhow::Error> {
        let scene_path = scene.registry_path();

        Self::load_shell_items(collection, scene_path, scene)?;
        Self::load_shellex_items(collection, scene_path, scene)?;
        Ok(())
    }

    fn load_shell_items(
        collection: &mut MenuItemCollection,
        scene_path: &str,
        scene: Scene,
    ) -> Result<(), anyhow::Error> {
        let shell_path = format!("{scene_path}\\shell");

        let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
        let shell_key = hkcr.open_subkey(&shell_path[18..])?;

        for subkey_name in shell_key.enum_keys() {
            let subkey_name = subkey_name?;
            let subkey_path = format!("{shell_path}\\{subkey_name}");

            if let Ok(subkey) = shell_key.open_subkey(&subkey_name) {
                let item = Self::create_shell_item(&subkey_path, &subkey_name, &subkey, scene)?;
                collection.add_item(item);
            }
        }

        Ok(())
    }

    fn load_shellex_items(
        collection: &mut MenuItemCollection,
        scene_path: &str,
        scene: Scene,
    ) -> Result<(), anyhow::Error> {
        let shellex_path = format!("{scene_path}\\ShellEx");

        let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
        if let Ok(shellex_key) = hkcr.open_subkey(&shellex_path[18..]) {
            if let Ok(cmh_key) = shellex_key.open_subkey("ContextMenuHandlers") {
                Self::load_context_menu_handlers(collection, &cmh_key, &shellex_path, scene);
            }

            if let Ok(ddh_key) = shellex_key.open_subkey("DragDropHandlers") {
                Self::load_drag_drop_handlers(collection, &ddh_key, &shellex_path, scene);
            }
        }

        Ok(())
    }

    fn load_context_menu_handlers(
        collection: &mut MenuItemCollection,
        cmh_key: &RegKey,
        shellex_path: &str,
        scene: Scene,
    ) -> Result<(), anyhow::Error> {
        for subkey_name in cmh_key.enum_keys() {
            let subkey_name = subkey_name?;
            let subkey_path = format!("{shellex_path}\\ContextMenuHandlers\\{subkey_name}");

            if let Ok(subkey) = cmh_key.open_subkey(&subkey_name) {
                let item = Self::create_shellex_item(&subkey_path, &subkey_name, &subkey, scene)?;
                collection.add_item(item);
            }
        }

        Ok(())
    }

    fn load_drag_drop_handlers(
        collection: &mut MenuItemCollection,
        ddh_key: &RegKey,
        shellex_path: &str,
        scene: Scene,
    ) -> Result<(), anyhow::Error> {
        for subkey_name in ddh_key.enum_keys() {
            let subkey_name = subkey_name?;
            let subkey_path = format!("{shellex_path}\\DragDropHandlers\\{subkey_name}");

            if let Ok(subkey) = ddh_key.open_subkey(&subkey_name) {
                let item = Self::create_shellex_item(&subkey_path, &subkey_name, &subkey, scene)?;
                collection.add_item(item);
            }
        }

        Ok(())
    }

    fn create_shell_item(
        registry_path: &str,
        key_name: &str,
        key: &RegKey,
        scene: Scene,
    ) -> Result<Win10MenuItem, anyhow::Error> {
        let id = format!("shell_{}_{}", scene.to_string(), key_name);

        let name = key
            .get_value("MUIVerb")
            .or_else(|_| key.get_value(""))
            .unwrap_or_else(|_| key_name.to_string());

        let command = if let Ok(cmd_key) = key.open_subkey("command") {
            cmd_key.get_value("").ok()
        } else {
            None
        };

        let icon = key.get_value("Icon").ok();

        let enabled = key.get_value::<String, _>("OnlyInBrowserWindow").is_err();

        Ok(Win10MenuItem {
            id,
            name,
            registry_path: registry_path.to_string(),
            item_type: MenuItemType::Shell,
            enabled,
            command,
            guid: None,
            icon,
            description: None,
        })
    }

    fn create_shellex_item(
        registry_path: &str,
        key_name: &str,
        key: &RegKey,
        scene: Scene,
    ) -> Result<Win10MenuItem, anyhow::Error> {
        let id = format!("shellex_{}_{}", scene.to_string(), key_name);

        let guid_str = key.get_value("").unwrap_or_else(|_| key_name.to_string());
        let guid = Uuid::parse_str(&guid_str).ok();

        let name = if let Some(g) = guid {
            format!("GUID: {g}")
        } else {
            key_name.to_string()
        };

        let enabled = !registry_path.contains("-ContextMenuHandlers")
            && !registry_path.contains("-DragDropHandlers");

        Ok(Win10MenuItem {
            id,
            name,
            registry_path: registry_path.to_string(),
            item_type: MenuItemType::ShellEx,
            enabled,
            command: None,
            guid,
            icon: None,
            description: None,
        })
    }

    pub fn enable_menu_item(
        id: &str,
        collection: &MenuItemCollection,
    ) -> Result<(), anyhow::Error> {
        if let Some(item) = collection.get_item(id) {
            match item.item_type {
                MenuItemType::Shell => Self::enable_shell_item(item),
                MenuItemType::ShellEx => Self::enable_shellex_item(item),
            }
        } else {
            panic!("MenuItemNotFound");
        }
    }

    pub fn disable_menu_item(
        id: &str,
        collection: &MenuItemCollection,
    ) -> Result<(), anyhow::Error> {
        if let Some(item) = collection.get_item(id) {
            match item.item_type {
                MenuItemType::Shell => Self::disable_shell_item(item),
                MenuItemType::ShellEx => Self::disable_shellex_item(item),
            }
        } else {
            panic!("MenuItemNotFound");
        }
    }

    fn enable_shell_item(item: &Win10MenuItem) -> Result<(), anyhow::Error> {
        let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
        // trim prefix: "HKEY_CLASSES_ROOT\"
        let path = &item.registry_path[18..];

        if let Ok(key) = hkcr.open_subkey_with_flags(path, KEY_WRITE) {
            let _ = key.delete_value("OnlyInBrowserWindow");
        }

        Ok(())
    }

    fn disable_shell_item(item: &Win10MenuItem) -> Result<(), anyhow::Error> {
        let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
        let path = &item.registry_path[18..];

        // if let Ok(key) = hkcr.open_subkey_with_flags(path, KEY_WRITE) {
        //     key.set_value("OnlyInBrowserWindow", &"")?;
        // }

        hkcr.delete_subkey_all(path)?;

        Ok(())
    }

    fn enable_shellex_item(item: &Win10MenuItem) -> Result<(), anyhow::Error> {
        let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);

        let disabled_path = item
            .registry_path
            .replace("ContextMenuHandlers", "-ContextMenuHandlers");
        let enabled_path = item
            .registry_path
            .replace("-ContextMenuHandlers", "ContextMenuHandlers");

        if disabled_path != item.registry_path
            && let Ok(disabled_key) = hkcr.open_subkey(&disabled_path[18..])
            && let Ok((enabled_key, _)) = hkcr.create_subkey(&enabled_path[18..])
        {
            if let Some(guid) = item.guid {
                enabled_key.set_value("", &guid.to_string())?;
            }
            let _ = hkcr.delete_subkey(&disabled_path[18..]);
        }

        Ok(())
    }

    fn disable_shellex_item(item: &Win10MenuItem) -> Result<(), anyhow::Error> {
        let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);

        let enabled_path = item
            .registry_path
            .replace("-ContextMenuHandlers", "ContextMenuHandlers");
        let disabled_path = item
            .registry_path
            .replace("ContextMenuHandlers", "-ContextMenuHandlers");

        if enabled_path != item.registry_path
            && let Ok(enabled_key) = hkcr.open_subkey(&enabled_path[18..])
            && let Ok((disabled_key, _)) = hkcr.create_subkey(&disabled_path[18..])
        {
            if let Some(guid) = item.guid {
                disabled_key.set_value("", &guid.to_string())?;
            }
            let _ = hkcr.delete_subkey(&enabled_path[18..]);
        }

        Ok(())
    }
}

pub fn list() -> Vec<MenuItem> {
    let mut vv = vec![];
    if let Ok(v) = RegistryManager::get_all_menu_items() {
        for i in v.items {
            vv.push(MenuItem {
                id: i.0,
                name: i.1.name,
                enabled: i.1.enabled,
                info: None,
            });
        }
    };

    vv
}

pub fn disable(id: &str) -> Result<(), anyhow::Error> {
    let collection = RegistryManager::get_all_menu_items()?;

    RegistryManager::disable_menu_item(id, &collection)?;

    Ok(())
}

pub fn enable(id: &str) -> Result<(), anyhow::Error> {
    let collection = RegistryManager::get_all_menu_items()?;

    if let Some(item) = collection.get_item(id) {
        if item.enabled {
            return Ok(());
        }

        RegistryManager::enable_menu_item(id, &collection)?;
    } else {
        panic!("");
    }
    Ok(())
}
