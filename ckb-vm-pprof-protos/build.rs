extern crate protoc_rust;

fn main() {
    let mut codegen = protoc_rust::Codegen::new();
    #[cfg(target_os = "linux")]
    {
        codegen.protoc_path(protoc_bin_vendored::protoc_bin_path().unwrap());
    }
    codegen.out_dir("src").inputs(&["protos/profile.proto"]).include("protos").run().expect("protoc");
}
