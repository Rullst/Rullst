### Changed

- `html!` writes a page into one `String`. Text known at compile time is
  joined into one write per run, `@if`, `@for` and `@match` write into the same
  buffer, and interpolated values are escaped (numbers formatted) straight into
  it, so rendering no longer allocates a `String` for every element, text node,
  block and value. The output is the same byte for byte, including a `continue`
  or `break` inside an `@for` body, which still leaves out the node it cut
  short. A value whose type does not implement `HtmlEscape` gets the same
  error; its note now names `rullst::html::push_escaped` (or
  `push_escaped_attr`) instead of `escape` (or `escape_attr`). `HtmlEscape`
  has two hidden provided methods that the expansion calls; an implementation
  outside Rullst keeps its output without changes.
