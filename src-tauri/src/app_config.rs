pub const TARGET_NAME: &str = "Bronzebeard";
pub const MANIFEST_ASSET_NAME: &str = "addon-manifest.json";
pub const INSTALLER_ASSET_NAME: &str = "AscensionAddonInstaller-win-x64.zip";

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
}
