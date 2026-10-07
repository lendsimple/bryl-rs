use bryl::Record;

#[derive(Record)]
struct R {
    #[bryl(alpha(2), align = "center")]
    a: String,
    #[bryl(alpha(2), pad = 'é')]
    b: String,
}

fn main() {}
