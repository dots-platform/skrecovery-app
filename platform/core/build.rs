fn main() {
    let proto_file = "../protos/dec_exec.proto";
    let proto_path = "../protos";

    tonic_build::configure()
        .build_server(true)
        .out_dir("./src")
        .compile(&[proto_file], &[proto_path])
        .unwrap_or_else(|e| panic!("protobuf compile error: {}", e));

    println!("cargo:rerun-if-changed={}", proto_file);
}