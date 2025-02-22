/// Returns the language that corresponds to the given file extension.
pub fn language_for_extension(extension: &str) -> Option<&'static str> {
    match extension {
        "asciidoc" | "adoc" => Some("AsciiDoc"),
        "asm" => Some("Assembly"),
        "c" | "h" => Some("C"),
        "clj" => Some("Clojure"),
        "coq" => Some("Coq"),
        "cpp" => Some("C++"),
        "cr" => Some("Crystal"),
        "cs" => Some("C#"),
        "css" => Some("CSS"),
        "csv" => Some("CSV"),
        "d" => Some("D"),
        "dart" => Some("Dart"),
        "diff" | "patch" => Some("Diff"),
        "el" => Some("Emacs Lisp"),
        "elm" => Some("Elm"),
        "erl" => Some("Erlang"),
        "ex" => Some("Elixir"),
        "fish" => Some("Fish"),
        "fs" | "fsi" | "fsx" => Some("F#"),
        "gd" => Some("GDScript"),
        "gleam" => Some("Gleam"),
        "glsl" => Some("GLSL"),
        "go" => Some("Go"),
        "graphql" | "gql" => Some("GraphQL"),
        "hbs" => Some("Handlebars"),
        "heex" => Some("HTML (EEx)"),
        "hs" => Some("Haskell"),
        "html" | "htm" => Some("HTML"),
        "hx" => Some("Haxe"),
        "hy" => Some("Hy"),
        "idr" => Some("Idris"),
        "java" => Some("Java"),
        "jl" => Some("Julia"),
        "js" | "mjs" | "cjs" => Some("JavaScript"),
        "json" => Some("JSON"),
        "jsx" => Some("JavaScript (React)"),
        "kdl" => Some("KDL"),
        "kt" | "ktm" | "kts" => Some("Kotlin"),
        "less" => Some("Less"),
        "lfe" => Some("LFE"),
        "lisp" => Some("Common Lisp"),
        "lua" => Some("Lua"),
        "md" | "markdown" => Some("Markdown"),
        "ml" | "mli" => Some("OCaml"),
        "ncl" => Some("Nickel"),
        "nim" => Some("Nim"),
        "nix" => Some("Nix"),
        "php" => Some("PHP"),
        "ps1" => Some("PowerShell"),
        "purs" => Some("PureScript"),
        "py" => Some("Python"),
        "rb" => Some("Ruby"),
        "rkt" => Some("Racket"),
        "roc" => Some("Roc"),
        "rs" => Some("Rust"),
        "rst" => Some("reStructuredText"),
        "scala" => Some("Scala"),
        "scm" => Some("Scheme"),
        "scss" => Some("SCSS"),
        "sh" => Some("Shell"),
        "sql" => Some("SQL"),
        "svg" => Some("SVG"),
        "swift" => Some("Swift"),
        "tex" => Some("LaTeX"),
        "toml" => Some("TOML"),
        "ts" | "mts" | "cts" => Some("TypeScript"),
        "tsx" => Some("TypeScript (React)"),
        "twig" => Some("Twig"),
        "txt" => Some("Plaintext"),
        "vala" => Some("Vala"),
        "vb" => Some("Visual Basic"),
        "vue" => Some("Vue"),
        "wit" => Some("WIT"),
        "xml" => Some("XML"),
        "yaml" | "yml" => Some("YAML"),
        "zig" => Some("Zig"),
        _ => None,
    }
}
