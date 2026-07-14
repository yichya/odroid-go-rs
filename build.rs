fn main() {
    embuild::espidf::sysenv::output();

    let out_dir = std::env::var("OUT_DIR").unwrap();

    let status = std::process::Command::new("python3")
        .args(["tools/build_font.py", "--out-dir", &out_dir])
        .status()
        .expect("failed to run build_font.py");
    if !status.success() {
        panic!("build_font.py failed");
    }

    println!("cargo:rerun-if-changed=tools/build_font.py");
    println!("cargo:rerun-if-changed=tools/extras.json");
    println!("cargo:rerun-if-changed=tools/base_code_table.txt");
}
