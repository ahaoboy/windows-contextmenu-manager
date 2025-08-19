use crate::APP_NAME;
use crate::BACKUP_NAME;
use crate::GuidManager;
use crate::MenuItem;
use crate::MenuItemInfo;
use crate::RegItem;
use crate::RegItemValue;
use crate::SceneRoot;
use crate::SceneType;
use cached::SizedCache;
use cached::proc_macro::cached;
use std::collections::HashSet;
use strum::IntoEnumIterator;
use windows::Win32::System::SystemInformation::GetWindowsDirectoryW;

fn get_backup() -> Vec<MenuItem> {
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

fn set_backup(items: &Vec<MenuItem>) {
    let mut old = get_backup();
    let old_keys: HashSet<String> = old.iter().map(|i| i.id.clone()).collect();

    let Some(d) = dirs::config_local_dir() else {
        return;
    };

    let app_dir = d.join(APP_NAME);
    if !std::fs::exists(&app_dir).unwrap_or(false) {
        let _ = std::fs::create_dir_all(&app_dir);
    }

    let backup_path = app_dir.join(BACKUP_NAME);

    for i in items {
        if !old_keys.contains(&i.id) {
            old.push(i.clone());
        }
    }
    if let Ok(s) = serde_json::to_string_pretty(&old) {
        let _ = std::fs::write(backup_path, &s);
    }
}

fn load_all() -> anyhow::Result<Vec<MenuItem>, anyhow::Error> {
    let mut v = vec![];
    let guid = GuidManager::new();
    for scene in SceneType::iter() {
        match scene {
            SceneType::Shell => {
                for (root, i) in scene.registry_path() {
                    let items = load_shell(*root, i, &guid).unwrap_or_default();
                    v.extend(items);
                }
            }
            SceneType::ShellEx => {
                for (root, i) in scene.registry_path() {
                    let items = load_shellex(*root, i, &guid).unwrap_or_default();
                    v.extend(items);
                }
            }
            SceneType::Edge => {
                for (root, i) in scene.registry_path() {
                    let items = load_edge(*root, i).unwrap_or_default();
                    v.extend(items);
                }
            }
        }
    }
    set_backup(&v);
    Ok(v)
}
#[cached]
fn get_system_directory() -> String {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::System::SystemInformation::GetSystemDirectoryW;

    let mut buffer = [0u16; 260];
    unsafe {
        let len = GetSystemDirectoryW(Some(buffer.as_mut_slice()));
        let path = OsString::from_wide(&buffer[..len as usize]);
        path.to_string_lossy().to_string() + "/"
    }
}
#[cached]
fn get_windows_directory() -> String {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    let mut buffer = [0u16; 260];
    unsafe {
        let len = GetWindowsDirectoryW(Some(buffer.as_mut_slice()));
        let path = OsString::from_wide(&buffer[..len as usize]);
        path.to_string_lossy().to_string()
    }
}
#[cached(
    ty = "SizedCache<String, Option<String>>",
    create = "{ SizedCache::with_size(100) }",
    convert = r#"{ format!("{}", s) }"#
)]
fn get_dll_txt(s: &str) -> Option<String> {
    let (dll, id) = parse_reg_path(s)?;
    exeico::get_dll_txt(dll, id).ok()
}
#[cached(
    ty = "SizedCache<String, Option<Vec<u8>>>",
    create = "{ SizedCache::with_size(100) }",
    convert = r#"{ format!("{}", s) }"#
)]
fn get_ico_from_str(s: &str) -> Option<Vec<u8>> {
    if s.is_empty() {
        return None;
    }
    if let Some((dll, id)) = parse_reg_path(s) {
        return exeico::get_dll_ico(dll, id).ok();
    };

    exeico::get_dll_icos(s).ok()?.first().cloned()
}

fn get_ico_from_reg(reg: &RegItem) -> Option<Vec<u8>> {
    if let Some(RegItemValue::SZ(icon)) = reg.values.get("Icon") {
        return get_ico_from_str(icon);
    }
    if let Some(RegItemValue::SZ(icon)) = reg.values.get("DefaultIcon") {
        return get_ico_from_str(icon);
    }

    if let Some(child) = reg.get_child("command")
        && let Some(RegItemValue::SZ(k)) = child.values.get("")
    {
        let exe = if let Some(index) = k.find(" ") {
            &k[..index]
        } else {
            k
        };
        if let Ok(exe_path) = which::which(exe) {
            return exeico::get_dll_icos(exe_path).ok()?.first().cloned();
        }

        return exeico::get_dll_icos(parse_path(exe)).ok()?.first().cloned();
    };

    // find first icon in children
    for child in &reg.children {
        if let Some(icon) = get_ico_from_reg(child) {
            return Some(icon);
        }
    }
    None
}
#[cached(
    ty = "SizedCache<String, String>",
    create = "{ SizedCache::with_size(100) }",
    convert = r#"{ format!("{}", path) }"#
)]
fn parse_path(path: &str) -> String {
    let path = path.to_lowercase();
    if path.starts_with("@%systemroot%") {
        path.replace("@%systemroot%", &get_windows_directory())
    } else if path.starts_with("%systemroot%") {
        path.replace("%systemroot%", &get_windows_directory())
    } else if path.starts_with('@') {
        path.replace("@", &(get_system_directory() + "/"))
    } else {
        get_system_directory() + "/" + &path
    }
}

