use bryl::Record;

#[derive(Record)]
#[bryl(length = 10)]
struct R {
    #[bryl(alpha(2))]
    a: String,
}

fn main() {}
