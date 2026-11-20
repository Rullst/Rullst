### Fixed

- The HTML page of `cargo rullst make:controller` used inline `style`
  attributes, which the default Content-Security-Policy
  (`style-src 'self' 'nonce-…'`) blocks, so it rendered unstyled and the
  browser logged violations. Its styles are now classes in a `<style>` element
  that carries the request's CSP nonce (`index` takes
  `Option<Extension<CspNonce>>`). Controllers generated earlier keep their
  attributes until edited.
