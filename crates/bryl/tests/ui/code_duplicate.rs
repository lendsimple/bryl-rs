use bryl::Code;

#[derive(Code)]
enum C {
    #[code("A")]
    A,
    #[code("A")]
    B,
}

fn main() {}
