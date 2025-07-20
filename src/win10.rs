use crate::APP_NAME;
use crate::BACKUP_NAME;
use crate::MenuItem;
use crate::MenuItemInfo;
use crate::RegItem;
use crate::SceneType;
use std::collections::HashSet;
use strum::IntoEnumIterator;
use windows::Win32::System::SystemInformation::GetWindowsDirectoryW;

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
    // let mut backup = vec![];

    for scene in SceneType::iter() {
        match scene {
            SceneType::Shell => {
                for i in scene.registry_path() {
                    let items = load_shells(i).unwrap_or_default();
                    v.extend(items);
                }
            }
            SceneType::ShellEx => load_shellex(),
        }
    }

    // for secne in Scene::iter() {
    //     for scene_path in secne.registry_path() {
    //         let Ok(reg) = RegItem::from_path(scene_path.0) else {
    //             continue;
    //         };
    //         for item in reg.children {
    //             let info = MenuItemInfo {
    //                 icon: item.values.get("Icon").and_then(|v| get_icon(v)),
    //                 publisher_display_name: String::new(),
    //                 description: String::new(),
    //                 types: vec![],
    //                 install_path: String::new(),
    //                 family_name: String::new(),
    //                 full_name: String::new(),
    //                 reg: Some(item.clone()),
    //             };
    //             let mut name = item
    //                 .path
    //                 .split('\\')
    //                 .next_back()
    //                 .unwrap_or_default()
    //                 .to_string();

    //             // TODO: add these to unknown
    //             if WIN10_SKIP_REGKEY.iter().any(|skip| name.ends_with(skip)) {
    //                 continue;
    //             }
    //             if let Some(handle_name) = get_handle_name(&item) {
    //                 name = handle_name
    //             }

    //             if let Some(child) = item.children.iter().find(|c| c.path.ends_with("\\command"))
    //                 && let Some(child_name) = get_handle_name(child)
    //             {
    //                 name = child_name;
    //             }

    //             if let Some(s) = item
    //                 .values
    //                 .get("MuiVerb")
    //                 .or(item.values.get("MUIVerb"))
    //                 .or(item.values.get(""))
    //             {
    //                 // TODO: ignore "": "@shell32.dll,-8506"
    //                 // "MuiVerb": "@appresolver.dll,-8501"
    //                 if !s.contains(",") {
    //                     name = s.clone();
    //                 }
    //                 if s.starts_with("@")
    //                     && s.contains(",")
    //                     && let Some(load_str) = load_indirect_string(s)
    //                 {
    //                     name = load_str;
    //                 }
    //             }

    //             if name.starts_with("{") && name.ends_with("}") {
    //                 // TODO: add to unknown
    //                 continue;
    //             }
    //             let item = MenuItem {
    //                 id: item.path.clone(),
    //                 name,
    //                 enabled: true,
    //                 info: Some(info),
    //             };
    //             backup.push(item.clone());
    //             v.push(item);
    //         }
    //     }
    // }

    println!("load_all: {}", v.len());
    // set_backup(&v);
    Ok(v)
}

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

fn load_shell() {}
fn load_shellex() {}

fn get_dll_txt(s: &str) -> Option<String> {
    let (dll, id) = parse_reg_path(s)?;
    exeico::get_dll_txt(dll, id).ok()
}

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
    // println!("get_ico_from_reg {}", reg.path);
    if let Some(icon) = reg.values.get("Icon") {
        return get_ico_from_str(icon);
    }
    if let Some(icon) = reg.values.get("DefaultIcon") {
        return get_ico_from_str(icon);
    }

    if let Some(child) = reg.get_child("command")
        && let Some(k) = child.values.get("")
        && let Some((exe, _)) = k.split_once(' ')
    {
        if let Ok(exe_path) = which::which(exe) {
            return exeico::get_exe_ico(exe_path).ok();
        }

        // println!("parse_path(exe) {}", parse_path(exe));
        return exeico::get_exe_ico(parse_path(exe)).ok();
    };

    // find first icon in children
    for child in &reg.children {
        if let Some(icon) = get_ico_from_reg(child) {
            return Some(icon);
        }
    }
    None
}

fn parse_path(path: &str) -> String {
    let path = path.to_lowercase();
    let dll = if path.starts_with("@%systemroot%") {
        path.replace("@%systemroot%", &get_windows_directory())
    } else if path.starts_with("%systemroot%") {
        path.replace("%systemroot%", &get_windows_directory())
    } else if path.starts_with('@') {
        path.replace("@", &(get_system_directory() + "/"))
    } else {
        get_system_directory() + "/" + &path
    };
    dll
}

fn parse_reg_path(s: &str) -> Option<(String, i32)> {
    let s = s.to_lowercase();
    let Some((path, id)) = s.split_once(",") else {
        return None;
    };
    let id = id.parse().ok()?;
    let dll = parse_path(path);
    // println!("dll: {dll}, id: {id}");
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
            if s.starts_with('@') && s.contains(',') {
                get_dll_txt(s)
            } else {
                Some(s.to_string())
            }
        });
    muiverb.unwrap_or(path_name)
}

fn from_shell(reg: &RegItem) -> anyhow::Result<MenuItem> {
    let info = MenuItemInfo {
        icon: get_ico_from_reg(reg),
        publisher_display_name: String::new(),
        description: String::new(),
        types: vec![],
        install_path: String::new(),
        family_name: String::new(),
        full_name: String::new(),
        reg: Some(reg.clone()),
    };
    let name = get_shell_name(reg);
    let menu = MenuItem {
        id: reg.path.clone(),
        name,
        enabled: true,
        info: Some(info),
    };

    Ok(menu)
}

fn load_shells(path: &str) -> anyhow::Result<Vec<MenuItem>> {
    let root = RegItem::from_path(path)?;
    let mut v = vec![];
    for i in root.children {
        // FIXME: skip no title item
        // let set: HashSet<_> = i.values.keys().collect();
        // if !["", "MuiVerb", "MUIVerb"]
        //     .iter()
        //     .any(|k| set.contains(&k.to_string()))
        // {
        //     continue;
        // }

        if let Ok(menu) = from_shell(&i) {
            v.push(menu);
        }
        println!("{}", v.len());
    }
    Ok(v)
}

pub fn list() -> Vec<MenuItem> {
    let v = load_all().unwrap_or_default();
    // let mut backup = get_backup();

    // for i in backup.iter_mut() {
    //     i.enabled = false;
    // }

    // for item in v {
    //     if let Some(i) = backup.iter_mut().find(|i| i.id == item.id) {
    //         i.enabled = true;
    //     }
    // }

    v
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
