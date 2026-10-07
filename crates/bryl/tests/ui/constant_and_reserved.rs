use bryl::{Const, Record};

#[derive(Record)]
struct R {
    #[bryl(alpha(2), constant = "AB", reserved)]
    a: Const,
}

fn main() {}
