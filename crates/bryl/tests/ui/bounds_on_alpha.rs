use bryl::Record;

#[derive(Record)]
struct R {
    #[bryl(alpha(2), min = 1, max = 5)]
    a: String,
}

fn main() {}
