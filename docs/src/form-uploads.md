# Form file uploads

`rullst::uploads::Multipart` reads a `multipart/form-data` request, the body a
browser sends for a form with a file input, under explicit limits. It is new in
13.0.0-alpha.2. Text fields stay in memory; each file streams to memory while it
is small and then to a private temporary file, so a large upload never sits in
memory as a whole.

Use it for ordinary form uploads that pass through the application. For large
files that the application forwards to S3 or R2 in resumable parts, see
[resumable private multipart uploads](private-multipart-uploads.md) instead.

## A handler

```rust,no_run
use rullst::uploads::{Multipart, MultipartError, MultipartPolicy, UploadKind};

async fn upload_document(multipart: Multipart) -> Result<String, MultipartError> {
    let policy = MultipartPolicy::default()
        .max_file_bytes(50 * 1024 * 1024)
        .max_total_bytes(51 * 1024 * 1024)
        .max_files(1)
        .allowed_kinds([UploadKind::Pdf])
        .temp_dir("storage/tmp");
    let mut form = multipart.read(&policy).await?;
    let title = form.field("title").unwrap_or("Untitled").to_owned();
    let Some(document) = form.take_file("document") else {
        return Ok("Choose a PDF to upload.".to_string());
    };
    // Content-addressed: the same PDF uploaded twice is the same file.
    let target = format!("storage/documents/{}.pdf", document.sha256_hex());
    match document.persist_to(&target).await {
        Ok(()) | Err(MultipartError::PersistTargetExists) => {}
        Err(error) => return Err(error),
    }
    Ok(format!("{title} was saved."))
}
```

Route it like any handler, for example `post("/documents" => upload_document)`
in `routes!`. `Multipart` must be the handler's last argument, because it takes
the request body. `MultipartError` is a response: a handler can return it with
`?`, as above, or map it to a page of its own.

