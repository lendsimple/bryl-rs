use bryl::Record;

#[derive(Record)]
struct R {
    #[bryl(alpha(2), colour = "red")]
    a: String,
}

fn main() {}
