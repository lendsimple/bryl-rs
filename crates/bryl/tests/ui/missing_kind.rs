use bryl::Record;

#[derive(Record)]
struct R {
    #[bryl(pad = ' ')]
    a: String,
    b: String,
}

fn main() {}
