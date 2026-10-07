use bryl::Record;

#[derive(Record)]
struct R {
    #[bryl(alpha(2), constant = "K1")]
    a: String,
}

fn main() {}
