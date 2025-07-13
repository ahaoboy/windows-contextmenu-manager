use std::collections::HashSet;
use std::sync::OnceLock;
use winreg::HKEY;
use winreg::RegKey;
use winreg::enums::*;

const REG_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Shell Extensions\Blocked";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockScope {
    User,
    Machine,
}

impl BlockScope {
    fn to_hive(self) -> HKEY {
        match self {
            BlockScope::User => HKEY_CURRENT_USER,
            BlockScope::Machine => HKEY_LOCAL_MACHINE,
        }
    }
}

pub struct Blocks {
    scope: BlockScope,
    items: HashSet<String>,
    is_readonly: OnceLock<bool>,
}

impl Blocks {
    pub fn new(scope: BlockScope) -> Self {
        Self {
            scope,
            items: HashSet::new(),
            is_readonly: OnceLock::new(),
        }
    }

    pub fn is_readonly(&self) -> bool {
        *self.is_readonly.get_or_init(|| {
            let hive = self.scope.to_hive();
            let base_key = RegKey::predef(hive);

            base_key.open_subkey_with_flags(REG_KEY, KEY_WRITE).is_err()
        })
    }

    pub fn load(&mut self) {
        let hive = self.scope.to_hive();
        let base_key = RegKey::predef(hive);

        if let Ok(sub_key) = base_key.open_subkey(REG_KEY) {
            self.items = sub_key
                .enum_values()
                .filter_map(|result| result.ok().map(|(name, _)| Self::from_reg_name(&name)))
                .collect();
        } else {
            self.items.clear();
        }
    }

    pub fn add(&mut self, id: &str) -> anyhow::Result<()> {
        if self.is_readonly() {
            anyhow::bail!(
                "Registry key is read-only. Try running as administrator for machine scope."
            );
        }

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

    pub fn scope(&self) -> BlockScope {
        self.scope
    }

    fn to_reg_name(val: &str) -> String {
        format!("{{{val}}}")
    }

    fn from_reg_name(val: &str) -> String {
        val.trim_matches('{').trim_matches('}').to_string()
    }

    pub fn user() -> Blocks {
        Blocks::new(BlockScope::User)
    }

    pub fn machine() -> Blocks {
        Blocks::new(BlockScope::Machine)
    }

    pub fn get_scope(scope: BlockScope) -> Blocks {
        match scope {
            BlockScope::User => Self::user(),
            BlockScope::Machine => Self::machine(),
        }
    }

    pub fn load_all() {
        Self::user().load();
        Self::machine().load();
    }
}

impl std::ops::Deref for Blocks {
    type Target = HashSet<String>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl std::ops::DerefMut for Blocks {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}


