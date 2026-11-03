//! Host paths must not assume that Android has a /bin directory.
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub fn executable(path: &Path) -> bool {
    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

pub fn posix_shell() -> PathBuf {
    shell_in(std::env::var_os("PREFIX").as_deref().map(Path::new))
}

fn shell_in(prefix: Option<&Path>) -> PathBuf {
    prefix
        .filter(|path| path.is_absolute())
        .map(|path| path.join("bin/sh"))
        .filter(|path| executable(path))
        .unwrap_or_else(|| PathBuf::from("/bin/sh"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn android_prefix_is_explicit_absolute_and_executable() {
        let root = std::env::temp_dir().join(format!("mobux-host-test-{}", std::process::id()));
        std::fs::create_dir_all(root.join("bin")).unwrap();
        let shell = root.join("bin/sh");
        std::fs::write(&shell, b"not executable").unwrap();
        std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(!executable(&shell));
        assert_eq!(shell_in(Some(&root)), PathBuf::from("/bin/sh"));
        std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(shell_in(Some(&root)), shell);
        assert_eq!(
            shell_in(Some(Path::new("relative"))),
            PathBuf::from("/bin/sh")
        );
        assert!(!executable(&root));
        std::fs::remove_dir_all(root).unwrap();
    }
}
