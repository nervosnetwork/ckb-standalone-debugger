extern crate protobuf_codegen;

fn main() {
    let mut codegen = protobuf_codegen::Codegen::new();
    #[cfg(target_os = "linux")]
    {
        codegen.protoc_path(&protoc_bin_vendored::protoc_bin_path().unwrap());
    }
    codegen.out_dir("src").inputs(&["protos/profile.proto"]).include("protos").run().expect("protoc");
}