fn parse_reg_path(s: &str) -> Option<(String, i32)> {
    let s = s.to_lowercase();
    let (path, id) = s.split_once(",")?;
    let id = id.parse().ok()?;
    let dll = parse_path(path);
    Some((dll, id))
}

fn get_shell_name(reg: &RegItem) -> String {
    let path_name = reg
        .path
        .split('\\')
        .next_back()
        .unwrap_or_default()
        .to_string();
    let muiverb = reg
        .values
        .get("MuiVerb")
        .or(reg.values.get("MUIVerb"))
        .or(reg.values.get(""))
        .and_then(|s| {
            if let RegItemValue::SZ(s) = s
                && s.starts_with('@')
                && s.contains(',')
            {
                get_dll_txt(s)
            } else {
                Some(s.to_string())
            }
        });
    if let Some(muiverb) = muiverb
        && !muiverb.trim().is_empty()
    {
        return muiverb;
    }

    if let Some(clsid) = reg.get_guid()
        && let Some(cls_name) = get_cls_name(&clsid)
    {
        return cls_name;
    }
    path_name
}

fn from_shell(reg: &RegItem, guid: &GuidManager) -> anyhow::Result<MenuItem> {
    if let Some(guid_key) = reg.get_guid()
        && let Some(item) = from_guid(guid_key.as_str(), reg, guid)
    {
        Ok(item)
    } else {
        let info = MenuItemInfo {
            icon: get_ico_from_reg(reg),
            publisher_display_name: String::new(),
            description: String::new(),
            types: vec![],
            install_path: String::new(),
            family_name: String::new(),
            full_name: String::new(),
            reg: Some(reg.clone()),
            reg_txt: Some(reg.to_reg_txt()),
        };
        let mut name = get_shell_name(reg);
        if is_clsid(&name)
            && let Some(cls_name) = get_cls_name(&name)
        {
            name = cls_name;
        }
        let menu = MenuItem {
            id: reg.path.clone(),
            name,
            enabled: true,
            info: Some(info),
        };

        Ok(menu)
    }
}
#[cached(
    ty = "SizedCache<String, Option<String>>",
    create = "{ SizedCache::with_size(100) }",
    convert = r#"{ format!("{}", name) }"#
)]
fn get_cls_name(name: &str) -> Option<String> {
    let name = if is_clsid(name) {
        name.to_string()
    } else {
        format!("{{{name}}}")
    };
    let reg = RegItem::from_path(SceneRoot::HKCR, &format!(r"CLSID\{name}")).ok()?;
    let RegItemValue::SZ(cls_name) = reg
        .values
        .get("LocalizedString")
        .or(reg.values.get(""))
        .cloned()?
    else {
        return None;
    };

    if cls_name.starts_with("@") || cls_name.starts_with('%') || cls_name.contains(",-") {
        return get_dll_txt(&cls_name);
    }
    Some(cls_name)
}

fn is_clsid(name: &str) -> bool {
    name.starts_with('{') && name.ends_with('}')
}

