extern crate cc;

fn main() {
    cc::Build::new()
        .file("src/timestamp.c")
        .compile("libtimestamp.a");
}
