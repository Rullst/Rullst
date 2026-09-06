use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    io,
    path::{Component, Path},
};
use tokio::sync::mpsc;

pub(super) fn relevant(event: &Event, root: &Path) -> bool {
    if matches!(event.kind, EventKind::Access(_) | EventKind::Other) {
        return false;
    }
    event.paths.iter().any(|path| {
        let Ok(relative) = path.strip_prefix(root) else {
            return false;
        };
        let mut parts = relative.components();
        match parts.next() {
            Some(Component::Normal(first))
                if ["src", "static", "templates", "assets"]
                    .iter()
                    .any(|name| first == *name) =>
            {
                true
            }
            Some(Component::Normal(first)) if parts.next().is_none() => {
                ["Cargo.toml", "Cargo.lock", "Rullst.toml", ".env"]
                    .iter()
                    .any(|name| first == *name)
            }
            _ => false,
        }
    })
}

pub(super) fn watch_project(root: &Path) -> io::Result<(RecommendedWatcher, mpsc::Receiver<()>)> {
    let root = root.canonicalize()?;
    let watched = root.clone();
    let (tx, rx) = mpsc::channel(1);
    let mut watcher =
        notify::recommended_watcher(move |event: Result<Event, notify::Error>| match event {
            Ok(event) if relevant(&event, &watched) => {
                let _ = tx.try_send(());
            }
            Err(error) => eprintln!("Development watcher error: {error}"),
            _ => {}
        })
        .map_err(io::Error::other)?;
    // Do not recursively register target/.git or unrelated runtime directories.
    watcher
        .watch(&root, RecursiveMode::NonRecursive)
        .map_err(io::Error::other)?;
    watch_directories(&mut watcher, &root)?;
    Ok((watcher, rx))
}

pub(super) fn watch_directories(watcher: &mut RecommendedWatcher, root: &Path) -> io::Result<()> {
    for directory in ["src", "static", "templates", "assets"] {
        let path = root.join(directory);
        if path.is_dir() {
            watcher
                .watch(&path, RecursiveMode::Recursive)
                .map_err(io::Error::other)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saves_deletes_and_atomic_renames_reload_but_build_outputs_do_not() {
        let root = Path::new("/project");
        for path in [
            "src/main.rs",
            "static/new.css",
            "templates/page.html",
            ".env",
            "Cargo.toml",
        ] {
            let event = Event::new(EventKind::Any).add_path(root.join(path));
            assert!(relevant(&event, root), "{path}");
        }
        for path in [
            "target/debug/app",
            ".git/index",
            "db.sqlite-wal",
            "logs/debug.log",
        ] {
            let event = Event::new(EventKind::Any).add_path(root.join(path));
            assert!(!relevant(&event, root), "{path}");
        }
        assert!(!relevant(
            &Event::new(EventKind::Access(notify::event::AccessKind::Read))
                .add_path(root.join("src/main.rs")),
            root
        ));
        let rename = Event::new(EventKind::Modify(notify::event::ModifyKind::Name(
            notify::event::RenameMode::Both,
        )))
        .add_path(root.join(".editor-temp"))
        .add_path(root.join("src/main.rs"));
        assert!(relevant(&rename, root));
    }
}
