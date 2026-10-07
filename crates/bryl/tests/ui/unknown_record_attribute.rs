use bryl::Record;

#[derive(Record)]
#[bryl(size = 2)]
struct R {
    #[bryl(alpha(2))]
    a: String,
}

fn main() {}
