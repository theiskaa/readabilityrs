use std::collections::HashMap;

use once_cell::sync::Lazy;

/// Map of language aliases to canonical names.
/// Covers 170+ languages mirroring defuddle's coverage.
static LANGUAGE_MAP: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // Common aliases
    m.insert("py", "python");
    m.insert("python3", "python");
    m.insert("py3", "python");
    m.insert("js", "javascript");
    m.insert("jsx", "javascript");
    m.insert("ts", "typescript");
    m.insert("tsx", "typescript");
    m.insert("rb", "ruby");
    m.insert("rs", "rust");
    m.insert("sh", "shell");
    m.insert("bash", "shell");
    m.insert("zsh", "shell");
    m.insert("fish", "shell");
    m.insert("ksh", "shell");
    m.insert("csh", "shell");
    m.insert("ps1", "powershell");
    m.insert("psm1", "powershell");
    m.insert("cs", "csharp");
    m.insert("c#", "csharp");
    m.insert("fs", "fsharp");
    m.insert("f#", "fsharp");
    m.insert("vb", "vbnet");
    m.insert("vbnet", "vbnet");
    m.insert("vb.net", "vbnet");
    m.insert("objc", "objectivec");
    m.insert("objective-c", "objectivec");
    m.insert("objectivec", "objectivec");
    m.insert("mm", "objectivecpp");
    m.insert("kt", "kotlin");
    m.insert("kts", "kotlin");
    m.insert("sc", "scala");
    m.insert("ex", "elixir");
    m.insert("exs", "elixir");
    m.insert("erl", "erlang");
    m.insert("hrl", "erlang");
    m.insert("hs", "haskell");
    m.insert("lhs", "haskell");
    m.insert("ml", "ocaml");
    m.insert("mli", "ocaml");
    m.insert("clj", "clojure");
    m.insert("cljs", "clojurescript");
    m.insert("cljc", "clojure");
    m.insert("coffee", "coffeescript");
    m.insert("litcoffee", "coffeescript");
    m.insert("pl", "perl");
    m.insert("pm", "perl");
    m.insert("php3", "php");
    m.insert("php4", "php");
    m.insert("php5", "php");
    m.insert("php7", "php");
    m.insert("php8", "php");
    m.insert("phtml", "php");
    m.insert("lua", "lua");
    m.insert("r", "r");
    m.insert("rscript", "r");
    m.insert("jl", "julia");
    m.insert("nim", "nim");
    m.insert("cr", "crystal");
    m.insert("d", "d");
    m.insert("dart", "dart");
    m.insert("elm", "elm");
    m.insert("groovy", "groovy");
    m.insert("gvy", "groovy");
    m.insert("gradle", "groovy");
    m.insert("m", "matlab");
    m.insert("mat", "matlab");
    m.insert("pas", "pascal");
    m.insert("pp", "pascal");
    m.insert("delphi", "pascal");
    m.insert("f90", "fortran");
    m.insert("f95", "fortran");
    m.insert("f03", "fortran");
    m.insert("f08", "fortran");
    m.insert("for", "fortran");
    m.insert("f", "fortran");
    m.insert("cob", "cobol");
    m.insert("cbl", "cobol");
    m.insert("asm", "assembly");
    m.insert("nasm", "assembly");
    m.insert("masm", "assembly");
    m.insert("s", "assembly");
    m.insert("v", "verilog");
    m.insert("sv", "systemverilog");
    m.insert("vhd", "vhdl");
    m.insert("vhdl", "vhdl");

    // Markup / Data
    m.insert("htm", "html");
    m.insert("xhtml", "html");
    m.insert("xml", "xml");
    m.insert("xsl", "xml");
    m.insert("xslt", "xml");
    m.insert("svg", "xml");
    m.insert("rss", "xml");
    m.insert("atom", "xml");
    m.insert("md", "markdown");
    m.insert("mdx", "markdown");
    m.insert("mkd", "markdown");
    m.insert("rst", "restructuredtext");
    m.insert("rest", "restructuredtext");
    m.insert("tex", "latex");
    m.insert("ltx", "latex");
    m.insert("sty", "latex");
    m.insert("json", "json");
    m.insert("json5", "json");
    m.insert("jsonc", "json");
    m.insert("geojson", "json");
    m.insert("yml", "yaml");
    m.insert("yaml", "yaml");
    m.insert("toml", "toml");
    m.insert("ini", "ini");
    m.insert("cfg", "ini");
    m.insert("conf", "ini");
    m.insert("csv", "csv");
    m.insert("tsv", "csv");
    m.insert("graphql", "graphql");
    m.insert("gql", "graphql");
    m.insert("proto", "protobuf");
    m.insert("protobuf", "protobuf");
    m.insert("thrift", "thrift");
    m.insert("avro", "avro");

    // CSS / Style
    m.insert("css", "css");
    m.insert("scss", "scss");
    m.insert("sass", "sass");
    m.insert("less", "less");
    m.insert("styl", "stylus");
    m.insert("stylus", "stylus");

    // Config / Infra
    m.insert("dockerfile", "dockerfile");
    m.insert("docker", "dockerfile");
    m.insert("tf", "terraform");
    m.insert("hcl", "terraform");
    m.insert("nix", "nix");
    m.insert("cmake", "cmake");
    m.insert("makefile", "makefile");
    m.insert("make", "makefile");
    m.insert("mk", "makefile");

    // Shell / Script
    m.insert("bat", "batch");
    m.insert("cmd", "batch");
    m.insert("ps", "powershell");
    m.insert("pwsh", "powershell");
    m.insert("awk", "awk");
    m.insert("sed", "sed");

    // Database
    m.insert("sql", "sql");
    m.insert("mysql", "sql");
    m.insert("pgsql", "sql");
    m.insert("plsql", "sql");
    m.insert("tsql", "sql");
    m.insert("sqlite", "sql");
    m.insert("psql", "sql");

    // Functional / Other
    m.insert("lisp", "lisp");
    m.insert("el", "lisp");
    m.insert("rkt", "racket");
    m.insert("scm", "scheme");
    m.insert("ss", "scheme");
    m.insert("sml", "sml");
    m.insert("sig", "sml");
    m.insert("pro", "prolog");
    m.insert("P", "prolog");

    // Systems
    m.insert("c", "c");
    m.insert("h", "c");
    m.insert("cpp", "cpp");
    m.insert("cc", "cpp");
    m.insert("cxx", "cpp");
    m.insert("hpp", "cpp");
    m.insert("hxx", "cpp");
    m.insert("c++", "cpp");
    m.insert("zig", "zig");
    m.insert("ada", "ada");
    m.insert("adb", "ada");
    m.insert("ads", "ada");

    // Web / Template
    m.insert("vue", "vue");
    m.insert("svelte", "svelte");
    m.insert("hbs", "handlebars");
    m.insert("handlebars", "handlebars");
    m.insert("mustache", "mustache");
    m.insert("ejs", "ejs");
    m.insert("pug", "pug");
    m.insert("jade", "pug");
    m.insert("erb", "erb");
    m.insert("haml", "haml");
    m.insert("slim", "slim");
    m.insert("twig", "twig");
    m.insert("jinja", "jinja2");
    m.insert("jinja2", "jinja2");
    m.insert("j2", "jinja2");
    m.insert("liquid", "liquid");

    // Mobile
    m.insert("swift", "swift");
    m.insert("java", "java");
    m.insert("go", "go");
    m.insert("golang", "go");

    // Misc
    m.insert("wasm", "wasm");
    m.insert("wat", "wasm");
    m.insert("sol", "solidity");
    m.insert("vy", "vyper");
    m.insert("move", "move");
    m.insert("cairo", "cairo");
    m.insert("diff", "diff");
    m.insert("patch", "diff");
    m.insert("applescript", "applescript");
    m.insert("osascript", "applescript");
    m.insert("ahk", "autohotkey");
    m.insert("autohotkey", "autohotkey");

    m
});