fn from_guid(key: &str, reg: &RegItem, guid: &GuidManager) -> Option<MenuItem> {
    if let Some(item) = guid.get_item(key.to_lowercase().as_str()) {
        let info = MenuItemInfo {
            icon: item.icon.clone().and_then(|s| get_ico_from_str(&s)),
            publisher_display_name: String::new(),
            description: String::new(),
            types: vec![],
            install_path: String::new(),
            family_name: String::new(),
            full_name: String::new(),
            reg: Some(reg.clone()),
            reg_txt: Some(reg.to_reg_txt()),
        };
        let mut name = item
            .res_text
            .clone()
            .and_then(|s| get_dll_txt(&s))
            .unwrap_or(item.text.clone().unwrap_or(get_shell_name(reg)));

        if is_clsid(&name)
            && let Some(cls_name) = get_cls_name(&name)
        {
            name = cls_name;
        }

        let menu = MenuItem {
            id: reg.path.clone(),
            name,
            enabled: true,
            info: Some(info),
        };
        return Some(menu);
    }
    None
}
fn from_shell_ex(reg: &RegItem, guid: &GuidManager) -> anyhow::Result<MenuItem> {
    if let Some(guid_key) = reg.get_guid()
        && let Some(item) = from_guid(guid_key.as_str(), reg, guid)
    {
        Ok(item)
    } else {
        let info = MenuItemInfo {
            icon: get_ico_from_reg(reg),
            publisher_display_name: String::new(),
            description: String::new(),
            types: vec![],
            install_path: String::new(),
            family_name: String::new(),
            full_name: String::new(),
            reg: Some(reg.clone()),
            reg_txt: Some(reg.to_reg_txt()),
        };
        let mut name = get_shell_name(reg);
        if is_clsid(&name)
            && let Some(cls_name) = get_cls_name(&name)
        {
            name = cls_name;
        }
        let menu = MenuItem {
            id: reg.path.clone(),
            name,
            enabled: true,
            info: Some(info),
        };
        Ok(menu)
    }
}

fn load_shell(root: SceneRoot, path: &str, guid: &GuidManager) -> anyhow::Result<Vec<MenuItem>> {
    let root = RegItem::from_path(root, path)?;
    let mut v = vec![];
    for i in root.children {
        if let Ok(menu) = from_shell(&i, guid) {
            v.push(menu);
        }
    }
    Ok(v)
}

fn load_edge(root: SceneRoot, path: &str) -> anyhow::Result<Vec<MenuItem>> {
    let root = RegItem::from_path(root, path)?;
    let mut v = vec![];
    let info = MenuItemInfo {
        reg: Some(root.clone()),
        reg_txt: Some(root.to_reg_txt()),
        ..Default::default()
    };
    let menu = MenuItem {
        id: path.to_string(),
        name: root.path,
        enabled: true,
        info: Some(info),
    };
    v.push(menu);
    Ok(v)
}

fn load_shellex(root: SceneRoot, path: &str, guid: &GuidManager) -> anyhow::Result<Vec<MenuItem>> {
    let root = RegItem::from_path(root, path)?;
    let mut v = vec![];
    for ex in [
        "ContextMenuHandlers",
        "DragDropHandlers",
        "CopyHookHandlers",
        "PropertySheetHandlers",
    ] {
        for i in root.get_child(ex).iter() {
            for reg in &i.children {
                if let Ok(menu) = from_shell_ex(reg, guid) {
                    v.push(menu);
                }
            }
        }
    }

    Ok(v)
}

pub fn list() -> Vec<MenuItem> {
    let v = load_all().unwrap_or_default();
    let mut backup = get_backup();

    for i in backup.iter_mut() {
        i.enabled = false;
    }

    for item in v {
        if let Some(i) = backup.iter_mut().find(|i| i.id == item.id) {
            i.enabled = true;
        }
    }

    backup
}

pub fn disable(id: &str) -> Result<(), anyhow::Error> {
    let backup = get_backup();
    if let Some(item) = backup.iter().find(|i| i.id == id)
        && let Some(info) = &item.info
        && let Some(reg) = &info.reg
    {
        let _ = reg.delete();
    }
    Ok(())
}

pub fn enable(id: &str) -> Result<(), anyhow::Error> {
    let backup = get_backup();
    if let Some(item) = backup.iter().find(|i| i.id == id)
        && let Some(info) = &item.info
        && let Some(reg) = &info.reg
    {
        reg.write();
    }
    Ok(())
}

#[cfg(test)]
mod test {
    #[test]
    fn test_get_dll_txt() {
        for i in [
            "@%systemroot%\\system32\\themecpl.dll,-10",
            "@shell32.dll,-51608",
            "@%SystemRoot%\\System32\\fvewiz.dll,-971",
            "@%SystemRoot%\\System32\\bdeunlock.exe,-100",
            "@%SystemRoot%\\system32\\WorkfoldersControl.dll,-1",
            "@%SystemRoot%\\system32\\cscui.dll,-7006",
            "@%SystemRoot%\\System32\\fvewiz.dll,-970",
            "@efscore.dll,-103",
            "@%SystemRoot%\\system32\\shell32.dll,-30309",
        ] {
            let txt = super::get_dll_txt(i);
            assert!(txt.is_some())
        }
    }

    #[test]
    fn test_get_dll_ico() {
        for i in [
            "%systemroot%\\system32\\themecpl.dll,-1",
            "edputil.dll,-1002",
            "C:\\Program Files\\Git\\git-bash.exe",
            "%SystemRoot%\\System32\\bdeunlock.exe",
        ] {
            let ico = super::get_ico_from_str(i);
            assert!(ico.is_some())
        }
    }
}
