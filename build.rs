fn main() {
    aarch64_windows_linker_setup();
}

fn aarch64_windows_linker_setup() {
    println!(r"cargo:rustc-link-search=/mnt/c/Program Files (x86)/Windows Kits/10/Lib/10.0.22621.0/um/arm64/");
    println!(r"cargo:rustc-link-search=/mnt/c/Program Files (x86)/Windows Kits/10/Lib/10.0.22621.0/ucrt/arm64");
    println!(r"cargo:rustc-link-search=/mnt/d/Program Files/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/14.40.33807/lib/arm64/");
}
