use bryl::{Const, Record};

#[derive(Record)]
struct R {
    #[bryl(numeric(2), constant = "AB")]
    a: Const,
    #[bryl(alpha(2), constant = 12)]
    b: Const,
}

fn main() {}
