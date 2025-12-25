// LUMENYX Runtime Build Script
// Compiles the runtime to WASM for on-chain execution

fn main() {
    #[cfg(feature = "std")]
    {
        substrate_wasm_builder::WasmBuilder::build_using_defaults();
    }
}
