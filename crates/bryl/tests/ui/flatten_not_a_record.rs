use bryl::Record;

#[derive(Record)]
struct Outer {
    #[bryl(flatten)]
    inner: String,
}

fn main() {}
