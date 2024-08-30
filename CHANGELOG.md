# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added the ability provide configuration via a configuration file.
  - The configuration file is located at `~/.config/code-stats/config.toml`.

### Removed

- Removed support for loading environment variables from `.env` files in parent directories.

## [0.2.1] - 2024-08-28

### Added

- Added support for more languages:
  - AsciiDoc
  - Assembly
  - C++
  - Coq
  - Crystal
  - CSV
  - D
  - Diff
  - Emacs Lisp
  - Fish
  - GDScript
  - GraphQL
  - Handlebars
  - Haxe
  - HTML (EEx)
  - Hy
  - Idris
  - Java
  - Kotlin
  - LaTeX
  - Less
  - LFE
  - Lua
  - Nim
  - Nix
  - Plaintext
  - PowerShell
  - Racket
  - reStructuredText
  - Roc
  - Scala
  - Scheme
  - Shell
  - SVG
  - Swift
  - Twig
  - Vala
  - Visual Basic
  - Vue
  - WIT
  - XML
- Added additional file extensions for languages:
  - F#: `.fsi` and `.fsx`
  - HTML: `.htm`
  - JavaScript: `.mjs` and `.cjs`
  - TypeScript: `.mts` and `.cts`

## [0.2.0] - 2024-08-27

### Added

- Added the ability to customize the API URL using the `CODE_STATS_API_URL` environment variable.
- Added a periodic flush of pulses to ensure XP doesn't get stuck in the queue.

## [0.1.1] - 2024-08-26

### Added

- Added support for more languages:
  - C
  - C#
  - Clojure
  - CSS
  - Dart
  - Elixir
  - Elm
  - Erlang
  - F#
  - Go
  - Haskell
  - Julia
  - OCaml
  - PHP
  - PureScript
  - Python
  - Ruby
  - SCSS
  - SQL
  - Zig

## [0.1.0] - 2024-08-26

- Initial release.

[unreleased]: https://github.com/maxdeviant/code-stats-ls/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/maxdeviant/code-stats-ls/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/maxdeviant/code-stats-ls/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/maxdeviant/code-stats-ls/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/maxdeviant/code-stats-ls/compare/f996fe9...v0.1.0