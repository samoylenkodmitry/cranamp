use super::*;

#[test]
fn properly_escapes_x11_messages() {
    let assert_eq = |input: &str, output: &[u8]| {
        let mut buf = vec![];
        quote_string(input, &mut buf);
        assert_eq!(buf, output);
    };

    assert_eq("", b"\"\"");
    assert_eq("foo", b"\"foo\"");
    assert_eq("foo\"bar", b"\"foo\\\"bar\"");
}
