//! # DSL词法分析器
//!
//! 这个模块提供了一个用于解析特定领域语言（DSL）的词法分析器。
//! DSL语言支持以下元素：
//! - 关键字（end, from, to）
//! - 帧索引（如 100f）
//! - 时间戳（如 100s, 1:2:3, 100ms）
//! - 操作符（+, -）
//!
//! 该分析器使用nom库进行解析，并包含表达式优化和验证功能。

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

/// 用于跟踪输入字符串位置的span类型，包含行号和列号信息
pub type Span<'a> = nom_locate::LocatedSpan<&'a str>;

trait Token {
    fn token(&self) -> &'static str;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
/// DSL中的关键字枚举
///
/// 支持的关键字包括:
/// - `End`: 表示结束
/// - `From`: 表示起始
/// - `To`: 表示目标
pub enum DSLKeywords {
    /// 结束关键字
    End,
    /// 起始关键字
    From,
    /// 目标关键字
    To,
}

impl Token for DSLKeywords {
    /// 返回关键字的字符串表示
    fn token(&self) -> &'static str {
        match self {
            Self::End => "end",
            Self::From => "from",
            Self::To => "to",
        }
    }
}

/// 创建一个解析指定标记的解析器函数
///
/// # 参数
/// * `token` - 需要解析的标记
///
/// # 返回值
/// 返回一个解析函数，该函数尝试匹配输入中的标记
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
/// DSL中支持的数据类型枚举
///
/// 包括帧索引、时间戳和关键字三种基本类型
pub enum DSLType {
    /// 帧索引，以f结尾，例如 100f
    FrameIndex(u64),
    /// 时间戳，可以是秒、毫秒或时:分:秒格式
    Timestamp(Duration),
    /// 关键字
    Keyword(DSLKeywords),
}

/// 解析DSL中的关键字
///
/// # 参数
/// * `input` - 输入的span
///
/// # 返回值
/// 返回解析结果，包含剩余输入和解析出的关键字
pub fn parse_keyword(input: Span) -> IResult<Span, DSLType> {
    let (input, keyword) = alt((
        _parse(DSLKeywords::End),
        _parse(DSLKeywords::From),
        _parse(DSLKeywords::To),
    ))
    .parse(input)?;
    Ok((input, DSLType::Keyword(keyword)))
}

/// 解析帧索引
///
/// 帧索引格式为数字后跟字母f，例如 100f
///
/// # 参数
/// * `input` - 输入的span
///
/// # 返回值
/// 返回解析结果，包含剩余输入和解析出的帧索引
pub fn parse_frame_index(input: Span) -> IResult<Span, DSLType> {
    let (input, value) = u64(input)?;
    Ok((tag("f")(input)?.0, DSLType::FrameIndex(value)))
}

/// 解析浮点数
///
/// 尝试解析整数或小数形式的数值
///
/// # 参数
/// * `input` - 输入的span
///
/// # 返回值
/// 返回解析结果，包含剩余输入和解析出的f64值
fn parse_f64(input: Span) -> IResult<Span, f64> {
    let (input, integer) = u64(input)?;
    match tag::<&str, Span, nom::error::Error<Span>>(".")(input) {
        Ok((input, _)) => {
            let (input, decimal) = nom::character::complete::digit1(input)?;
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

/// 解析秒级时间戳
///
/// 格式为数字后跟字母s，例如 100s 或 100.5s
///
/// # 参数
/// * `input` - 输入的span
///
/// # 返回值
/// 返回解析结果，包含剩余输入和解析出的时间戳
pub fn parse_timestamp1(input: Span) -> IResult<Span, DSLType> {
    let (input, value) = parse_f64(input)?;
    Ok((
        tag("s")(input)?.0,
        DSLType::Timestamp(Duration::from_secs_f64(value)),
    ))
}

/// 解析时:分:秒格式的时间戳
///
/// 支持格式如: 1:2, 1:2:3, 1:2.5 等
///
/// # 参数
/// * `input` - 输入的span
///
/// # 返回值
/// 返回解析结果，包含剩余输入和解析出的时间戳
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
                let res = nom::character::complete::digit1(res.0)?;
                input = res.0;
                println!(
                    "{}{}",
                    res.1,
                    "0".repeat(3usize.saturating_sub(res.1.len()))
                );
                ms = format!(
                    "{}{}",
                    res.1,
                    "0".repeat(3usize.saturating_sub(res.1.len()))
                )
                .parse::<u64>()
                .map(Some)
                .unwrap_or_default();
                println!("ms: {ms:?}");
                break;
            }
        }
    }
    let len = times.len();
    if len < 2 {
        return Err(nom::Err::Failure(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Fail,
        )));
    }
    let secs = times.iter().enumerate().fold(0u64, |acc, (index, value)| {
        acc + *value * 60u64.pow((len - index - 1) as u32)
    });
    let sec = Duration::from_secs(secs);
    let time = sec + ms.map(Duration::from_millis).unwrap_or_default();
    Ok((input, DSLType::Timestamp(time)))
}