The form renders `_token` first (see [CSRF](#csrf)):

```html
<form method="post" action="/documents" enctype="multipart/form-data">
  <input type="hidden" name="_token" value="…">
  <input name="title">
  <input type="file" name="document" accept="application/pdf">
  <button>Upload</button>
</form>
```

## Limits

Every limit has a safe default and a builder method. Exceeding one stops reading
at once: the rest of the body is never read, files already received are
dropped and their temporary files removed.

| Method | Default | Error | Status |
| :--- | :--- | :--- | :--- |
| `max_file_bytes` | 10 MiB | `FileTooLarge` | 413 |
| `max_total_bytes` (whole body; a larger `Content-Length` is refused before reading) | 16 MiB | `TotalTooLarge` | 413 |
| `max_files` | 4 | `TooManyFiles` | 413 |
| `max_fields` (text fields, `_token` included) | 32 | `TooManyFields` | 413 |
| `max_field_bytes` | 64 KiB | `FieldTooLarge` | 413 |
| `max_field_name_bytes` | 128 | `FieldNameTooLong` | 400 |
| `idle_timeout` (wait for the next piece of the body) | 30 s | `Timeout` | 408 |
| part header block (fixed) | 8 KiB | `PartHeadersTooLarge` | 400 |

A file input left empty arrives as a part with `filename=""` and no content; it
is skipped and does not count against `max_files`. The extractor ignores axum's
`DefaultBodyLimit`: `max_total_bytes` bounds the body instead. A body that does
not follow the form-data format (a preamble before the first boundary, a part
without `Content-Disposition`, a nested `multipart/mixed` part, a body that ends
before the closing boundary) is `Malformed` (400), and a text field that is not
UTF-8 is `FieldNotUtf8` (400). Error responses are plain text with
`Cache-Control: no-store` and never echo what the client sent.

## Temporary files

A file stays in memory while it is at most `memory_threshold` bytes (32 KiB by
default, at most 1 MiB; `0` writes every non-empty file to disk). A larger file
streams to a file named `.rullst-upload-<random>.part` in `temp_dir`, which
defaults to `std::env::temp_dir()` (`TMPDIR` on Unix). The file is created with
`create_new` and, on Unix, mode `0600`, and it is removed when its
`UploadedFile` is dropped, including when reading fails.

A process that crashes, or is killed, while it holds such a file cannot remove
it. Since 13.0.0-alpha.3 the first upload that a process writes to a
`temp_dir`, and the first one after each further 24 hours, first sweeps that
directory: it removes the regular files named exactly
`.rullst-upload-<32 lowercase hex digits>.part` (the names Rullst creates)
that were last modified 24 hours ago or earlier. It never follows or removes a
symbolic link or a directory, never touches other names, and examines at most
10,000 entries and removes at most 1,000 files per sweep. The sweep runs on the
blocking thread pool before that upload's file is created; if the directory
cannot be read or a file cannot be removed, it logs a `rullst::uploads`
warning and the upload continues.

`UploadedFile::persist_to(target)` keeps the content at `target`:

- it never overwrites: an existing `target`, a symbolic link included, is
  `PersistTargetExists`;
- `target` must name a file in an existing directory and must not contain
  `..`; otherwise it is `InvalidPersistTarget`;
- a temporary file is hard-linked into place when `temp_dir` is on the same
  file system, and copied otherwise; the new file has mode `0600` on Unix and
  is synced to disk.

Choose a `temp_dir` on the same file system as the final directory, and build
`target` from trusted values, such as an id or the SHA-256 digest, never from
the client's file name.

`bytes()` returns the content of a file kept in memory, `temp_path()` the
temporary file of a larger one (for a scanner that reads files, for example),
and `read_to_vec()` either.

## What the client says is not trusted

- **Content type.** `declared_content_type()` is what the browser claimed. The
  checks look at the bytes: `allowed_kinds` admits only the `UploadKind`s
  (PNG, JPEG, PDF, MP4, WebM, plain text) that the in-memory `UploadPolicy`
  recognizes, with the same rules, and `expect_signature(b"%PDF-")` admits only
  files starting with one of the given byte strings (1 to 64 bytes each). Both
  run while the file streams, so a mismatch stops the upload early with
  `ContentDenied` (415). `kind()` reports what was recognized either way.
- **File name.** `display_name()` is for display only: the last `/` or `\`
  segment of the client's name, without control or invisible format characters
  (such as the right-to-left override that disguises `fdp.png` as `png.pdf`),
  trimmed and cut to 255 bytes. Escape it when rendering, as `html!` does.

For a quarantine and malware-scanner flow, pass the content to
`UploadPolicy::admit` and `UploadScanner` (`read_to_vec()` gives the bytes).

## CSRF

The CSRF middleware reads a multipart body only until its first `_token` field
ends, within the first 64 KiB, so that it never buffers a whole upload. Render
`_token` as the form's first field, before any file input, or send the
`X-CSRF-Token` header (HTMX: `hx-headers`). Otherwise staging and production
answer `403` with:

```text
Missing CSRF token: a multipart/form-data form must send its _token field before any file field (the CSRF check reads only the first 64 KiB of the body), or send the X-CSRF-Token header
```

Development and test do not mount the CSRF middleware, so `Multipart::read`
applies the same rule in every environment: a first `_token` field that ends
after the first 64 KiB, without the header, is `CsrfTokenTooLate`, answered with
the same `403` and text. The mistake therefore shows up before a deployment.

## WAF

The WAF does not read multipart bodies. Where it runs, it marks the request,
and `Multipart::read` inspects the name and value of each text field with the
same signatures as a URL-encoded form body: staging and production refuse a
match with the WAF's `403` (`WafRefused`), development logs it at `warn` level
(target `rullst::security::waf`, rule `multipart-field-signature`, the path
only). Paths listed in `[security] waf_body_exempt_paths` are not inspected.
File contents and file names are not inspected.

## What it does not do

It does not parse documents, strip metadata, transcode media or scan for
malware, and it does not resume interrupted uploads. Leftover temporary files
are swept only from a directory this process writes uploads to, and only once
they are 24 hours old (see [Temporary files](#temporary-files)).
