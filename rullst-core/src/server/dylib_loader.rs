use crate::Router;
use crate::server::server_middleware::{inject_hmr_script, zstd_static_middleware};

/// Dynamically loads an application router from a compiled dynamic library (`cdylib`).
#[cfg_attr(mutants, mutants::skip)]
pub fn load_dylib_router(
    lib_path: &str,
    is_dev: bool,
) -> Result<(axum::Router, libloading::Library), Box<dyn std::error::Error>> {
    let lib_extension = if cfg!(target_os = "windows") {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    };

    let full_lib_path = if lib_path.ends_with(".dll")
        || lib_path.ends_with(".so")
        || lib_path.ends_with(".dylib")
    {
        lib_path.to_string()
    } else {
        format!("{}.{}", lib_path, lib_extension)
    };

    let path_buf = std::path::Path::new(&full_lib_path);
    if !path_buf.exists() {
        return Err(format!("Dylib not found at: {}", full_lib_path).into());
    }

    let parent = path_buf.parent().unwrap_or(std::path::Path::new("."));
    let filename = path_buf
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid dylib path or non-UTF-8 characters detected",
            )
        })?;

    // Clean up older active files that are no longer locked by any process
    let expected_prefix = format!("{}_active_", filename);
    if let Ok(entries) = std::fs::read_dir(parent) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with(&expected_prefix) && name.ends_with(lib_extension) {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    }

    // L-1: Use UUID v4 instead of nanosecond timestamp to guarantee filename uniqueness
    //       even on systems with low-resolution clocks.
    let unique_id = uuid::Uuid::new_v4().as_simple().to_string();
    let temp_filename = format!("{}_active_{}.{}", filename, unique_id, lib_extension);
    let temp_path = parent.join(temp_filename);

    // Copy the dylib file to prevent locking issues
    std::fs::copy(&full_lib_path, &temp_path)?;

    // SAFETY: The code below performs dynamic library loading and raw pointer extraction.
    // Invariant requirements for safety:
    //  - The dynamically loaded library must expose a symbol `rullst_router_init`
    //    with signature `unsafe extern "C" fn() -> *mut Router`.
    //  - The `rullst_router_init` function must allocate a `Box<Router>` and return
    //    the raw pointer produced by `Box::into_raw`. The caller here uses
    //    `Box::from_raw` to take ownership of that pointer; therefore the
    //    library MUST NOT keep or free that pointer after returning it.
    //  - The loaded library must be ABI-compatible with the host Router layout.
    //  - The `temp_path` copy must remain available on disk for the lifetime of
    //    any references into the loaded library. We keep the returned `Library`
    //    object to ensure the library remains mapped until dropped by the caller.
    //  - Calls into the plugin must be synchronized appropriately by the host if
    //    the plugin is not internally thread-safe.
    //
    // This block is `unsafe` because it relies on the above invariants; any
    // future changes to router ABI or plugin implementations must be reflected
    // here and documented. Review and audit this section when upgrading
    // `libloading`, `Router` types, or changing the plugin API.
    let lib = unsafe { libloading::Library::new(&temp_path)? };
    if let Err(e) = std::fs::remove_file(&temp_path) {
        #[cfg(not(target_os = "windows"))]
        eprintln!(
            "⚠️ Rullst: failed to remove temporary dylib file at {:?}: {}",
            temp_path, e
        );
        #[cfg(target_os = "windows")]
        {
            // On Windows, sharing violation (error code 32) is normal, so we only log other errors.
            if e.raw_os_error() != Some(32) {
                eprintln!(
                    "⚠️ Rullst: failed to remove temporary dylib file at {:?}: {}",
                    temp_path, e
                );
            }
        }
    }
    let init_fn: libloading::Symbol<unsafe extern "C" fn() -> *mut Router> =
        unsafe { lib.get(b"rullst_router_init")? };
    let router_ptr = unsafe { init_fn() };

    // Convert *mut Router back to Router box and extract it
    let rullst_router = unsafe { *Box::from_raw(router_ptr) };

    // Convert Rullst Router to Axum Router and mount built-in Scalar API docs (/docs)
    let mut axum_router = rullst_router
        .into_axum()
        .merge(crate::scalar::scalar_docs_router("/openapi.json"));

    // Serve static files from "static" directory if it exists
    if std::path::Path::new("static").exists() {
        axum_router = axum_router
            .nest_service(
                "/static",
                tower_http::services::ServeDir::new("static").precompressed_br(),
            )
            .layer(axum::middleware::from_fn(zstd_static_middleware));
    }

    // Attach development explain / console routes
    if is_dev {
        axum_router = axum_router
            .route(
                "/_rullst/explain",
                axum::routing::get(crate::error_console::handle_explain),
            )
            .route(
                "/_rullst/autofix",
                axum::routing::post(crate::error_console::handle_autofix),
            )
            .layer(axum::middleware::from_fn(
                crate::error_console::catch_panic_middleware,
            ))
            .layer(axum::middleware::from_fn(inject_hmr_script));
    }

    Ok((axum_router, lib))
}
