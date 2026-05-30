fn main() {
    // Record rustc version for ABI compatibility checking
    let rustc_version = rustc_version::version().expect("Failed to get rustc version");
    println!("cargo:rustc-env=RUSTC_VERSION={}", rustc_version);
}
