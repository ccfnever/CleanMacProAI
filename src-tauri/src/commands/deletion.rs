use super::path_safety::{self, FileIdentity};
use crate::models::DeletionMode;
use std::{fs, path::Path};

pub fn confirmed_mode(
    mode: Option<DeletionMode>,
    permanent_confirmed: Option<bool>,
) -> Result<DeletionMode, String> {
    let mode = mode.unwrap_or_default();
    if mode == DeletionMode::Permanent && permanent_confirmed != Some(true) {
        return Err("永久删除必须明确确认；默认操作为移入废纸篓".into());
    }
    Ok(mode)
}

pub fn execute(path: &Path, identity: &FileIdentity, mode: DeletionMode) -> Result<(), String> {
    let path = path_safety::validate_delete(path)?;
    identity.verify(&path)?;
    match mode {
        DeletionMode::Trash => super::trash_support::move_to_trash(&path),
        DeletionMode::Permanent => {
            let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
            if metadata.is_dir() {
                fs::remove_dir_all(path)
            } else {
                fs::remove_file(path)
            }
            .map_err(|e| e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_is_trash_and_permanent_requires_explicit_confirmation() {
        assert_eq!(confirmed_mode(None, None).unwrap(), DeletionMode::Trash);
        assert!(confirmed_mode(Some(DeletionMode::Permanent), None).is_err());
        assert!(confirmed_mode(Some(DeletionMode::Permanent), Some(false)).is_err());
        assert_eq!(
            confirmed_mode(Some(DeletionMode::Permanent), Some(true)).unwrap(),
            DeletionMode::Permanent
        );
    }
    #[test]
    fn permanent_deletes_only_the_validated_fixture() {
        let root = std::env::temp_dir().join(format!("cleanmac-delete-{}", std::process::id()));
        fs::create_dir_all(root.join("folder")).unwrap();
        fs::write(root.join("folder/item"), "fixture").unwrap();
        let path = root.join("folder");
        let id = FileIdentity::capture(&path).unwrap();
        execute(
            &path,
            &id,
            confirmed_mode(Some(DeletionMode::Permanent), Some(true)).unwrap(),
        )
        .unwrap();
        assert!(!path.exists());
        fs::remove_dir(root).unwrap();
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn default_moves_fixture_to_trash_without_permanent_fallback() {
        let root =
            std::env::temp_dir().join(format!("cleanmac-default-trash-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("disposable.txt");
        fs::write(&path, "fixture").unwrap();
        let id = FileIdentity::capture(&path).unwrap();
        execute(&path, &id, confirmed_mode(None, None).unwrap()).unwrap();
        assert!(!path.exists());
        assert!(execute(&path, &id, DeletionMode::Trash).is_err());
        fs::remove_dir(root).unwrap();
    }
}
