use bryl::Record;

#[derive(Record)]
struct Tuple(String);

#[derive(Record)]
enum E {
    A,
}

#[derive(Record)]
struct Generic<T> {
    #[bryl(alpha(2))]
    a: T,
}

fn main() {}
