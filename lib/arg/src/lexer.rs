use nom::IResult;
use nom::Parser;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::space1;
use nom::character::complete::u64;
use nom::multi::many0;
use std::collections::HashMap;
use std::fmt::Debug;
use std::time::Duration;

pub type Span<'a> = nom_locate::LocatedSpan<&'a str>;

trait Token {
    fn token(&self) -> &'static str;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DSLKeywords {
    End,
    From,
    To,
}

impl Token for DSLKeywords {
    fn token(&self) -> &'static str {
        match self {
            Self::End => "end",
            Self::From => "from",
            Self::To => "to",
        }
    }
}

fn _parse<T>(token: T) -> Box<dyn Fn(Span) -> IResult<Span, T>>
where
    T: Token + Copy + 'static,
{
    Box::new(move |input: Span| {
        let (input, _) = tag(token.token())(input)?;
        Ok((input, token))
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum DSLType {
    FrameIndex(u64),
    Timestamp(Duration),
    Keyword(DSLKeywords),
}

pub fn parse_keyword(input: Span) -> IResult<Span, DSLType> {
    let (input, keyword) = alt((
        _parse(DSLKeywords::End),
        _parse(DSLKeywords::From),
        _parse(DSLKeywords::To),
    ))
    .parse(input)?;
    Ok((input, DSLType::Keyword(keyword)))
}

pub fn parse_frame_index(input: Span) -> IResult<Span, DSLType> {
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
                input,
                nom::error::ErrorKind::Count,
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
            input,
            nom::error::ErrorKind::Fail,
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

pub fn parse_timestamp3(input: Span) -> IResult<Span, DSLType> {
    let (input, value) = u64(input)?;
    Ok((
        tag("ms")(input)?.0,
        DSLType::Timestamp(Duration::from_millis(value)),
    ))
}

#[derive(Debug)]
#[allow(unused)]
pub struct DSLItem<T: Debug> {
    pub content: T,
    pub offset: usize,
    pub length: usize,
}

impl<T: Debug + PartialEq> PartialEq for DSLItem<T> {
    fn eq(&self, other: &Self) -> bool {
        self.content.eq(&other.content)
    }
}

impl<T: Debug + PartialEq> PartialEq<T> for DSLItem<T> {
    fn eq(&self, other: &T) -> bool {
        other.eq(&self.content)
    }
}

impl<T: Debug> DSLItem<T> {
    pub fn set(&mut self, content: T) {
        self.content = content;
    }
}

fn map_err(
    err: nom::Err<nom::error::Error<Span>>,
    offset: usize,
    kind: error::ParseErrorKind,
) -> nom::Err<error::ParseError<nom::error::Error<Span>>> {
    match err {
        nom::Err::Error(err) => nom::Err::Error(error::ParseError {
            kind,
            offset,
            length: err.input.location_offset() - offset,
            source: Box::new(err),
        }),
        nom::Err::Failure(err) => nom::Err::Failure(error::ParseError {
            kind,
            offset,
            length: err.input.location_offset() - offset,
            source: Box::new(err),
        }),
        nom::Err::Incomplete(need) => nom::Err::Incomplete(need),
    }
}

fn map_err_build(
    offset: usize,
) -> Box<
    dyn Fn(
        nom::Err<nom::error::Error<Span>>,
    ) -> nom::Err<error::ParseError<nom::error::Error<Span>>>,
> {
    Box::new(move |err| map_err(err, offset, error::ParseErrorKind::Nom))
}

fn map_err_build2(
    offset: usize,
    kind: error::ParseErrorKind,
) -> Box<
    dyn Fn(
        nom::Err<nom::error::Error<Span>>,
    ) -> nom::Err<error::ParseError<nom::error::Error<Span>>>,
> {
    Box::new(move |err| map_err(err, offset, kind))
}

pub fn parse_item(input: Span) -> error::ParseExprResult<Span, Option<DSLItem<DSLType>>> {
    let (input, _) = many0(space1)
        .parse(input)
        .map_err(map_err_build(input.location_offset()))?;
    if input.is_empty() {
        return Ok((input, None));
    }
    let offset = input.location_offset();
    match parse_timestamp2(input) {
        Ok((input, item)) => {
            return Ok((
                input,
                Some(DSLItem {
                    offset,
                    content: item,
                    length: input.location_offset() - offset,
                }),
            ));
        }
        Err(e) => match e {
            nom::Err::Failure(ref err) if err.code == nom::error::ErrorKind::Count => {
                return Err(map_err_build(input.location_offset())(e));
            }
            _ => {}
        },
    }

    let (input, item) =
        match alt((parse_frame_index, parse_timestamp1, parse_timestamp3)).parse(input) {
            Ok(res) => res,
            Err(e) => match e {
                nom::Err::Error(err) if err.code == nom::error::ErrorKind::Digit => {
                    parse_keyword(input).map_err(map_err_build(input.location_offset()))?
                }
                _ => return Err(map_err_build(input.location_offset())(e)),
            },
        };
    Ok((
        input,
        Some(DSLItem {
            offset,
            content: item,
            length: input.location_offset() - offset,
        }),
    ))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DSLOp {
    Add,
    Sub,
}

impl DSLOp {
    fn reversed(&self) -> Self {
        match self {
            Self::Add => Self::Sub,
            Self::Sub => Self::Add,
        }
    }
    fn reverse(&mut self) {
        *self = self.reversed();
    }
}

impl Token for DSLOp {
    fn token(&self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
        }
    }
}

pub fn parse_op(input: Span) -> error::ParseExprResult<Span, Option<DSLItem<DSLOp>>> {
    let (input, _) = many0(space1).parse(input).map_err(map_err_build2(
        input.location_offset(),
        error::ParseErrorKind::Op,
    ))?;
    if input.is_empty() {
        return Ok((input, None));
    }
    let offset = input.location_offset();
    let (input, op) = alt((_parse(DSLOp::Add), _parse(DSLOp::Sub)))
        .parse(input)
        .map_err(map_err_build2(
            input.location_offset(),
            error::ParseErrorKind::Op,
        ))?;
    Ok((
        input,
        Some(DSLItem {
            offset,
            content: op,
            length: input.location_offset() - offset,
        }),
    ))
}

#[derive(Debug, Default)]
pub struct Expr {
    pub items: Vec<DSLItem<DSLType>>,
    pub ops: Vec<DSLItem<DSLOp>>,
}

pub fn parse_expr(input: Span) -> error::ParseExprResult<Span, Expr> {
    let (mut input, Some(item)) = parse_item(input)? else {
        return Ok((input, Expr::default()));
    };
    let mut items = vec![item];
    let mut ops = vec![];
    while !input.is_empty() {
        let res = parse_op(input)?;
        let Some(op) = res.1 else {
            break;
        };
        input = res.0;
        let offset = op.offset;
        ops.push(op);

        let res = parse_item(input)?;
        let Some(item) = res.1 else {
            return Err(map_err_build(offset)(nom::Err::Failure(
                nom::error::Error::new(input, nom::error::ErrorKind::Escaped),
            )));
        };
        input = res.0;
        items.push(item);
    }
    Ok((input, Expr { items, ops }))
}

macro_rules! get {
    ($($name:ident)::*, $val:expr) => {
        match $val {
            $($name)::*(v) => v,
            _ => unreachable!(),
        }
    };
}

pub fn optimize_expr(expr: &mut Expr) {
    expr.ops.insert(
        0,
        DSLItem {
            content: DSLOp::Add,
            offset: 0,
            length: 0,
        },
    );
    if expr.items.len() < 2 {
        return;
    }
    let mut frame_index: Option<usize> = None;
    let mut time_index: Option<usize> = None;
    let mut index = 0;
    while index < expr.items.len() {
        match expr.items[index].content {
            DSLType::FrameIndex(this) => match frame_index {
                Some(first_index) => {
                    let first = get!(DSLType::FrameIndex, expr.items[first_index].content);
                    if expr.ops[first_index] == expr.ops[index] {
                        expr.items[first_index].set(DSLType::FrameIndex(first + this));
                    } else {
                        if first > this {
                            expr.items[first_index].set(DSLType::FrameIndex(first - this));
                        } else {
                            expr.ops[first_index].content.reverse();
                            expr.items[first_index].set(DSLType::FrameIndex(this - first));
                        }
                    }
                    expr.ops.remove(index);
                    expr.items.remove(index);
                    continue;
                }
                None => frame_index = Some(index),
            },
            DSLType::Timestamp(this) => match time_index {
                Some(first_index) => {
                    let first = get!(DSLType::Timestamp, expr.items[first_index].content);
                    if expr.ops[first_index] == expr.ops[index] {
                        expr.items[first_index].set(DSLType::Timestamp(first + this));
                    } else {
                        if first > this {
                            expr.items[first_index].set(DSLType::Timestamp(first - this));
                        } else {
                            expr.ops[first_index].content.reverse();
                            expr.items[first_index].set(DSLType::Timestamp(this - first));
                        }
                    }
                    expr.ops.remove(index);
                    expr.items.remove(index);
                    continue;
                }
                None => time_index = Some(index),
            },
            DSLType::Keyword(..) => {}
        }
        index += 1;
    }
}

#[derive(Debug)]
pub struct CheckedExpr {
    pub items: Vec<DSLType>,
    pub ops: Vec<DSLOp>,
}

pub fn check_expr(expr: &Expr) -> Result<CheckedExpr, String> {
    let mut counter = HashMap::<DSLKeywords, isize>::new();
    let mut has_add = false;
    for (item, op) in expr.items.iter().zip(expr.ops.iter()) {
        match item.content {
            DSLType::Keyword(word) => {
                if *op == DSLOp::Add {
                    *counter.entry(word).or_default() += 1;
                } else {
                    *counter.entry(word).or_default() -= 1;
                }
            }
            _ => {}
        }
        if *op == DSLOp::Add {
            has_add = true;
        }
    }
    if !has_add && !expr.ops.is_empty() {
        return Err("Overflow: all is sub".to_string());
    }
    if counter.values().any(|v| v.abs() > 1) {
        return Err("Too many keywords".to_string());
    }
    if counter.contains_key(&DSLKeywords::From) && counter.contains_key(&DSLKeywords::To) {
        return Err("circular references".to_string());
    }
    Ok(CheckedExpr {
        items: expr
            .items
            .iter()
            .map(|item| item.content.clone())
            .collect::<_>(),
        ops: expr.ops.iter().map(|item| item.content).collect::<_>(),
    })
}

pub mod error {
    use std::error::Error;
    use std::fmt::Formatter;

    #[derive(Debug, Clone, Copy)]
    pub enum ParseErrorKind {
        Nom,
        Op,
    }

    pub type ParseExprResult<I, O, E = ParseError<nom::error::Error<I>>> =
        Result<(I, O), nom::Err<E>>;

    #[derive(Debug)]
    pub struct ParseError<T>
    where
        T: Error,
    {
        pub offset: usize,
        pub length: usize,
        pub source: Box<T>,
        pub kind: ParseErrorKind,
    }

    impl<T> std::fmt::Display for ParseError<T>
    where
        T: Error,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "in 1:{}(length {}): {}",
                self.offset, self.offset, self.source
            )
        }
    }
    impl<T> Error for ParseError<T>
    where
        T: Error,
    {
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_parser() {
        let keywords = vec![
            ("end", DSLKeywords::End),
            ("from", DSLKeywords::From),
            ("to", DSLKeywords::To),
        ];
        for (word, keyword) in keywords {
            let (_, k) = parse_keyword(word.into()).unwrap();
            assert_eq!(DSLType::Keyword(keyword), k);
        }
        assert!(parse_keyword("hello".into()).is_err());
    }

    #[test]
    fn test_frame_parser() {
        let (_, val) = parse_frame_index("100f".into()).unwrap();
        match val {
            DSLType::FrameIndex(v) => assert_eq!(v, 100),
            _ => panic!("Error type"),
        }
        assert!(parse_frame_index("100".into()).is_err());
        assert!(parse_frame_index("100d".into()).is_err());
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

    #[test]
    fn test_timestamp_parser3() {
        let (_, val) = parse_timestamp3("100ms".into()).unwrap();
        match val {
            DSLType::Timestamp(v) => assert_eq!(v, Duration::from_millis(100)),
            _ => panic!("Error type"),
        }
        let (_, val) = parse_timestamp3("114514ms".into()).unwrap();
        match val {
            DSLType::Timestamp(v) => assert_eq!(v, Duration::from_millis(114514)),
            _ => panic!("Error type"),
        }
        assert!(parse_timestamp3("100.0ms".into()).is_err());
        assert!(parse_timestamp3("100d".into()).is_err());
    }

    #[test]
    fn test_item_parser() {
        let keywords = vec![
            ("end", DSLKeywords::End),
            ("from", DSLKeywords::From),
            ("to", DSLKeywords::To),
        ];
        for (word, keyword) in keywords {
            let (_, k) = parse_item(word.into()).unwrap();
            assert_eq!(DSLType::Keyword(keyword), k.unwrap().content);
        }
        let (_, val) = parse_item("100f".into()).unwrap();
        match val.unwrap().content {
            DSLType::FrameIndex(v) => assert_eq!(v, 100),
            _ => panic!("Error type"),
        }
        let (_, val) = parse_item("100.0s".into()).unwrap();
        match val.unwrap().content {
            DSLType::Timestamp(v) => assert_eq!(v, Duration::from_secs_f64(100f64)),
            _ => panic!("Error type"),
        }
        let (_, val) = parse_item("100.11s".into()).unwrap();
        match val.unwrap().content {
            DSLType::Timestamp(v) => assert_eq!(v, Duration::from_secs_f64(100.11)),
            _ => panic!("Error type"),
        }
        let (_, val) = parse_item("0:1".into()).unwrap();
        match val.unwrap().content {
            DSLType::Timestamp(v) => assert_eq!(v, Duration::from_secs(1)),
            _ => panic!("Error type"),
        }
        let (_, val) = parse_item("1:2".into()).unwrap();
        match val.unwrap().content {
            DSLType::Timestamp(v) => assert_eq!(v, Duration::from_secs(62)),
            _ => panic!("Error type"),
        }
        let (_, val) = parse_item("1:2:3".into()).unwrap();
        match val.unwrap().content {
            DSLType::Timestamp(v) => assert_eq!(v, Duration::from_secs(3723)),
            _ => panic!("Error type"),
        }
        let (_, val) = parse_item("1:2:3.4".into()).unwrap();
        match val.unwrap().content {
            DSLType::Timestamp(v) => {
                assert_eq!(v, Duration::from_secs(3723) + Duration::from_millis(400))
            }
            _ => panic!("Error type"),
        }
        let (_, val) = parse_item("1.4".into()).unwrap();
        match val.unwrap().content {
            DSLType::Timestamp(v) => {
                assert_eq!(v, Duration::from_secs(1) + Duration::from_millis(400))
            }
            _ => panic!("Error type"),
        }
        let (_, val) = parse_item("100ms".into()).unwrap();
        match val.unwrap().content {
            DSLType::Timestamp(v) => assert_eq!(v, Duration::from_millis(100)),
            _ => panic!("Error type"),
        }
        let (_, val) = parse_item("114514ms".into()).unwrap();
        match val.unwrap().content {
            DSLType::Timestamp(v) => assert_eq!(v, Duration::from_millis(114514)),
            _ => panic!("Error type"),
        }

        assert!(parse_item("hello".into()).is_err());
        assert!(parse_item("100".into()).is_err());
        assert!(parse_item("100d".into()).is_err());
        assert!(parse_item("1:2:3:4".into()).is_err());
    }

    #[test]
    fn test_expr_parser() {
        let (_, expr) = parse_expr("end + from - to + 1f - 2s + 3ms - 4:5".into()).unwrap();
        let items = vec![
            DSLType::Keyword(DSLKeywords::End),
            DSLType::Keyword(DSLKeywords::From),
            DSLType::Keyword(DSLKeywords::To),
            DSLType::FrameIndex(1),
            DSLType::Timestamp(Duration::from_secs_f64(2f64)),
            DSLType::Timestamp(Duration::from_millis(3)),
            DSLType::Timestamp(Duration::from_secs(245)),
        ];
        for (item, expr_item) in items.iter().zip(expr.items.iter()) {
            assert_eq!(expr_item, item);
        }
        assert_eq!(
            expr.ops,
            vec![
                DSLOp::Add,
                DSLOp::Sub,
                DSLOp::Add,
                DSLOp::Sub,
                DSLOp::Add,
                DSLOp::Sub,
            ]
        );
        assert!(parse_expr("++".into()).is_err());
    }

    #[test]
    fn test_expr_opt() {
        // end + from - to + 1f - 246.997s
        let (_, mut expr) = parse_expr("end + from - to + 1f - 2s + 3ms - 4:5".into()).unwrap();
        optimize_expr(&mut expr);
        let items = vec![
            DSLType::Keyword(DSLKeywords::End),
            DSLType::Keyword(DSLKeywords::From),
            DSLType::Keyword(DSLKeywords::To),
            DSLType::FrameIndex(1),
            DSLType::Timestamp(Duration::from_secs(247) - Duration::from_millis(3)),
        ];
        for (item, expr_item) in items.iter().zip(expr.items.iter()) {
            assert_eq!(expr_item, item);
        }
        assert_eq!(
            expr.ops,
            vec![DSLOp::Add, DSLOp::Add, DSLOp::Sub, DSLOp::Add, DSLOp::Sub,]
        );
    }
}
