use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Display;
use strum::IntoEnumIterator;
use strum_macros::Display;
use strum_macros::EnumIter;
use strum_macros::EnumString;
use tempfile::NamedTempFile;
use windows::Win32::System::Threading::CREATE_NO_WINDOW;
use winreg::HKEY;
use winreg::RegKey;
use winreg::RegValue;
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
    "removeproperties",
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct MenuItem {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub info: Option<MenuItemInfo>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct TypeItem {
    pub id: String,
    pub ty: String,
    pub clsid: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct MenuItemInfo {
    #[serde(with = "base64_option_vec")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<Vec<u8>>,
    pub publisher_display_name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<TypeItem>,
    pub install_path: String,
    pub family_name: String,
    pub full_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reg: Option<RegItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reg_txt: Option<String>,
}

use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Deserializer, Serializer};

pub mod base64_option_vec {
    use super::*;

    pub fn serialize<S>(value: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(vec) => {
                let encoded = general_purpose::STANDARD.encode(vec);
                serializer.serialize_some(&encoded)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        match opt {
            Some(s) => {
                let decoded = general_purpose::STANDARD
                    .decode(&s)
                    .map_err(serde::de::Error::custom)?;
                Ok(Some(decoded))
            }
            None => Ok(None),
        }
    }
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
        .creation_flags(CREATE_NO_WINDOW.0)
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum RegItemValue {
    SZ(String),
    DWORD(u32),
    ExpandSz(Vec<u8>),
    MultiSz(String),
    QWORD(u64),
    BINARY(Vec<u8>),
    None(Vec<u8>),
}

fn parse_dword(value: &RegValue) -> Option<u32> {
    if value.vtype == RegType::REG_DWORD && value.bytes.len() == 4 {
        let mut arr = [0u8; 4];
        arr.copy_from_slice(&value.bytes);
        Some(u32::from_le_bytes(arr))
    } else {
        None
    }
}
fn parse_qword(value: &RegValue) -> Option<u64> {
    if value.vtype == RegType::REG_QWORD && value.bytes.len() == 8 {
        let mut arr = [0u8; 8];
        arr.copy_from_slice(&value.bytes);
        Some(u64::from_le_bytes(arr))
    } else {
        None
    }
}
impl TryFrom<RegValue> for RegItemValue {
    type Error = anyhow::Error;

    fn try_from(value: RegValue) -> Result<Self, Self::Error> {
        let v = match value.vtype {
            REG_SZ => RegItemValue::SZ(value.to_string()),
            REG_EXPAND_SZ => RegItemValue::ExpandSz(value.bytes.to_vec()),
            REG_MULTI_SZ => RegItemValue::MultiSz(value.to_string()),
            REG_DWORD => RegItemValue::DWORD(parse_dword(&value).expect("parse_dword error")),
            REG_QWORD => RegItemValue::QWORD(parse_qword(&value).expect("parse_qword error")),
            REG_BINARY => RegItemValue::BINARY(value.bytes),
            REG_NONE => RegItemValue::None(value.bytes),
            REG_DWORD_BIG_ENDIAN => todo!(),
            REG_LINK => todo!(),
            REG_RESOURCE_LIST => todo!(),
            REG_FULL_RESOURCE_DESCRIPTOR => todo!(),
            REG_RESOURCE_REQUIREMENTS_LIST => todo!(),
        };
        Ok(v)
    }
}

impl RegItemValue {
    fn write(&self, key: &RegKey, name: &str) {
        let _ = match self {
            RegItemValue::SZ(v) => key.set_value(name, v),
            RegItemValue::DWORD(v) => key.set_value(name, v),
            RegItemValue::ExpandSz(bytes) => {
                let data = RegValue {
                    vtype: REG_EXPAND_SZ,
                    bytes: bytes.to_vec(),
                };
                key.set_raw_value(name, &data)
            }
            RegItemValue::MultiSz(v) => key.set_value(name, v),
            RegItemValue::QWORD(v) => key.set_value(name, v),
            RegItemValue::BINARY(bytes) => {
                let data = RegValue {
                    vtype: REG_BINARY,
                    bytes: bytes.to_vec(),
                };
                key.set_raw_value(name, &data)
            }
            RegItemValue::None(bytes) => {
                let data = RegValue {
                    vtype: REG_NONE,
                    bytes: bytes.to_vec(),
                };
                key.set_raw_value(name, &data)
            }
        };
    }
}

impl Display for RegItemValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            RegItemValue::SZ(v) => v.to_string(),
            RegItemValue::DWORD(v) => v.to_string(),
            RegItemValue::ExpandSz(v) => String::from_utf16(
                &v.chunks(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .take_while(|&u| u != 0)
                    .collect::<Vec<u16>>(),
            )
            .expect("ExpandSz format error"),
            RegItemValue::MultiSz(v) => v.to_string(),
            RegItemValue::QWORD(v) => v.to_string(),
            RegItemValue::None(_) => "REG_NONE".to_string(),
            RegItemValue::BINARY(bytes) => {
                let hex: Vec<_> = bytes.iter().map(|b| format!("{b:02x}")).collect();
                format!("0x{}", hex.join(""))
            }
        };
        f.write_str(&s)
    }
}

fn escape_str(s: &str) -> String {
    s.replace("\\", "\\\\").replace("\"", "\\\"")
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct RegItem {
    pub path: String,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub values: HashMap<String, RegItemValue>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<RegItem>,

    pub root: SceneRoot,
}

fn split_bytes(data: &[u8]) -> Vec<&[u8]> {
    let mut groups = Vec::new();

    if data.len() <= 23 {
        groups.push(data);
        return groups;
    }

    groups.push(&data[..23]);

    let mut i = 23;
    while i < data.len() {
        let end = usize::min(i + 25, data.len());
        groups.push(&data[i..end]);
        i += 25;
    }

    groups
}

fn to_hex_line(bytes: &[u8], hex_type: u32) -> String {
    let mut s = if hex_type == 0 {
        String::from("hex:")
    } else {
        format!("hex({hex_type}):")
    };

    let mut v = vec![];
    for i in split_bytes(bytes) {
        let hex: Vec<_> = i.iter().map(|b| format!("{b:02x}")).collect();
        v.push(hex.join(","));
    }
    s.push_str(&v.join(",\\\n  "));
    s
}

impl RegItem {
    pub fn get_child(&self, name: &str) -> Option<&RegItem> {
        self.children
            .iter()
            .find(|c| c.path.split('\\').next_back() == Some(name))
    }

    pub fn get_guid(&self) -> Option<String> {
        for i in ["CommandStateHandler", "DelegateExecute", "CLSID"] {
            if let Some(RegItemValue::SZ(value)) = self.values.get(i)
                && value.starts_with("{")
                && value.ends_with("}")
            {
                return Some(value[1..value.len() - 1].to_string());
            }
        }

        for i in [
            "command",
            "DropTarget",
            "SystemFileAssociations",
            "PropertySheetHandlers",
            "DragDropHandlers",
            "CopyHookHandlers",
        ] {
            if let Some(child) = self.get_child(i)
                && let Some(cid) = child.get_guid()
            {
                return Some(cid);
            }
        }

        let guid_re = regex::Regex::new(r"(?i)[A-F0-9]{8}(-[A-F0-9]{4}){3}-[A-F0-9]{12}").unwrap();

        for i in [
            self.path.clone(),
            self.values
                .get("")
                .map_or("".to_string(), |v| v.to_string()),
            self.values
                .get("CLSID")
                .map_or("".to_string(), |v| v.to_string()),
        ] {
            if let Some(cap) = guid_re.find_iter(&i).next() {
                return Some(cap.as_str().to_string());
            }
        }

        None
    }
    pub fn from_path(root: SceneRoot, path: &str) -> io::Result<RegItem> {
        let reg_key = RegKey::predef(root.get_reg()).open_subkey(path)?;

        let mut values = HashMap::new();
        for (name, value) in reg_key.enum_values().flatten() {
            if let Ok(item_value) = RegItemValue::try_from(value) {
                values.insert(name.clone(), item_value);
            }
        }

        let mut children: Vec<RegItem> = Vec::new();
        for subkey_name in reg_key.enum_keys().map(|x| x.unwrap()) {
            let subkey_path = format!("{path}\\{subkey_name}");
            let subkey_item = RegItem::from_path(root, &subkey_path)?;
            children.push(subkey_item);
        }

        Ok(RegItem {
            path: path.to_string(),
            values,
            children,
            root,
        })
    }

    fn is_safe(&self) -> bool {
        for scene_type in SceneType::iter() {
            for (_, reg_path) in scene_type.registry_path() {
                if self.path.starts_with(reg_path) {
                    return true;
                }
            }
        }
        false
    }

    pub fn write(&self) {
        if !self.is_safe() {
            return;
        }
        let root = RegKey::predef(self.root.get_reg());
        if let Ok((key, _disp)) = root.create_subkey(&self.path) {
            for (name, value) in &self.values {
                value.write(&key, name);
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
        let reg_key = RegKey::predef(self.root.get_reg())
            .open_subkey_with_flags(self.path.clone(), KEY_WRITE)?;
        for i in &self.children {
            let _ = i.delete();
        }
        reg_key.delete_subkey_with_flags("", KEY_WRITE)?;
        Ok(())
    }

    pub fn to_reg_txt(&self) -> String {
        fn write_item(item: &RegItem, out: &mut String) {
            out.push_str(&format!("\n[HKEY_CLASSES_ROOT\\{}]\n", item.path));
            for (name, value) in &item.values {
                let key = if name.is_empty() {
                    "@".to_string()
                } else {
                    format!(r#""{}""#, escape_str(name))
                };

                let v = match value {
                    RegItemValue::SZ(v) => format!(r#""{}""#, escape_str(v)),
                    RegItemValue::DWORD(v) => format!("dword:{v:08x}"),
                    RegItemValue::QWORD(v) => format!("qword:{v:016x}"),
                    RegItemValue::ExpandSz(v) => escape_str(&to_hex_line(v, 2)),
                    RegItemValue::MultiSz(v) => escape_str(v),
                    RegItemValue::BINARY(v) => to_hex_line(v, 0),
                    RegItemValue::None(v) => {
                        let hex = v
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<Vec<_>>()
                            .join(",");
                        format!("hex(0):{hex}")
                    }
                };
                let line = format!("{key}={v}\n");
                out.push_str(&line);
            }
            for child in &item.children {
                write_item(child, out);
            }
        }
        let mut out = String::from("Windows Registry Editor Version 5.00\n");
        write_item(self, &mut out);
        out
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
pub enum SceneRoot {
    #[default]
    /// HKEY_CLASSES_ROOT
    HKCR,
    /// HKEY_CURRENT_USER
    HKCU,
    /// HKEY_LOCAL_MACHINE
    HKLM,
}

impl SceneRoot {
    pub fn get_reg(&self) -> HKEY {
        match self {
            SceneRoot::HKCR => HKEY_CLASSES_ROOT,
            SceneRoot::HKCU => HKEY_CURRENT_USER,
            SceneRoot::HKLM => HKEY_LOCAL_MACHINE,
        }
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
pub(crate) enum SceneType {
    #[default]
    Shell,
    ShellEx,
    Edge,
    FileExts,
}

impl SceneType {
    pub fn registry_path(&self) -> &[(SceneRoot, &'static str)] {
        use SceneRoot::*;

        match self {
            SceneType::Shell => &[
                (HKCR, r"*\Shell"),
                (HKCR, r"Folder\shell"),
                (HKCR, r"Directory\Background\Shell"),
                (HKCR, r"Directory\Shell"),
                (HKCR, r"DesktopBackground\Shell"),
                (HKCR, r"Drive\Shell"),
                (HKCR, r"AllFilesystemObjects\Shell"),
                (HKCR, r"LibraryFolder\Shell"),
                (HKCR, r"UserLibraryFolder\Shell"),
                (HKCR, r"Launcher.ImmersiveApplication\Shell"),
                (HKCR, r"LibraryFolder\Background\Shell"),
                (HKCR, r"CLSID\{20D04FE0-3AEA-1069-A2D8-08002B30309D}\shell"), // Computer
                (HKCR, r"CLSID\{645FF040-5081-101B-9F08-00AA002F954E}\Shell"), // RecycleBin
                (HKCR, r"CLSID\{645FF040-5081-101B-9F08-00AA002F954E}\Shell"), // RecycleBin
            ],
            SceneType::ShellEx => &[
                (HKCR, r"*\ShellEx"),
                (HKCR, r"Folder\ShellEx"),
                (HKCR, r"Directory\Background\ShellEx"),
                (HKCR, r"Directory\ShellEx"),
                (HKCR, r"DesktopBackground\ShellEx"),
                (HKCR, r"Drive\ShellEx"),
                (HKCR, r"LibraryFolder\Background\ShellEx"),
                (HKCR, r"UserLibraryFolder\ShellEx"),
                (HKCR, r"Launcher.ImmersiveApplication\ShellEx"),
                (HKCR, r"LibraryFolder\ShellEx"),
                (
                    HKCU,
                    r"CLSID\{20D04FE0-3AEA-1069-A2D8-08002B30309D}\ShellEx",
                ), // Computer
                (
                    HKCU,
                    r"CLSID\{645FF040-5081-101B-9F08-00AA002F954E}\ShellEx",
                ), // RecycleBin
            ],
            SceneType::Edge => &[
                (HKCU, r"SOFTWARE\Policies\Microsoft\Edge"),
                (HKLM, r"SOFTWARE\Policies\Microsoft\Edge"),
            ],
            SceneType::FileExts => &[(
                HKCU,
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts",
            )],
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct GuidItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "ResText")]
    pub res_text: Option<String>,
    #[serde(rename = "Text")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "Icon")]
    pub icon: Option<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct GuidManager {
    pub items: HashMap<String, GuidItem>,
}

impl GuidManager {
    pub fn new() -> Self {
        let s = include_str!("../assets/guid.json");
        let items = serde_json::from_str::<HashMap<String, GuidItem>>(s).unwrap_or_default();
        GuidManager { items }
    }

    pub fn get_item(&self, guid: &str) -> Option<&GuidItem> {
        self.items.get(guid)
    }
}
use std::os::windows::process::CommandExt;

pub fn export_reg(reg_path: &str) -> io::Result<Vec<u8>> {
    let temp_file = NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    drop(temp_file);

    let temp_path = temp_path.to_string_lossy().to_string();

    let full_path = format!("HKEY_CLASSES_ROOT\\{reg_path}");

    let status = Command::new("reg")
        .args(["export", &full_path, &temp_path, "/y"])
        .creation_flags(CREATE_NO_WINDOW.0)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    if !status.success() {
        return Err(io::Error::other("reg export failed"));
    }
    std::fs::read(temp_path)
}
