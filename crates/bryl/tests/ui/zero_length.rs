use bryl::Record;

#[derive(Record)]
struct R {
    #[bryl(alpha(0))]
    a: String,
}

fn main() {}
