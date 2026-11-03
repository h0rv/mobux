//! Bundled, curated metadata. This is not a package installer or remote code feed.
use std::sync::LazyLock;

use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppKind {
    #[default]
    App,
    Agent,
    AgentManager,
    Editor,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogItem {
    pub id: String,
    pub label: String,
    pub bin: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub blurb: String,
    #[serde(default)]
    pub kind: AppKind,
    pub homepage: String,
    pub icon: String,
    pub mobile_notes: String,
    #[serde(default)]
    pub termux_package: Option<String>,
    #[serde(default)]
    pub requires: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    version: u8,
    apps: Vec<CatalogItem>,
}

pub static CATALOG: LazyLock<Vec<CatalogItem>> = LazyLock::new(|| {
    let catalog: Catalog = serde_json::from_str(include_str!("../catalog/apps.json"))
        .expect("bundled app catalog must be valid JSON");
    assert_eq!(catalog.version, 1, "unsupported bundled catalog version");
    catalog.apps
});

pub fn by_bin(bin: &str) -> Option<&'static CatalogItem> {
    CATALOG.iter().find(|app| app.bin == bin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn bundled_catalog_has_unique_ids_commands_and_explicit_review_notes() {
        let mut ids = HashSet::new();
        let mut bins = HashSet::new();
        assert!(!CATALOG.is_empty());
        for app in CATALOG.iter() {
            assert!(ids.insert(&app.id), "duplicate id {}", app.id);
            assert!(bins.insert(&app.bin), "duplicate command {}", app.bin);
            assert!(!app.bin.is_empty() && !app.bin.chars().any(char::is_whitespace));
            assert!(!app.label.is_empty() && !app.blurb.is_empty() && !app.icon.is_empty());
            assert!(app.homepage.starts_with("https://"));
            assert!(!app.mobile_notes.is_empty());
            assert!(
                crate::icons::Glyph::from_catalog(&app.icon).is_some(),
                "unknown icon {}",
                app.icon
            );
        }
        assert_eq!(by_bin("opencode").unwrap().label, "OpenCode");
        assert_eq!(by_bin("hunk").unwrap().args, ["diff"]);
        assert!(by_bin("herdr").is_some());
        assert!(by_bin("nvim").is_some());
        assert_eq!(by_bin("gh-dash").unwrap().requires, ["gh"]);
        for app in CATALOG.iter() {
            for dependency in &app.requires {
                assert!(
                    by_bin(dependency).is_some(),
                    "unknown dependency {dependency}"
                );
                assert_ne!(dependency, &app.bin);
            }
        }
    }
}
