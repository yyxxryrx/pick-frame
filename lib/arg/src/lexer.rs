use nom::IResult;
use nom::bytes::complete::tag;
use nom::character::complete::u64;
use std::time::Duration;

type Span<'a> = nom_locate::LocatedSpan<&'a str>;

pub enum DSLType {
    FrameIndex(u64),
    Timestamp(Duration),
}

pub fn parse_frame_index1(input: Span) -> IResult<Span, DSLType> {
    u64(input).map(|(s, v)| (s, DSLType::FrameIndex(v)))
}

pub fn parse_frame_index2(input: Span) -> IResult<Span, DSLType> {
    let (input, value) = u64(input)?;
    Ok((tag("f")(input)?.0, DSLType::FrameIndex(value)))
}

fn parse_f64(input: Span) -> IResult<Span, f64> {
    let (input, integer) = u64(input)?;
    match tag::<&str, Span, nom::error::Error<Span>>(".")(input) {
        Ok((input, _)) => {
            let (input, decimal) = u64(input)?;
            Ok((
                input,
                format!("{integer}.{decimal}")
                    .parse::<f64>()
                    .unwrap_or_default(),
            ))
        }
        Err(..) => Ok((input, integer as f64)),
    }
}

pub fn parse_timestamp1(input: Span) -> IResult<Span, DSLType> {
    let (input, value) = parse_f64(input)?;
    Ok((
        tag("s")(input)?.0,
        DSLType::Timestamp(Duration::from_secs_f64(value)),
    ))
}

pub fn parse_timestamp2(input: Span) -> IResult<Span, DSLType> {
    let (mut input, value) = u64(input)?;
    let mut times = vec![value];
    let mut ms: Option<u64> = None;
    let mut i = 0;
    loop {
        if i > 2 {
            return Err(nom::Err::Failure(nom::error::Error::new(
                Span::new("Too many args"),
                nom::error::ErrorKind::TooLarge,
            )));
        }
        match tag::<&str, Span, nom::error::Error<Span>>(":")(input) {
            Ok(res) => {
                input = res.0;
                let res = u64(input)?;
                input = res.0;
                times.push(res.1);
                i += 1;
            }
            Err(..) => {
                let Ok(res) = tag::<&str, Span, nom::error::Error<Span>>(".")(input) else {
                    break;
                };
                let res = u64(res.0)?;
                input = res.0;
                ms = Some(res.1);
                break;
            }
        }
    }
    let len = times.len();
    if len < 2 && ms.is_none() {
        return Err(nom::Err::Failure(nom::error::Error::new(
            "only one number".into(),
            nom::error::ErrorKind::Count,
        )));
    }
    let secs = times.iter().enumerate().fold(0u64, |acc, (index, value)| {
        acc + *value * 60u64.pow((len - index - 1) as u32)
    });
    let sec = Duration::from_secs(secs);
    let time = sec
        + ms.map(|ms| {
            let bit = ms.ilog10() + 1;
            match bit.checked_sub(3) {
                Some(0) => Duration::from_millis(ms),
                None => Duration::from_millis(ms * 10u64.pow(3 - bit)),
                Some(v) => Duration::from_millis((ms as f64 / 10f64.powi(v as i32)).round() as u64),
            }
        })
        .unwrap_or_default();
    Ok((input, DSLType::Timestamp(time)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_parse1() {
        let (_, val) = parse_frame_index1("100".into()).unwrap();
        match val {
            DSLType::FrameIndex(v) => assert_eq!(v, 100),
            _ => panic!("Error type"),
        }
        assert!(parse_frame_index1("a".into()).is_err())
    }

    #[test]
    fn test_frame_parse2() {
        let (_, val) = parse_frame_index2("100f".into()).unwrap();
        match val {
            DSLType::FrameIndex(v) => assert_eq!(v, 100),
            _ => panic!("Error type"),
        }
        assert!(parse_frame_index2("100".into()).is_err());
        assert!(parse_frame_index2("100d".into()).is_err());
    }

    #[test]
    fn test_parse_f64() {
        let (input, val) = parse_f64("114.15s".into()).unwrap();
        assert_eq!(val, 114.15);
        assert_eq!(input.to_string(), "s".to_string());
        let (input, val) = parse_f64("11415s".into()).unwrap();
        assert_eq!(val, 11415f64);
        assert_eq!(input.to_string(), "s".to_string());
    }

    #[test]
    fn test_timestamp_parser1() {
        let (_, val) = parse_timestamp1("100.0s".into()).unwrap();
        match val {
            DSLType::Timestamp(v) => assert_eq!(v, Duration::from_secs_f64(100f64)),
            _ => panic!("Error type"),
        }
        let (_, val) = parse_timestamp1("100.11s".into()).unwrap();
        match val {
            DSLType::Timestamp(v) => assert_eq!(v, Duration::from_secs_f64(100.11)),
            _ => panic!("Error type"),
        }
        assert!(parse_timestamp1("100".into()).is_err());
        assert!(parse_timestamp1("100d".into()).is_err());
    }

    #[test]
    fn test_timestamp_parser2() {
        let (_, val) = parse_timestamp2("0:1".into()).unwrap();
        match val {
            DSLType::Timestamp(v) => assert_eq!(v, Duration::from_secs(1)),
            _ => panic!("Error type"),
        }
        let (_, val) = parse_timestamp2("1:2".into()).unwrap();
        match val {
            DSLType::Timestamp(v) => assert_eq!(v, Duration::from_secs(62)),
            _ => panic!("Error type"),
        }
        let (_, val) = parse_timestamp2("1:2:3".into()).unwrap();
        match val {
            DSLType::Timestamp(v) => assert_eq!(v, Duration::from_secs(3723)),
            _ => panic!("Error type"),
        }
        let (_, val) = parse_timestamp2("1:2:3.4".into()).unwrap();
        match val {
            DSLType::Timestamp(v) => {
                assert_eq!(v, Duration::from_secs(3723) + Duration::from_millis(400))
            }
            _ => panic!("Error type"),
        }
        let (_, val) = parse_timestamp2("1.4".into()).unwrap();
        match val {
            DSLType::Timestamp(v) => {
                assert_eq!(v, Duration::from_secs(1) + Duration::from_millis(400))
            }
            _ => panic!("Error type"),
        }
        assert!(parse_timestamp2("100".into()).is_err());
        assert!(parse_timestamp2("1:2:3:4".into()).is_err());
    }
}