/// 解析毫秒级时间戳
///
/// 格式为数字后跟ms，例如 100ms
///
/// # 参数
/// * `input` - 输入的span
///
/// # 返回值
/// 返回解析结果，包含剩余输入和解析出的时间戳
pub fn parse_timestamp3(input: Span) -> IResult<Span, DSLType> {
    let (input, value) = u64(input)?;
    Ok((
        tag("ms")(input)?.0,
        DSLType::Timestamp(Duration::from_millis(value)),
    ))
}

#[derive(Debug)]
#[allow(unused)]
/// 表示DSL中的一个项目，包含内容、偏移量和长度信息
///
/// 泛型参数T代表项目的实际内容类型
pub struct DSLItem<T: Debug> {
    /// 项目的实际内容
    pub content: T,
    /// 项目在源字符串中的偏移量
    pub offset: usize,
    /// 项目的长度
    pub length: usize,
}

impl<T: Debug + PartialEq> PartialEq for DSLItem<T> {
    /// 比较两个DSLItem是否相等，只比较内容部分
    fn eq(&self, other: &Self) -> bool {
        self.content.eq(&other.content)
    }
}

impl<T: Debug + PartialEq> PartialEq<T> for DSLItem<T> {
    /// 比较DSLItem的内容与另一个值是否相等
    fn eq(&self, other: &T) -> bool {
        other.eq(&self.content)
    }
}

impl<T: Debug> DSLItem<T> {
    /// 设置DSLItem的内容
    ///
    /// # 参数
    /// * `content` - 新的内容
    pub fn set(&mut self, content: T) {
        self.content = content;
    }
}

/// 将nom错误转换为自定义解析错误
///
/// # 参数
/// * `err` - 原始的nom错误
/// * `offset` - 错误发生的位置偏移
/// * `kind` - 错误类型
///
/// # 返回值
/// 转换后的自定义解析错误
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

/// 创建一个错误映射函数，用于将nom错误转换为自定义错误
///
/// # 参数
/// * `offset` - 错误发生的位置偏移
///
/// # 返回值
/// 返回一个错误转换函数
fn map_err_build(
    offset: usize,
) -> Box<
    dyn Fn(
        nom::Err<nom::error::Error<Span>>,
    ) -> nom::Err<error::ParseError<nom::error::Error<Span>>>,
> {
    Box::new(move |err| map_err(err, offset, error::ParseErrorKind::Nom))
}

/// 创建一个错误映射函数，用于将nom错误转换为指定类型的自定义错误
///
/// # 参数
/// * `offset` - 错误发生的位置偏移
/// * `kind` - 错误类型
///
/// # 返回值
/// 返回一个错误转换函数
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

/// 解析单个DSL项
///
/// 尝试解析各种类型的DSL项，包括关键字、帧索引和时间戳
///
/// # 参数
/// * `input` - 输入的span
///
/// # 返回值
/// 返回解析结果，包含剩余输入和解析出的DSL项（如果存在）
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
                    parse_keyword(input).map_err(map_err_build2(
                        input.location_offset(),
                        error::ParseErrorKind::Keywords,
                    ))?
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
/// DSL中的操作符枚举
///
/// 支持加法和减法两种操作符
pub enum DSLOp {
    /// 加法操作符 (+)
    Add,
    /// 减法操作符 (-)
    Sub,
}

