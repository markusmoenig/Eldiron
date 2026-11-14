// Original-channel codecs run in a dedicated realm with no editor, GPU or
// application startup. This file lives beside both generated wasm modules.
if (typeof window !== "undefined" || typeof document !== "undefined") {
    throw new Error("Bake codecs require a dedicated worker realm");
}
const { default: initialize } = await import("./eldiron-bake-codec-worker.js");
await initialize();
