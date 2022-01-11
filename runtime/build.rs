use substrate_wasm_builder::WasmBuilder;

// copy from substrate-node-template, standard build script for substrate chain
fn main() {
	WasmBuilder::new()
		.with_current_project()
		.export_heap_base()
		.import_memory()
		.build()
}
