//! Audited recipes for the host's package manager. No remote scripts or shell interpolation.
use crate::catalog::CatalogItem;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub title: String,
    pub package: String,
}

impl Plan {
    pub fn for_app(app: &CatalogItem, termux: bool) -> Option<Self> {
        let package = app.termux_package.as_ref()?;
        if !termux
            || package.is_empty()
            || !package
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            || package.starts_with('-')
        {
            return None;
        }
        Some(Self {
            title: app.label.clone(),
            package: package.clone(),
        })
    }

    pub fn args(&self) -> Vec<String> {
        vec!["install".into(), self.package.clone()]
    }

    pub fn command(&self) -> String {
        format!("pkg install {}", self.package)
    }
}

pub fn is_termux() -> bool {
    std::env::var_os("TERMUX_VERSION").is_some_and(|version| !version.is_empty())
        && crate::state::on_path("pkg")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipes_are_host_specific_and_do_not_guess_android_support() {
        let gh = crate::catalog::by_bin("gh").unwrap();
        let plan = Plan::for_app(gh, true).unwrap();
        assert_eq!(plan.command(), "pkg install gh");
        assert_eq!(plan.args(), ["install", "gh"]);
        assert!(Plan::for_app(gh, false).is_none());
        assert!(Plan::for_app(crate::catalog::by_bin("herdr").unwrap(), true).is_none());
        assert!(Plan::for_app(crate::catalog::by_bin("hunk").unwrap(), true).is_none());
        assert_eq!(
            Plan::for_app(crate::catalog::by_bin("micro").unwrap(), true)
                .unwrap()
                .args(),
            ["install", "micro"]
        );
    }

    #[test]
    fn package_names_cannot_be_options_or_shell_commands() {
        for package in ["", "--help", "gh; id", "gh other", "$(id)", "gh\n"] {
            let mut app = crate::catalog::by_bin("gh").unwrap().clone();
            app.termux_package = Some(package.to_string());
            assert!(Plan::for_app(&app, true).is_none());
        }
    }
}