/// Known canonical language names for bare class matching.
static KNOWN_LANGUAGES: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "python", "javascript", "typescript", "ruby", "rust", "go", "java", "kotlin",
        "scala", "swift", "dart", "elixir", "erlang", "haskell", "ocaml", "clojure",
        "perl", "php", "lua", "r", "julia", "nim", "crystal", "shell", "bash",
        "powershell", "csharp", "fsharp", "vbnet", "objectivec", "cpp", "c",
        "zig", "ada", "fortran", "cobol", "pascal", "assembly", "verilog", "vhdl",
        "systemverilog", "sql", "html", "css", "scss", "sass", "less", "xml",
        "json", "yaml", "toml", "markdown", "latex", "graphql", "protobuf",
        "dockerfile", "terraform", "nix", "makefile", "cmake", "batch",
        "vue", "svelte", "handlebars", "mustache", "ejs", "pug", "erb",
        "haml", "slim", "twig", "jinja2", "liquid", "diff", "wasm",
        "solidity", "matlab", "groovy", "coffeescript", "lisp", "scheme",
        "racket", "prolog", "sml", "ini", "csv", "restructuredtext",
        "applescript", "autohotkey",
    ]
});

/// Normalize a language identifier to its canonical name.
///
/// Returns the canonical name if found in the alias map or the known language
/// list; otherwise returns the lowercased input unchanged.
pub fn normalize_language(lang: &str) -> String {
    let lower = lang.trim().to_lowercase();

    // Direct alias lookup
    if let Some(&canonical) = LANGUAGE_MAP.get(lower.as_str()) {
        return canonical.to_string();
    }

    // Already a known canonical name
    if KNOWN_LANGUAGES.contains(&lower.as_str()) {
        return lower;
    }

    lower
}

/// Check whether a class token could be a bare language name.
pub fn is_known_language(name: &str) -> bool {
    let lower = name.trim().to_lowercase();
    LANGUAGE_MAP.contains_key(lower.as_str()) || KNOWN_LANGUAGES.contains(&lower.as_str())
}
