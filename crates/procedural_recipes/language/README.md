# Recipe language tooling

`highlighting.toml` is the only hand-maintained vocabulary used for Recipe
syntax highlighting. The platform-specific grammars are generated with:

```sh
cargo xtask recipe-language
```

CI verifies that generated files are current with:

```sh
cargo xtask recipe-language --check
```

The Rust parser remains the source of truth for Recipe validity and semantics.
The generated grammars are intentionally forgiving lexical highlighters, not
independent implementations of the Recipe language.
