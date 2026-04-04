pub const TARGET_NAME: &str = "Bronzebeard";
pub const COA_TARGET_NAME: &str = "CoA";
pub const SUPPORTED_TARGETS: [&str; 2] = [TARGET_NAME, COA_TARGET_NAME];
pub const MANIFEST_ASSET_NAME: &str = "addon-manifest.json";
pub const INSTALLER_ASSET_NAME: &str = "AscensionAddonInstaller-win-x64.zip";

pub fn is_supported_target(target: &str) -> bool {
    SUPPORTED_TARGETS
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(target))
}

pub fn contains_target(values: &[String], target: &str) -> bool {
    values
        .iter()
        .any(|value| value.eq_ignore_ascii_case(target))
}

pub fn resolve_target_name(selected_target: Option<&str>, path_hints: &[Option<&str>]) -> String {
    if path_hints
        .iter()
        .flatten()
        .any(|hint| infer_target_name_from_path_hint(hint).is_some_and(|target| target == COA_TARGET_NAME))
    {
        COA_TARGET_NAME.to_string()
    } else if let Some(target) = selected_target.filter(|target| is_supported_target(target)) {
        canonical_target_name(target)
            .unwrap_or(TARGET_NAME)
            .to_string()
    } else {
        TARGET_NAME.to_string()
    }
}

pub fn infer_target_name_from_path_hint(path: &str) -> Option<&'static str> {
    if path_looks_like_coa(path) {
        Some(COA_TARGET_NAME)
    } else if path_looks_like_live(path) {
        Some(TARGET_NAME)
    } else {
        None
    }
}

fn canonical_target_name(target: &str) -> Option<&'static str> {
    if target.eq_ignore_ascii_case(TARGET_NAME) {
        Some(TARGET_NAME)
    } else if target.eq_ignore_ascii_case(COA_TARGET_NAME) {
        Some(COA_TARGET_NAME)
    } else {
        None
    }
}

fn path_looks_like_coa(path: &str) -> bool {
    let normalized = path.replace('/', "\\").to_ascii_lowercase();
    normalized.contains("ascension ptr")
        || normalized.contains("ascension_ptr")
        || normalized.contains("\\ptr\\")
        || normalized.ends_with("\\ptr")
        || normalized.contains("\\coa\\")
        || normalized.ends_with("\\coa")
        || normalized.contains("\\rexxar\\")
        || normalized.ends_with("\\rexxar")
        || normalized.contains("conquest of azeroth")
}

fn path_looks_like_live(path: &str) -> bool {
    let normalized = path.replace('/', "\\").to_ascii_lowercase();
    if path_looks_like_coa(&normalized) {
        return false;
    }

    normalized.contains("\\resources\\client\\")
        || normalized.ends_with("\\interface\\addons")
        || normalized.contains("\\interface\\addons\\")
}

pub fn installer_repo_owner() -> String {
    option_env!("ASCENSIONUP_REPO_OWNER")
        .unwrap_or("dmedlin87")
        .to_string()
}

pub fn installer_repo_name() -> String {
    option_env!("ASCENSIONUP_REPO_NAME")
        .unwrap_or("AscensionUp")
        .to_string()
}

pub fn catalog_url() -> String {
    option_env!("ASCENSIONUP_CATALOG_URL")
        .map(str::to_string)
        .unwrap_or_else(|| {
            format!(
                "https://raw.githubusercontent.com/{}/{}/main/catalog.json",
                installer_repo_owner(),
                installer_repo_name()
            )
        })
}

pub fn installer_release_page_url() -> String {
    format!(
        "https://github.com/{}/{}/releases/latest",
        installer_repo_owner(),
        installer_repo_name()
    )
}

pub fn installer_download_url() -> String {
    format!(
        "https://github.com/{}/{}/releases/latest/download/{}",
        installer_repo_owner(),
        installer_repo_name(),
        INSTALLER_ASSET_NAME
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_BRONZEBEARD_PATH: &str = r"C:\Games\Ascension";
    const TEST_COA_PTR_PATH: &str = r"C:\Program Files\Ascension PTR";
    const TEST_COA_LAUNCHER_PTR_PATH: &str = r"C:\Program Files\Ascension Launcher\resources\ascension_ptr";

    #[test]
    fn test_installer_repo_owner_default() {
        assert_eq!(installer_repo_owner(), "dmedlin87");
    }

    #[test]
    fn test_installer_repo_name_default() {
        assert_eq!(installer_repo_name(), "AscensionUp");
    }

    #[test]
    fn test_catalog_url_default() {
        assert_eq!(
            catalog_url(),
            "https://raw.githubusercontent.com/dmedlin87/AscensionUp/main/catalog.json"
        );
    }

    #[test]
    fn test_installer_release_page_url() {
        assert_eq!(
            installer_release_page_url(),
            "https://github.com/dmedlin87/AscensionUp/releases/latest"
        );
    }

    #[test]
    fn test_installer_download_url() {
        assert_eq!(
            installer_download_url(),
            "https://github.com/dmedlin87/AscensionUp/releases/latest/download/AscensionAddonInstaller-win-x64.zip"
        );
    }
    #[test]
    fn generates_default_catalog_url() {
        let url = catalog_url();
        assert_eq!(
            url,
            "https://raw.githubusercontent.com/dmedlin87/AscensionUp/main/catalog.json"
        );
    }

    #[test]
    fn resolves_coa_target_from_ptr_path() {
        let target = resolve_target_name(
            Some(TARGET_NAME),
            &[Some(TEST_COA_PTR_PATH), None],
        );

        assert_eq!(target, COA_TARGET_NAME);
    }

    #[test]
    fn resolves_coa_target_from_ascension_ptr_path() {
        let target = resolve_target_name(
            Some(TARGET_NAME),
            &[Some(TEST_COA_LAUNCHER_PTR_PATH), None],
        );

        assert_eq!(target, COA_TARGET_NAME);
    }

    #[test]
    fn preserves_supported_target_when_no_coa_hint_exists() {
        let target = resolve_target_name(Some(COA_TARGET_NAME), &[Some(TEST_BRONZEBEARD_PATH)]);

        assert_eq!(target, COA_TARGET_NAME);
    }
}
