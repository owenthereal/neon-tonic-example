fn main() {
    tonic_build::configure()
        .build_client(false)
        .build_server(true)
        .out_dir(&"src")
        .format(true)
        .compile(&["proto/service.proto"], &["proto"])
        .unwrap();
}
