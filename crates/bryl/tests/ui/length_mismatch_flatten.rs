use bryl::Record;

#[derive(Record)]
struct Inner {
    #[bryl(alpha(2))]
    a: String,
}

#[derive(Record)]
#[bryl(length = 10)]
struct Outer {
    #[bryl(flatten)]
    inner: Inner,
    #[bryl(alpha(2))]
    b: String,
}

fn main() {}