impl DSLOp {
    /// 获取相反的操作符
    ///
    /// # 返回值
    /// 如果当前是Add则返回Sub，如果是Sub则返回Add
    fn reversed(&self) -> Self {
        match self {
            Self::Add => Self::Sub,
            Self::Sub => Self::Add,
        }
    }
    /// 反转当前操作符
    fn reverse(&mut self) {
        *self = self.reversed();
    }
}

impl Token for DSLOp {
    /// 返回操作符的字符串表示
    fn token(&self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
        }
    }
}

/// 解析DSL中的操作符
///
/// 尝试解析加法(+)或减法(-)操作符
///
/// # 参数
/// * `input` - 输入的span
///
/// # 返回值
/// 返回解析结果，包含剩余输入和解析出的操作符（如果存在）
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
/// 表示完整的DSL表达式
///
/// 包含项列表和操作符列表
pub struct Expr {
    /// 表达式中的项列表
    pub items: Vec<DSLItem<DSLType>>,
    /// 表达式中的操作符列表
    pub ops: Vec<DSLItem<DSLOp>>,
}

/// 解析完整的DSL表达式
///
/// 表达式由项和操作符交替组成，例如: end + from - 100f + 5s
///
/// # 参数
/// * `input` - 输入的span
///
/// # 返回值
/// 返回解析结果，包含剩余输入和解析出的表达式
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

/// 安全地从枚举中提取值的宏
///
/// 假设输入值一定是指定的变体，否则会导致未定义行为
///
/// # 参数
/// * `$($name:ident)::` - 枚举变体的路径
/// * `$val:expr` - 要提取值的表达式
macro_rules! get {
    ($($name:ident)::*, $val:expr) => {
        match $val {
            $($name)::*(v) => v,
            _ => unreachable!(),
        }
    };
}

/// 优化DSL表达式
///
/// 合并相同类型的项（帧索引与帧索引，时间戳与时间戳），简化表达式
///
/// # 参数
/// * `expr` - 需要优化的表达式引用
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
/// 经过验证的DSL表达式
///
/// 仅包含类型，不包含位置信息
pub struct CheckedExpr {
    /// 表达式中的项列表
    pub items: Vec<DSLType>,
    /// 表达式中的操作符列表
    pub ops: Vec<DSLOp>,
}

/// 验证DSL表达式的语义正确性
///
/// 检查表达式是否符合语义规则，例如关键字的使用次数等
///
/// # 参数
/// * `expr` - 需要验证的表达式引用
///
/// # 返回值
/// 验证成功返回CheckedExpr，失败返回错误信息
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

/// 解析错误处理模块
///
/// 提供了自定义的解析错误类型和相关工具
pub mod error {
    use std::error::Error;
    use std::fmt::Formatter;

    #[derive(Debug, Clone, Copy, PartialEq)]
    /// 解析错误的种类
    pub enum ParseErrorKind {
        /// 来自nom库的基本解析错误
        Nom,
        /// 操作符相关的解析错误
        Op,
        /// 关键字相关的解析错误
        Keywords,
    }

    /// 解析表达式的返回类型
    pub type ParseExprResult<I, O, E = ParseError<nom::error::Error<I>>> =
        Result<(I, O), nom::Err<E>>;

    #[derive(Debug)]
    /// 自定义解析错误类型
    ///
    /// 包含错误位置信息和原始错误
    pub struct ParseError<T>
    where
        T: Error,
    {
        /// 错误在输入中的偏移量
        pub offset: usize,
        /// 错误的长度
        pub length: usize,
        /// 源错误
        pub source: Box<T>,
        /// 错误类型
        pub kind: ParseErrorKind,
    }

    impl<T> std::fmt::Display for ParseError<T>
    where
        T: Error,
    {
        /// 格式化错误信息
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "in 1:{}(length {}): {}",
                self.offset, self.offset, self.source
            )
        }
    }
    impl<T> Error for ParseError<T> where T: Error {}
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
