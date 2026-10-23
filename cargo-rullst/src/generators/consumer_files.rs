//! Validate the complete plan before replacing any application file.
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

pub(super) struct Edit {
    path: PathBuf,
    old: Option<Vec<u8>>,
    new: Vec<u8>,
}

impl Edit {
    pub fn replace(path: PathBuf, old: String, new: String) -> Self {
        Self {
            path,
            old: Some(old.into_bytes()),
            new: new.into_bytes(),
        }
    }
    pub fn create(path: PathBuf, new: String) -> io::Result<Self> {
        Self::create_bytes(path, new.into_bytes())
    }
    /// [`Self::create`] for a file that is not text, such as an icon.
    pub fn create_bytes(path: PathBuf, new: Vec<u8>) -> io::Result<Self> {
        reject_links(&path)?;
        if path.try_exists()? {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "consumer scaffold output already exists",
            ));
        }
        Ok(Self {
            path,
            old: None,
            new,
        })
    }
}

/// The project root with the symlinks in its own spelling resolved: macOS
/// spells temporary directories `/var` -> `/private/var`, and a Windows
/// project may sit behind a junction. Every path joined to the result then
/// has no link above the root, so the guard refuses only links beneath it.
pub(super) fn project_root(root: &Path) -> io::Result<PathBuf> {
    fs::canonicalize(root)
}

pub(super) fn read(path: &Path) -> io::Result<String> {
    reject_links(path)?;
    fs::read_to_string(path)
}

fn reject_links(path: &Path) -> io::Result<()> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if metadata.is_symlink() => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "consumer scaffold application paths must not contain symlinks",
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn replace(path: &Path, content: &[u8], create: bool) -> io::Result<()> {
    reject_links(path)?;
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "consumer scaffold output has no parent",
        )
    })?;
    fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(content)?;
    if let Ok(metadata) = fs::metadata(path) {
        file.as_file().set_permissions(metadata.permissions())?;
    }
    file.as_file().sync_all()?;
    if create {
        file.persist_noclobber(path).map_err(|error| error.error)?;
    } else {
        file.persist(path).map_err(|error| error.error)?;
    }
    Ok(())
}

fn unchanged(edit: &Edit) -> io::Result<()> {
    reject_links(&edit.path)?;
    let current = if edit.path.try_exists()? {
        Some(fs::read(&edit.path)?)
    } else {
        None
    };
    if current != edit.old {
        return Err(io::Error::other(
            "consumer scaffold input changed since the plan was prepared",
        ));
    }
    Ok(())
}

pub(super) fn apply(edits: &[Edit]) -> io::Result<()> {
    apply_with(edits, replace)
}

fn apply_with(
    edits: &[Edit],
    mut write: impl FnMut(&Path, &[u8], bool) -> io::Result<()>,
) -> io::Result<()> {
    for edit in edits {
        unchanged(edit)?;
    }
    for (index, edit) in edits.iter().enumerate() {
        if let Err(error) =
            unchanged(edit).and_then(|()| write(&edit.path, &edit.new, edit.old.is_none()))
        {
            let mut restored = true;
            for applied in edits[..index].iter().rev() {
                if fs::read(&applied.path).ok().as_deref() != Some(applied.new.as_slice()) {
                    restored = false;
                    continue;
                }
                let undo = match &applied.old {
                    Some(old) => replace(&applied.path, old, false),
                    None => fs::remove_file(&applied.path),
                };
                restored &= undo.is_ok();
            }
            return Err(if restored {
                error
            } else {
                io::Error::other(
                    "consumer scaffold write failed; concurrent changes or a rollback error require reviewing the application diff",
                )
            });
        }
    }
    Ok(())
}

