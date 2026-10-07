use bryl::Record;

#[derive(Record)]
struct R {
    #[bryl(date("MMDDYYY"))]
    a: chrono::NaiveDate,
}

fn main() {}
