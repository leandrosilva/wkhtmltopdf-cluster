#[cfg(windows)]
fn main() {
    println!(r"cargo:rustc-link-search=C:\Program Files (x86)\wkhtmltopdf\lib");
}

#[cfg(unix)]
fn main() {
}