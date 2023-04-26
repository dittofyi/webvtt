use std::time::Duration;

use crate::{parse_timestamp, parse_file};

#[test]
fn timestamp() {
    let line = "00:31.500";
    let result = parse_timestamp(line);
    assert_eq!(result, Some((Duration::from_millis(31_500), "")));

    let line = "2:31.500";
    let result = parse_timestamp(line);
    assert_eq!(result, None);

    let line = "02:31.500";
    let result = parse_timestamp(line);
    assert_eq!(
        result,
        Some((Duration::from_millis(2 * 60_000 + 31_500), ""))
    );

    let line = "02:31.500 -> 03:31.500";
    let result = parse_timestamp(line);
    assert_eq!(
        result,
        Some((Duration::from_millis(2 * 60_000 + 31_500), " -> 03:31.500"))
    );

    let line = "1:02:31.500";
    let result = parse_timestamp(line);
    assert_eq!(
        result,
        Some((
            Duration::from_millis(1 * 3600_000 + 2 * 60_000 + 31_500),
            ""
        ))
    );

    let line = "11:02:31.500";
    let result = parse_timestamp(line);
    assert_eq!(
        result,
        Some((
            Duration::from_millis(11 * 3600_000 + 2 * 60_000 + 31_500),
            ""
        ))
    );

    let line = "111:02:31.500";
    let result = parse_timestamp(line);
    assert_eq!(
        result,
        Some((
            Duration::from_millis(111 * 3600_000 + 2 * 60_000 + 31_500),
            ""
        ))
    );

    let line = "11:11:02:31.500";
    let result = parse_timestamp(line);
    assert_eq!(result, None);

    let line = "111:11:02:31.500";
    let result = parse_timestamp(line);
    assert_eq!(result, None);

    let line = "02:02:31.5001";
    let result = parse_timestamp(line);
    assert_eq!(result, None);

    let line = "02:31.5001";
    let result = parse_timestamp(line);
    assert_eq!(result, None);
}

#[test]
fn sample1() {
  let sample1 = include_str!("../test/sample1.vtt");
  let file = parse_file(sample1).unwrap();
  println!("{file:#?}");
}

#[test]
fn sample2() {
  let sample = include_str!("../test/sample2.vtt");
  let file = parse_file(sample).unwrap();
  println!("{file:#?}");
}
