// build.rs — Phase 1 stub
// Type stubs are hand-written in src/generated/types.rs for Phase 1.
// XSD codegen will be added in a future phase once toolchain constraints are resolved.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=wsdl/");
}
