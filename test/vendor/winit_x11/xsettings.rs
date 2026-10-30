use super::*;

const XSETTINGS: &str = include_str!("xsettings.dat");

#[test]
fn empty() {
    let err = match read_settings(&[]) {
        Ok(_) => panic!(),
        Err(err) => err,
    };
    match err {
        ParserError::NoMoreBytes { expected, found } => {
            assert_eq!(expected.get(), 1);
            assert_eq!(found, 0);
        }

        _ => panic!(),
    }
}

#[test]
fn parse_xsettings() {
    let data = XSETTINGS
        .trim()
        .split(',')
        .map(|tok| {
            let val = tok.strip_prefix("0x").unwrap();
            u8::from_str_radix(val, 16).unwrap()
        })
        .collect::<Vec<_>>();

    let settings = read_settings(&data)
        .unwrap()
        .collect::<Result<Vec<_>>>()
        .unwrap();

    let dpi = settings.iter().find(|s| s.name == b"Xft/DPI").unwrap();
    assert_int(&dpi.data, 96 * 1024);
    let hinting = settings.iter().find(|s| s.name == b"Xft/Hinting").unwrap();
    assert_int(&hinting.data, 1);

    let rgba = settings.iter().find(|s| s.name == b"Xft/RGBA").unwrap();
    assert_string(&rgba.data, "rgb");
    let lcd = settings
        .iter()
        .find(|s| s.name == b"Xft/Lcdfilter")
        .unwrap();
    assert_string(&lcd.data, "lcddefault");
}

fn assert_string(dat: &SettingData<'_>, s: &str) {
    match dat {
        SettingData::String(left) => assert_eq!(*left, s.as_bytes()),
        _ => panic!("invalid data type"),
    }
}

fn assert_int(dat: &SettingData<'_>, i: i32) {
    match dat {
        SettingData::Integer(left) => assert_eq!(*left, i),
        _ => panic!("invalid data type"),
    }
}
