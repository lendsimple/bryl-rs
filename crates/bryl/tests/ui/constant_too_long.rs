use bryl::{Const, Record};

#[derive(Record)]
struct R {
    #[bryl(numeric(2), constant = 999)]
    a: Const,
    #[bryl(alpha(2), constant = "HEADER")]
    b: Const,
}

fn main() {}
