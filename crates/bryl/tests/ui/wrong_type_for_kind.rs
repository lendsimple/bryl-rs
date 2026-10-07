use bryl::Record;

#[derive(Record)]
struct R {
    #[bryl(numeric(5))]
    a: String,
    #[bryl(alpha(5))]
    b: u32,
    #[bryl(date("YYMMDD"))]
    c: chrono::NaiveDateTime,
}

fn main() {}
