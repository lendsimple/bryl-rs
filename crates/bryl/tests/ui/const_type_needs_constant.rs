use bryl::{Const, Record};

#[derive(Record)]
struct R {
    #[bryl(alpha(2))]
    a: Const,
}

fn main() {}
