//! Shared filesystem boundaries. Caller scopes must come from backend rules/results,
//! never from an unchecked frontend path. Fail closed on unreadable directories.
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;
use walkdir::WalkDir;

pub fn existing_path(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err("路径必须是绝对路径，且不能包含 ..".into());
    }
    let mut prefix = PathBuf::new();
    for part in path.components() {
        prefix.push(part);
        let metadata = fs::symlink_metadata(&prefix).map_err(|e| format!("路径无法访问：{e}"))?;
        // macOS itself aliases /var and /tmp to /private. All user-controlled
        // symlinks, including an ancestor of the requested target, are rejected.
        if metadata.is_symlink() && prefix != Path::new("/var") && prefix != Path::new("/tmp") {
            return Err("不能通过符号链接访问或删除文件".into());
        }
    }
    path.canonicalize()
        .map_err(|e| format!("路径无法访问：{e}"))
}

fn protected_roots() -> &'static [PathBuf] {
    static ROOTS: OnceLock<Vec<PathBuf>> = OnceLock::new();
    ROOTS.get_or_init(|| {
        let mut roots: Vec<PathBuf> = [
            "/System",
            "/usr",
            "/bin",
            "/sbin",
            "/etc",
            "/private/etc",
            "/private/var/db",
            "/Library/Keychains",
            "/Library/LaunchAgents",
            "/Library/LaunchDaemons",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect();
        if let Some(home) = dirs::home_dir() {
            roots.extend(
                ["Library/Keychains", "Library/Mail", ".ssh", ".gnupg"]
                    .into_iter()
                    .map(|name| home.join(name)),
            );
        }
        if let Ok(executable) = std::env::current_exe() {
            if let Some(bundle) = executable
                .ancestors()
                .find(|p| p.extension().is_some_and(|ext| ext == "app"))
            {
                roots.push(bundle.to_path_buf());
            } else {
                roots.push(executable);
            }
        }
        roots
    })
}

pub fn readable(path: &Path) -> bool {
    !protected_roots().iter().any(|root| path.starts_with(root))
}

pub fn validate_child(root: &Path, target: &Path) -> Result<PathBuf, String> {
    let root = existing_path(root)?;
    let target = existing_path(target)?;
    if target == root || !target.starts_with(&root) {
        return Err("只能处理后端允许范围内的子项，不能删除范围根目录".into());
    }
    validate_delete(&target)?;
    Ok(target)
}

pub fn validate_delete(path: &Path) -> Result<PathBuf, String> {
    let target = existing_path(path)?;
    let metadata = fs::symlink_metadata(&target).map_err(|e| e.to_string())?;
    if !metadata.is_file() && !metadata.is_dir() {
        return Err("只能处理普通文件和文件夹".into());
    }
    if target == Path::new("/")
        || protected_roots()
            .iter()
            .any(|root| target.starts_with(root) || root.starts_with(&target))
    {
        return Err("此路径包含受保护的系统、归档或用户数据，不能删除".into());
    }
    if let Some(home) = dirs::home_dir() {
        for protected in [
            home.join("Library/Group Containers"),
            home.join("Library/Developer/Xcode/Archives"),
        ] {
            if target.starts_with(&protected) || protected.starts_with(&target) {
                return Err("发布归档和共享容器数据必须手动管理，不能删除".into());
            }
        }
        let roots = [
            home.clone(),
            home.join("Applications"),
            home.join("Downloads"),
            home.join("Documents"),
            home.join("Desktop"),
            home.join("Pictures"),
            home.join("Music"),
            home.join("Library"),
            home.join("Library/Caches"),
            home.join("Library/Logs"),
            home.join("Library/Preferences"),
            home.join("Library/Application Support"),
            home.join("Library/Containers"),
            home.join("Library/Saved Application State"),
            home.join(".Trash"),
        ];
        if roots.contains(&target) {
            return Err("不能删除用户目录或数据分类根目录".into());
        }
    }
    if [
        "/Applications",
        "/Users",
        "/Library",
        "/Library/Caches",
        "/Library/Logs",
        "/Volumes",
        "/private",
        "/private/var",
    ]
    .iter()
    .any(|root| target == Path::new(root))
    {
        return Err("不能删除系统或应用分类根目录".into());
    }
    Ok(target)
}

