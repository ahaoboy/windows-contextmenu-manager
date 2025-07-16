use crate::APP_NAME;
use crate::BACKUP_NAME;
use crate::MenuItem;
use crate::MenuItemInfo;
use crate::RegItem;
use crate::Scene;
use strum::IntoEnumIterator;
use windows::core::PCWSTR;
pub struct RegistryManager;

fn get_icon(path: &str) -> Option<Vec<u8>> {
    if path.contains(",") {
        if let Some((path, id)) = path.split_once(",") {
            let bin = std::fs::read(path).ok()?;
            let id = id.parse().ok()?;
            return exeico::get_ico(&bin, id).ok();
        }
    } else {
        let bin = std::fs::read(path).ok()?;
        let v = exeico::get_icos(&bin).ok()?;
        return Some(v.first()?.data.clone());
    }
    None
}

use windows::Win32::UI::Shell::SHLoadIndirectString;

fn load_indirect_string(s: &str) -> Option<String> {
    let mut buffer = [0u16; 512];

    let wide_input: Vec<u16> = s.encode_utf16().chain(Some(0)).collect();

    unsafe {
        SHLoadIndirectString(PCWSTR(wide_input.as_ptr()), &mut buffer, None).ok()?;
    }

    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..len]))
}

fn get_handle_name(i: &RegItem) -> Option<String> {
    if let Some(handle) = i
        .values
        .get("DelegateExecute")
        .or(i.values.get("CommandStateHandler"))
        .or(i.values.get("ExplorerCommandHandler"))
        && let Ok(handle_reg) = RegItem::from_path(&format!("CLSID\\{handle}"))
        && let Some(handle_name) = handle_reg.values.get("")
        && !handle_name.is_empty()
    {
        return Some(handle_name.to_string());
    }

    None
}

fn get_backup() -> Vec<RegItem> {
    let Some(d) = dirs::config_local_dir() else {
        return vec![];
    };

    let app_dir = d.join(APP_NAME);
    if !std::fs::exists(&app_dir).unwrap_or(false) {
        let _ = std::fs::create_dir_all(&app_dir);
    }

    let backup_path = app_dir.join(BACKUP_NAME);

    let Ok(s) = std::fs::read_to_string(backup_path) else {
        return vec![];
    };

    serde_json::from_str(&s).unwrap_or_default()
}

fn set_backup(items: &Vec<RegItem>) {
    let Some(d) = dirs::config_local_dir() else {
        return;
    };

    let app_dir = d.join(APP_NAME);
    if !std::fs::exists(&app_dir).unwrap_or(false) {
        let _ = std::fs::create_dir_all(&app_dir);
    }

    let backup_path = app_dir.join(BACKUP_NAME);

    if let Ok(s) = serde_json::to_string_pretty(items) {
        let _ = std::fs::write(backup_path, &s);
    }
}

impl RegistryManager {
    pub fn get_all_menu_items() -> anyhow::Result<Vec<MenuItem>, anyhow::Error> {
        let mut v = vec![];
        let mut reg_backup = vec![];
        for secne in Scene::iter() {
            for scene_path in secne.registry_path() {
                let Ok(reg) = RegItem::from_path(scene_path) else {
                    continue;
                };
                for item in reg.children {
                    reg_backup.push(item.clone());
                    let info = MenuItemInfo {
                        icon: item.values.get("Icon").and_then(|v| get_icon(v)),
                        publisher_display_name: String::new(),
                        description: String::new(),
                        types: vec![],
                        install_path: String::new(),
                        family_name: String::new(),
                        full_name: String::new(),
                    };
                    let mut name = item
                        .path
                        .split('\\')
                        .next_back()
                        .unwrap_or_default()
                        .to_string();
                    if let Some(s) = item.values.get("") {
                        // TODO: ignore "": "@shell32.dll,-8506"
                        if !s.contains(",") {
                            name = s.clone();
                        }
                        if s.starts_with("@")
                            && s.contains(",")
                            && let Some(load_str) = load_indirect_string(s)
                        {
                            name = load_str;
                        }
                    }

                    if let Some(handle_name) = get_handle_name(&item) {
                        name = handle_name
                    }

                    if let Some(child) =
                        item.children.iter().find(|c| c.path.ends_with("\\command"))
                        && let Some(child_name) = get_handle_name(child)
                    {
                        name = child_name;
                    }
                    let item = MenuItem {
                        id: item.path.clone(),
                        name,
                        enabled: true,
                        info: Some(info),
                    };
                    v.push(item);
                }
            }
        }

        set_backup(&reg_backup);
        Ok(v)
    }
}

pub fn list() -> Vec<MenuItem> {
    RegistryManager::get_all_menu_items().unwrap_or_default()
}

pub fn disable(id: &str) -> Result<(), anyhow::Error> {
    let backup = get_backup();
    if let Some(reg) = backup.iter().find(|i| i.path == id) {
        let _ = reg.delete();
    }
    Ok(())
}

pub fn enable(id: &str) -> Result<(), anyhow::Error> {
    let backup = get_backup();
    println!("{}", backup.len());
    if let Some(reg) = backup.iter().find(|i| i.path == id) {
        reg.write();
    }
    Ok(())
}
