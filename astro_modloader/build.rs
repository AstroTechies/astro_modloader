use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo-rerun-if-changed=build.rs");
    println!("cargo-rerun-if-changed=Cargo.lock");
    #[cfg(windows)]
    {
        winres::WindowsResource::new()
            .set_icon("assets/icon.ico")
            .compile()
            .unwrap();
    }

    let template_file = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("Failed to read CARGO_MANIFEST_DIR"),
    )
    .join("..") // gross hack because no way to get manifest dir
    .join("about.hbs");

    let licenses_dir =
        PathBuf::from(env::var_os("OUT_DIR").expect("Failed to read OUT_DIR")).join("licenses.md");

    Command::new("cargo")
        .arg("about")
        .arg("generate")
        .arg("--all-features")
        .arg("--workspace")
        .arg("-o")
        .arg(licenses_dir)
        .arg(template_file)
        .spawn()
        .ok()
        .and_then(|mut e| e.wait().ok())
        .expect("Failed to generate license summary");
}
