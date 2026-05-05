# Security Policy

## Reporting a vulnerability

If you discover a security issue in GlyphWeave (input parsers,
font/image loading, fuzz crashes, denial-of-service, etc.), please
report it privately rather than opening a public issue.

**Preferred:** open a GitHub Security Advisory at
<https://github.com/Acture/glyphweave/security/advisories/new>.

**Alternative:** email the maintainers (acturea@gmail.com) with a
description and reproduction steps. Please do not include private user
data.

We aim to respond within 7 days. Coordinated disclosure dates are
agreed case-by-case.

## Supported versions

| Version | Supported |
|---|---|
| 0.4.x | ✅ active |
| 0.3.x | ❌ end-of-life |
| < 0.3 | ❌ end-of-life |

Security fixes land on the latest minor branch. Older minor branches
are not back-ported.

## Scope

In-scope:

- The library and CLI in this repository (`Acture/glyphweave`)
- The Rust dependencies declared in `Cargo.toml` (we accept reports
  about specific transitive issues that the binary actually exercises)

Out-of-scope:

- Vulnerabilities in dependencies that GlyphWeave does not exercise
  (please report those upstream)
- Issues in the embedded font (Noto Sans SC, SIL OFL 1.1) that are not
  triggered by GlyphWeave's code paths
- Resource exhaustion via deliberately adversarial inputs (e.g. a
  300×300 mask with `--max-tries 10000000`); these are tuning issues,
  not vulnerabilities

## Hardening notes

- Image loading uses the `image` crate's safe APIs; `--shape-image`
  inputs go through `Nearest` filter resampling and alpha-threshold,
  no unsafe.
- Word-file parsing is line-based with explicit length and weight
  bounds; pathological inputs are tested by the hand-rolled fuzz
  harness in `tests/fuzz_inputs.rs` and the cargo-fuzz target in
  `fuzz/`.
- Font loading delegates to `fontdue`; we apply no additional parsing
  on the binary.
- The crate has `#![forbid(unsafe_code)]`-clean code paths in `src/`
  (verify locally with `! grep -RIn 'unsafe' src/`).

## License

This project is AGPL-3.0; see [LICENSE](LICENSE) for the full text.
Security disclosures are accepted under that license.
