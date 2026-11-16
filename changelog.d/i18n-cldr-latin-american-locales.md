### Added

- `rullst::i18n` formats numbers, money and dates for seven more locales
  with CLDR 48.0 data: `es-419` (Latin American Spanish), `es-AR`, `es-CL`,
  `es-CO`, `es-MX`, `es-PE` and `es-UY`, which until now wrote like Spain's
  `es` (`1234,56 ARS`). `es-AR` now writes `$ 1.234,56` for pesos and
  `es-MX` writes `$1,234.56`. `Currency` has ten more constants with each
  locale's symbol: `ARS`, `MXN`, `CLP`, `COP`, `PEN`, `UYU`, `CAD`, `AUD`,
  `CHF` and `CNY` (ISO 4217 minor digits; `CLP` has none), so `en` writes
  `CA$1,234.56` and `es-PE` writes `S/ 1,234.56` instead of the ISO code.
  The new `Locale::currency_name(currency)` returns CLDR's display name of
  any currency with a constant (`Some("Peso argentino")` in `pt-BR`), and
  `None` for other codes. ([#636](https://github.com/Rullst/Rullst/issues/636))
