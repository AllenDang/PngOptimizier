extern crate embed_resource;
fn main() {
    println!("cargo:rerun-if-changed=gui/mainview.fl");
    embed_resource::compile("PngOptimizer.rc", embed_resource::NONE)
        .manifest_required()
        .unwrap();
}
