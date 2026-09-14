# Studio stylesheet

`studio.css` is the minified, same-origin stylesheet embedded by
`rullst-studio`. It is generated from `studio-input.css` and the class names in
the Rust views with Tailwind CSS 3.4.17:

```bash
npx --yes tailwindcss@3.4.17 \
  -i ./assets/studio-input.css \
  -o ./assets/studio.css \
  --minify \
  --content './src/**/*.rs'
```

Regenerate it after adding or changing utility classes. The released Studio UI
must not require the Tailwind Play CDN or any other third-party browser script.
