use bryl::{Const, Record};

#[derive(Record)]
struct R {
    #[bryl(date("MMDDYYYY"), reserved)]
    a: Const,
}

fn main() {}
