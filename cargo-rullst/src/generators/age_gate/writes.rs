//! Validate the complete plan before replacing any application file.
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

pub(super) struct Edit {
    path: PathBuf,
    old: Option<String>,
    new: String,
}

impl Edit {
    pub fn replace(path: PathBuf, old: String, new: String) -> Self {
        Self {
            path,
            old: Some(old),
            new,
        }
    }
    pub fn create(path: PathBuf, new: String) -> io::Result<Self> {
        reject_links(&path)?;
        if path.try_exists()? {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "age-gate output already exists",
            ));
        }
        Ok(Self {
            path,
            old: None,
            new,
        })
    }
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
                    "age-gate application paths must not contain symlinks",
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn replace(path: &Path, content: &str, create: bool) -> io::Result<()> {
    reject_links(path)?;
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "age-gate output has no parent")
    })?;
    fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(content.as_bytes())?;
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
        Some(read(&edit.path)?)
    } else {
        None
    };
    if current != edit.old {
        return Err(io::Error::other(
            "age-gate input changed since the plan was prepared",
        ));
    }
    Ok(())
}

pub(super) fn apply(edits: &[Edit]) -> io::Result<()> {
    apply_with(edits, replace)
}

fn apply_with(
    edits: &[Edit],
    mut write: impl FnMut(&Path, &str, bool) -> io::Result<()>,
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
                if read(&applied.path).ok().as_deref() != Some(&applied.new) {
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
                    "age-gate write failed; concurrent changes or a rollback error require reviewing the application diff",
                )
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_inputs_are_preserved_and_late_io_failure_restores_prior_edits() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("main.rs");
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
        let generated = directory.path().join("generated.rs");
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
}
