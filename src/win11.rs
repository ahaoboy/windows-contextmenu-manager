use std::path::PathBuf;

use serde_xml_rs::from_str;
use winreg::{RegKey, enums::HKEY_CLASSES_ROOT};

use crate::MenuItem;
use crate::blocks::{BlockScope, Blocks};

use windows::Management::Deployment::PackageManager;
use windows::core::HSTRING;

fn get_info(manifest_path: &PathBuf) -> Option<(String, String)> {
    let xml = std::fs::read_to_string(manifest_path).ok()?;
    let package = from_str::<serde_appxmanifest::Package>(&xml).ok()?;
    for app in package.applications.application {
        if let Some(ext) = app.extensions {
            if let Some(desktop_extension) = ext.desktop_extension {
                for i in desktop_extension {
                    if let Some(ty) = i
                        .file_explorer_context_menus
                        .item_type
                        .iter()
                        .find(|i| i.ty == "Directory" || i.ty == "*")
                    {
                        return Some((ty.verb.id.clone(), ty.verb.clsid.clone()));
                    }
                }
            }

            if let Some(com_extension) = ext.com_extension {
                for i in com_extension {
                    if let Some(ty) = i.com_server.and_then(|i| i.surrogate_server)
                        && let Some(i) = ty.com_class.first()
                    {
                        return Some((i.id.to_owned(), ty.display_name));
                    }
                }
            }
        }
    }
    None
}

pub fn list() -> Vec<MenuItem> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    let subkey = hkcr.open_subkey("PackagedCom\\Package").unwrap();
    let names: Vec<_> = subkey.enum_keys().flat_map(|x| x.ok()).collect();
    let package_manager = PackageManager::new().unwrap();

    let mut v = vec![];
    let scope = BlockScope::User;
    let mut blocks = Blocks::get_scope(scope);
    blocks.load();

    for full_name in names {
        if let Ok(pkg) = package_manager.FindPackageByPackageFullName(&HSTRING::from(&full_name)) {
            let is_bundle = pkg.IsBundle().unwrap_or(false);
            let manifest_name = if is_bundle {
                "AppxMetadata\\AppxBundleManifest.xml"
            } else {
                "AppxManifest.xml"
            };

            let install_path = std::path::PathBuf::from(pkg.InstalledPath().unwrap().to_string());
            let manifest_path = install_path.join(manifest_name);
            if let Some((name, id)) = get_info(&manifest_path) {
                let icon = pkg
                    .Logo()
                    .ok()
                    .and_then(|logo| logo.RawUri().ok())
                    .and_then(|p| {
                        std::fs::read(p.to_string()).ok()
                    });
                v.push(MenuItem {
                    enabled: !blocks.contains(&id),
                    id,
                    name,
                    icon,
                });
            }
        }
    }

    v
}

pub fn enable(id: &str, scope: BlockScope) -> Result<(), anyhow::Error> {
    let mut blocks = Blocks::get_scope(scope);
    blocks.load();
    blocks.remove(id)
}

pub fn disable(id: &str, scope: BlockScope) -> Result<(), anyhow::Error> {
    let mut blocks = Blocks::get_scope(scope);
    blocks.load();
    blocks.add(id)
}
