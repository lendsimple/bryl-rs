use bryl::Code;

#[derive(Code)]
enum C {
    #[code("A")]
    A,
    #[code(2)]
    B,
}

fn main() {}
