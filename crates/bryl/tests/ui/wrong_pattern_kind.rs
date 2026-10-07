use bryl::Record;

#[derive(Record)]
struct R {
    #[bryl(date("YYYYMMDDhhmm"))]
    a: chrono::NaiveDate,
}

fn main() {}
