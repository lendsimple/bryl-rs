use bryl::Record;

#[derive(Record)]
struct R {
    #[bryl(numeric(2), sanitize(upper))]
    a: u8,
}

fn main() {}