/// Moving a directory also moves excluded descendants. Check the entire tree;
/// nested symlinks are safe to move/unlink (never followed), but exclusions still apply.
pub fn validate_tree(path: &Path, excluded: impl Fn(&Path) -> bool) -> Result<PathBuf, String> {
    let target = validate_delete(path)?;
    #[cfg(unix)]
    let device = {
        use std::os::unix::fs::MetadataExt;
        fs::symlink_metadata(&target)
            .map_err(|e| e.to_string())?
            .dev()
    };
    for entry in WalkDir::new(&target).follow_links(false) {
        let entry = entry.map_err(|e| format!("无法完整检查目录，未执行删除：{e}"))?;
        #[cfg(unix)]
        if !entry.file_type().is_symlink() {
            use std::os::unix::fs::MetadataExt;
            if entry.metadata().map_err(|e| e.to_string())?.dev() != device {
                return Err("目录包含其他挂载卷，不能整体删除".into());
            }
        }
        if !readable(entry.path()) || excluded(entry.path()) {
            return Err(format!(
                "目录包含受保护或排除的项目：{}",
                entry.path().display()
            ));
        }
    }
    Ok(target)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileIdentity {
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    is_dir: bool,
}

impl FileIdentity {
    pub fn capture(path: &Path) -> Result<Self, String> {
        let path = existing_path(path)?;
        let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        Ok(Self {
            #[cfg(unix)]
            device: {
                use std::os::unix::fs::MetadataExt;
                metadata.dev()
            },
            #[cfg(unix)]
            inode: {
                use std::os::unix::fs::MetadataExt;
                metadata.ino()
            },
            is_dir: metadata.is_dir(),
        })
    }
    pub fn verify(&self, path: &Path) -> Result<(), String> {
        if *self != Self::capture(path)? {
            return Err("文件在扫描后已被替换，请重新扫描".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("cleanmac-safety-{name}-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        root.canonicalize().unwrap()
    }
    #[test]
    fn rejects_traversal_scope_roots_and_similar_prefixes() {
        let root = fixture("scope");
        let outside = fixture("scope-other");
        fs::write(root.join("item"), "keep").unwrap();
        fs::write(outside.join("item"), "keep").unwrap();
        assert!(validate_child(&root, &root.join("item")).is_ok());
        assert!(validate_child(&root, &root).is_err());
        assert!(validate_child(&root, &outside.join("item")).is_err());
        assert!(existing_path(&root.join("../scope/item")).is_err());
        assert!(existing_path(Path::new("relative")).is_err());
        assert!(validate_delete(Path::new("/System")).is_err());
        assert!(validate_delete(Path::new("/Applications")).is_err());
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn rejects_leaf_and_ancestor_symlinks_and_replaced_entries() {
        let root = fixture("links");
        fs::create_dir(root.join("folder")).unwrap();
        fs::write(root.join("folder/item"), "keep").unwrap();
        std::os::unix::fs::symlink(root.join("folder"), root.join("link")).unwrap();
        assert!(existing_path(&root.join("link")).is_err());
        assert!(existing_path(&root.join("link/item")).is_err());
        let id = FileIdentity::capture(&root.join("folder/item")).unwrap();
        fs::rename(root.join("folder/item"), root.join("old")).unwrap();
        fs::write(root.join("folder/item"), "new").unwrap();
        assert!(id.verify(&root.join("folder/item")).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn directory_deletion_cannot_bypass_descendant_exclusions() {
        let root = fixture("excluded");
        fs::write(root.join("protected.plist"), "keep").unwrap();
        assert!(validate_tree(&root, |p| p.extension().is_some_and(|ext| ext == "plist")).is_err());
        assert!(root.join("protected.plist").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
