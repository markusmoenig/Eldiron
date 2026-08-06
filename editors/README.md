# Eldiron editor language support

These are unpublished editor integration scaffolds for the evolving Eldiron
Recipe language.

- `vscode-eldiron-recipe` contains a generated, locally installable VS Code
  language extension.
- `tree-sitter-eldiron-recipe` contains the generated shallow Tree-sitter
  grammar used by Zed.
- `zed-eldiron-recipe` contains generated Zed language configuration and
  queries. It intentionally has no publishable extension manifest yet.

Do not edit generated files in these directories. Change
`crates/procedural_recipes/language/highlighting.toml` and run:

```sh
cargo xtask recipe-language
```
