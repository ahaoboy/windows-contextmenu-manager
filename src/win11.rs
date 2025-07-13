use crate::{MenuItem, MenuItemInfo};
use crate::{Scope, TypeItem};
use serde_xml_rs::from_str;
use std::collections::HashSet;
use std::path::PathBuf;
use windows::Management::Deployment::PackageManager;
use windows::core::HSTRING;
use winreg::enums::*;
use winreg::{RegKey, enums::HKEY_CLASSES_ROOT};

const REG_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Shell Extensions\Blocked";

pub struct Blocks {
    pub scope: Scope,
    pub items: HashSet<String>,
}

impl Blocks {
    pub fn new(scope: Scope) -> Self {
        let hive = scope.to_hive();
        let base_key = RegKey::predef(hive);

        let items = if let Ok(sub_key) = base_key.open_subkey(REG_KEY) {
            sub_key
                .enum_values()
                .filter_map(|result| result.ok().map(|(name, _)| Self::from_reg_name(&name)))
                .collect()
        } else {
            HashSet::new()
        };

        Self { scope, items }
    }

    pub fn add(&mut self, id: &str) -> anyhow::Result<()> {
        let hive = self.scope.to_hive();
        let base_key = RegKey::predef(hive);

        let sub_key = base_key.create_subkey(REG_KEY)?.0;
        sub_key.set_value(Self::to_reg_name(id), &"")?;
        self.items.insert(id.to_string());

        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> anyhow::Result<()> {
        if !self.items.contains(id) {
            return Ok(());
        }

        let hive = self.scope.to_hive();
        let base_key = RegKey::predef(hive);

        if let Ok(sub_key) = base_key.open_subkey_with_flags(REG_KEY, KEY_WRITE) {
            let _ = sub_key.delete_value(Self::to_reg_name(id));
            self.items.remove(id);
        } else {
            self.items.clear();
        }

        Ok(())
    }

    pub fn contains(&self, id: &str) -> bool {
        self.items.contains(id)
    }

    fn to_reg_name(val: &str) -> String {
        format!("{{{val}}}")
    }

    fn from_reg_name(val: &str) -> String {
        val.trim_matches('{').trim_matches('}').to_string()
    }
}

struct Ext {
    id: String,
    display_name: String,
    publisher_display_name: String,
    description: String,
    types: Vec<TypeItem>,
}

fn get_info(manifest_path: &PathBuf) -> Option<Ext> {
    let xml = std::fs::read_to_string(manifest_path).ok()?;
    let package = from_str::<serde_appxmanifest::Package>(&xml).ok()?;
    let display_name = package.properties.display_name;
    let publisher_display_name = package.properties.publisher_display_name;
    for app in package.applications.application {
        if let Some(ext) = app.extensions {
            let description = app.visual_elements.description;

            if let Some(desktop_extension) = ext.desktop_extension {
                let types = desktop_extension
                    .iter()
                    .flat_map(|i| {
                        i.file_explorer_context_menus
                            .item_type
                            .iter()
                            .map(|v| TypeItem {
                                ty: v.ty.clone(),
                                id: v.verb.id.clone(),
                                clsid: v.verb.clsid.clone(),
                            })
                    })
                    .collect::<Vec<_>>();

                for i in desktop_extension {
                    if let Some(ty) = i
                        .file_explorer_context_menus
                        .item_type
                        .iter()
                        .find(|i| i.ty == "Directory" || i.ty == "*")
                    {
                        return Some(Ext {
                            id: ty.verb.clsid.clone(),
                            display_name,
                            publisher_display_name,
                            description,
                            types,
                        });
                    }
                }
            }

            if let Some(com_extension) = ext.com_extension {
                for i in com_extension {
                    if let Some(ty) = i.com_server.and_then(|i| i.surrogate_server)
                        && let Some(i) = ty.com_class.first()
                    {
                        return Some(Ext {
                            id: i.id.clone(),
                            display_name: ty.display_name.clone(),
                            publisher_display_name,
                            description,
                            types: vec![],
                        });
                    }
                }
            }
        }
    }
    None
}

pub fn list(scope: Scope) -> Vec<MenuItem> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    let subkey = hkcr.open_subkey("PackagedCom\\Package").unwrap();
    let names: Vec<_> = subkey.enum_keys().flat_map(|x| x.ok()).collect();
    let package_manager = PackageManager::new().unwrap();

    let mut v = vec![];
    let blocks = Blocks::new(scope);

    for full_name in names {
        if let Ok(pkg) = package_manager.FindPackageByPackageFullName(&HSTRING::from(&full_name)) {
            let is_bundle = pkg.IsBundle().unwrap_or(false);
            let manifest_name = if is_bundle {
                "AppxMetadata\\AppxBundleManifest.xml"
            } else {
                "AppxManifest.xml"
            };

            let family_name = pkg
                .Id()
                .and_then(|i| i.FamilyName())
                .map(|i| i.to_string())
                .unwrap_or_default();
            let full_name = pkg
                .Id()
                .and_then(|i| i.FullName())
                .map(|i| i.to_string())
                .unwrap_or_default();
            let display_name = pkg.DisplayName().map(|i| i.to_string()).unwrap_or_default();

            let install_path = std::path::PathBuf::from(pkg.InstalledPath().unwrap().to_string());
            let manifest_path = install_path.join(manifest_name);
            if let Some(Ext {
                id,
                display_name,
                publisher_display_name,
                description,
                types,
            }) = get_info(&manifest_path)
            {
                let icon = pkg
                    .Logo()
                    .ok()
                    .and_then(|logo| logo.RawUri().ok())
                    .and_then(|p| std::fs::read(p.to_string()).ok());

                let mut visit: HashSet<String> = HashSet::new();
                for ty in types.clone() {
                    if visit.contains(&ty.clsid) {
                        continue;
                    }
                    visit.insert(ty.clsid.clone());
                    let info = Some(MenuItemInfo {
                        icon: icon.clone(),
                        publisher_display_name: publisher_display_name.clone(),
                        description: description.clone(),
                        types: types
                            .iter()
                            .filter(|i| i.clsid == ty.clsid)
                            .cloned()
                            .collect(),
                        install_path: install_path.to_string_lossy().to_string(),
                        family_name: family_name.clone(),
                        full_name: family_name.clone(),
                    });

                    v.push(MenuItem {
                        enabled: !blocks.contains(&ty.clsid),
                        id: ty.clsid.clone(),
                        name: display_name.clone(),
                        info,
                    });
                }
            }
        }
    }

    v
}

pub fn enable(id: &str, scope: Scope) -> Result<(), anyhow::Error> {
    let mut blocks = Blocks::new(scope);
    blocks.remove(id)
}

pub fn disable(id: &str, scope: Scope) -> Result<(), anyhow::Error> {
    let mut blocks = Blocks::new(scope);
    blocks.add(id)
}