/// A project directory reached through a symlinked parent, like macOS's
/// temporary directory, for the consumer generator tests.
#[cfg(test)]
pub(super) fn symlinked_project(temporary: &Path) -> PathBuf {
    let real = temporary.join("real");
    fs::create_dir(&real).expect("real project directory");
    #[cfg(unix)]
    {
        let link = temporary.join("link");
        std::os::unix::fs::symlink(&real, &link).expect("symlinked project parent");
        link
    }
    #[cfg(not(unix))]
    real
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_inputs_are_preserved_and_late_io_failure_restores_prior_edits() {
        let directory = tempfile::tempdir().unwrap();
        // macOS's temporary-directory spelling may traverse /var -> /private/var.
        // Exercise the same resolved root that the CLI obtains from current_dir.
        let root = directory.path().canonicalize().unwrap();
        let path = root.join("main.rs");
        fs::write(&path, "original").unwrap();
        let plan = vec![Edit::replace(
            path.clone(),
            "original".into(),
            "generated".into(),
        )];
        fs::write(&path, "concurrent edit").unwrap();
        assert!(apply(&plan).is_err());
        assert_eq!(read(&path).unwrap(), "concurrent edit");

        fs::write(&path, "original").unwrap();
        let mut plan = plan;
        let generated = root.join("generated.rs");
        plan.push(Edit::create(generated.clone(), "content".into()).unwrap());
        let mut calls = 0;
        assert!(
            apply_with(&plan, |path, content, create| {
                calls += 1;
                if calls == 2 {
                    Err(io::Error::other("injected second-write failure"))
                } else {
                    replace(path, content, create)
                }
            })
            .is_err()
        );
        assert_eq!(calls, 2);
        assert_eq!(read(&path).unwrap(), "original");
        assert!(!generated.exists());
    }

    #[test]
    fn binary_files_are_created_exactly_and_removed_on_rollback() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let icon = root.join("static/icon.png");
        let bytes = vec![0x89, b'P', b'N', b'G', 0xff, 0x00];
        apply(&[Edit::create_bytes(icon.clone(), bytes.clone()).unwrap()]).unwrap();
        assert_eq!(fs::read(&icon).unwrap(), bytes);
        assert!(Edit::create_bytes(icon.clone(), bytes.clone()).is_err());

        let second = root.join("static/second.png");
        let plan = vec![
            Edit::create_bytes(second.clone(), bytes.clone()).unwrap(),
            Edit::create(root.join("late.rs"), "late".into()).unwrap(),
        ];
        let mut calls = 0;
        assert!(
            apply_with(&plan, |path, content, create| {
                calls += 1;
                if calls == 2 {
                    Err(io::Error::other("injected second-write failure"))
                } else {
                    replace(path, content, create)
                }
            })
            .is_err()
        );
        assert!(!second.exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_rollback_that_cannot_restore_an_edit_says_so() {
        use std::os::unix::fs::PermissionsExt;
        if rustix::process::geteuid().is_root() {
            return;
        }
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let locked = root.join("locked");
        fs::create_dir(&locked).unwrap();
        let first = locked.join("main.rs");
        fs::write(&first, "original").unwrap();
        let plan = vec![
            Edit::replace(first.clone(), "original".into(), "generated".into()),
            Edit::create(root.join("late.rs"), "late".into()).unwrap(),
        ];
        let mut calls = 0;
        let error = apply_with(&plan, |path, content, create| {
            calls += 1;
            if calls == 2 {
                // The restore of main.rs needs a temporary file in `locked`.
                fs::set_permissions(&locked, fs::Permissions::from_mode(0o500))?;
                Err(io::Error::other("injected second-write failure"))
            } else {
                replace(path, content, create)
            }
        })
        .unwrap_err();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            error.to_string(),
            "consumer scaffold write failed; concurrent changes or a rollback error require reviewing the application diff"
        );
        assert_eq!(fs::read_to_string(&first).unwrap(), "generated");
    }

    #[cfg(unix)]
    #[test]
    fn project_symlinks_still_refuse_writes_after_resolving_the_root() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let real = root.join("real");
        fs::create_dir(&real).unwrap();
        let link = root.join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert!(Edit::create(link.join("generated.rs"), "content".into()).is_err());
        assert!(!real.join("generated.rs").exists());
    }
}
