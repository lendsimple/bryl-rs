use bryl::Code;

#[derive(Code)]
enum WithFields {
    #[code("A")]
    A(u8),
}

#[derive(Code)]
enum Empty {}

#[derive(Code)]
struct NotEnum;

fn main() {}
