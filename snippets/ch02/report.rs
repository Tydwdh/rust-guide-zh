use std::io::Write;

fn main() {
    let mut out = std::io::stdout();
    out.write_all(b"report: 42\n").unwrap();
}
