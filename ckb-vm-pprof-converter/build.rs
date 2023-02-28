extern crate protoc_rust;

fn main() {
    protoc_rust::Codegen::new()
        .protoc_path(protoc_bin_vendored::protoc_bin_path().unwrap())
        .out_dir("src/protos")
        .inputs(&["proto/profile.proto"])
        .include("proto")
        .run()
        .expect("protoc");
}
