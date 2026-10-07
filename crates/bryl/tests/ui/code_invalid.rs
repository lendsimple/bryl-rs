use bryl::Code;

#[derive(Code)]
enum C {
    #[code(" A")]
    A,
    #[code("")]
    B,
    #[code(1.5)]
    D,
}

fn main() {}
