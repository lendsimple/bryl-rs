use bryl::Record;

#[derive(Record)]
struct R {
    #[bryl(alpha(2), numeric(2))]
    a: String,
}

fn main() {}
