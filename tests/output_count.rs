use tracegrep::output::{CountFormatter, Formatter};
use tracegrep::Record;

#[test]
fn count_formatter_writes_zero_then_newline_when_flushed_without_writes() {
    let mut buf = Vec::new();
    let mut f = CountFormatter::new(&mut buf);
    f.flush().unwrap();
    assert_eq!(String::from_utf8(buf).unwrap(), "0\n");
}

#[test]
fn count_formatter_accumulates_writes_until_flush_emitting_total_counter() {
    let mut buf = Vec::new();
    {
        let mut f = CountFormatter::new(&mut buf);
        let prototype = Record::new("{}".to_owned());
        f.write(&prototype).unwrap();
        f.write(&prototype).unwrap();
        f.write(&prototype).unwrap();
        f.flush().unwrap();
    }
    assert_eq!(String::from_utf8(buf).unwrap(), "3\n");
}
