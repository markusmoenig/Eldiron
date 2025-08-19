fn main() {
    let css = r#"body {
        background-color: #000;
        border-color: #000;
        margin: 0;
        overflow: hidden;
        height: 100vh;
        display: flex;
        justify-content: center;
        align-items: center;
    }
    canvas:focus {
        outline: none;
    }"#;

    cargo_run_wasm::run_wasm_with_css(css);
}
