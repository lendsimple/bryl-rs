use bryl::Record;

#[derive(Record)]
struct Inner {
    #[bryl(alpha(2))]
    a: String,
}

#[derive(Record)]
struct Outer {
    #[bryl(flatten, pad = '0')]
    inner: Inner,
}

fn main() {}
