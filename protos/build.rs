extern crate protoc_rust;

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    let mut codegen = protoc_rust::Codegen::new();
    if target_os == "linux" {
        codegen.protoc_path(protoc_bin_vendored::protoc_bin_path().unwrap());
    }
    codegen.out_dir("src/protos").inputs(&["../protos/profile.proto"]).include("../protos").run().expect("protoc");
}
