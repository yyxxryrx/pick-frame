#[cfg(feature = "dsl")]
mod lexer;
#[cfg(feature = "dsl")]
mod tui;

use clap::Parser;
use std::{ffi::CString, os::raw::c_char, time::Duration};

const AV_NOPTS_VALUE: i64 = i64::MIN;

#[unsafe(no_mangle)]
pub extern "C" fn create_video_info(
    fps: f64,
    time_base_den: i64,
    time_base_num: i64,
    start_time: i64,
    duration: i64,
) -> *mut VideoInfo {
    Box::into_raw(Box::new(VideoInfo {
        fps,
        duration,
        start_time,
        time_base_den,
        time_base_num,
    }))
}

#[unsafe(no_mangle)]
pub extern "C" fn free_video_info(info: *mut VideoInfo) {
    if info.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(info);
    }
}

#[derive(Debug, Copy, Clone)]
pub struct VideoInfo {
    pub fps: f64,
    pub time_base_den: i64,
    pub time_base_num: i64,
    pub start_time: i64,
    pub duration: i64,
}

impl VideoInfo {
    pub fn frame_to_timestamp(&self, frame_index: u64) -> i64 {
        let seconds = frame_index as f64 / self.fps;
        let tb_val = self.time_base_num as f64 / self.time_base_den as f64;
        let mut target_ts = (seconds / tb_val) as i64;
        if self.start_time != AV_NOPTS_VALUE {
            target_ts += self.start_time;
        }
        target_ts
    }

    pub fn milliseconds_to_timestamp(&self, ms: u64) -> i64 {
        let seconds = ms as f64 / 1000f64;
        let tb_val = self.time_base_num as f64 / self.time_base_den as f64;
        let mut target_ts = (seconds / tb_val) as i64;
        if self.start_time != AV_NOPTS_VALUE {
            target_ts += self.start_time;
        }
        target_ts
    }

    pub fn end_to_timestamp(&self) -> i64 {
        self.duration
    }
}

#[repr(C)]
#[derive(Debug)]
pub enum TimeTypeKind {
    Frame = 0,
    Millisecond = 1,
    End = 2,
}

impl Default for TimeTypeKind {
    fn default() -> Self {
        Self::Millisecond
    }
}

#[derive(Debug, Default)]
pub struct PaserTimeType {
    pub kind: TimeTypeKind,
    pub value: u64,
}

pub struct ArgParseResultContext {
    pub input: *const c_char,
    pub output: *const c_char,
    pub thread_count: u16,
    pub format: *const c_char,

    start: TimeType,
    end: TimeType,
}

enum TimeType {
    Parser(PaserTimeType),
    #[cfg(feature = "dsl")]
    DSL(lexer::CheckedExpr),
}

#[derive(Debug, Clone, Copy)]
enum Time {
    Frame(u64),
    Time(Duration),
    End,
}

impl std::str::FromStr for Time {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.to_lowercase() == "end" {
            return Ok(Self::End);
        }
        if let Ok(frame) = s.parse::<u64>() {
            return Ok(Self::Frame(frame));
        }
        if s.ends_with('s') {
            let sub = s.chars().take(s.len() - 1).collect::<String>();
            let Ok(v) = sub.parse::<f64>() else {
                return Err(format!("Wrong second format: '{sub}'"));
            };
            return Ok(Self::Time(Duration::from_secs_f64(v)));
        }
        let segments = s.split(':').collect::<Vec<_>>();
        if segments.len() > 3 || segments.len() < 2 {
            return Err("Wrong time format".to_string());
        }
        let mut segs = segments.iter();
        let hour = if segments.len() == 3 {
            segs.next()
                .unwrap()
                .parse::<u64>()
                .map_err(|err| err.to_string())?
        } else {
            0
        };
        let min = segs
            .next()
            .unwrap()
            .parse::<u64>()
            .map_err(|err| err.to_string())?;
        let mut secs = segs.next().unwrap().split('.');
        let sec = secs
            .next()
            .unwrap()
            .parse::<u64>()
            .map_err(|err| err.to_string())?;
        let mm = if let Some(mm) = secs.next() {
            let a = format!("{mm:0<3}");
            if a.len() > 3 {
                return Err("millis rank must less than 4".to_string());
            }
            a.parse::<u64>().map_err(|err| err.to_string())?
        } else {
            0
        };
        let sec = Duration::from_secs(
            hour.saturating_mul(3600)
                .saturating_add(min.saturating_mul(60))
                .saturating_add(sec),
        );
        let mm = Duration::from_millis(mm);
        Ok(Self::Time(sec.saturating_add(mm)))
    }
}

impl From<Time> for PaserTimeType {
    fn from(value: Time) -> Self {
        match value {
            Time::Time(t) => Self {
                kind: TimeTypeKind::Millisecond,
                value: t.as_millis() as u64,
            },
            Time::Frame(f) => Self {
                kind: TimeTypeKind::Frame,
                value: f,
            },
            Time::End => Self {
                kind: TimeTypeKind::End,
                value: 0,
            },
        }
    }
}

impl From<Time> for TimeType {
    fn from(value: Time) -> Self {
        Self::Parser(value.into())
    }
}

#[derive(Debug, Clone)]
enum ThreadCount {
    Auto,
    Custom(u16),
}

impl From<ThreadCount> for u16 {
    fn from(value: ThreadCount) -> Self {
        match value {
            ThreadCount::Auto => 0,
            ThreadCount::Custom(v) => v,
        }
    }
}

impl std::str::FromStr for ThreadCount {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("auto") {
            Ok(Self::Auto)
        } else {
            s.parse::<u16>()
                .map(Self::Custom)
                .map_err(|err| err.to_string())
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    about = "A simple video frame picker\n\nTips:\n\t`xxx` is frame index\n\t`xx:xx.xx` is timestamp\n\t`end` is the end of video\n\t`xx.xxs` is seconds-base timestamp"
)]
struct Cli {
    #[arg(short, long, help = "The video path")]
    input: String,
    #[cfg(feature = "dsl")]
    #[arg(
        short,
        long,
        value_name = "expr",
        help = "time expression",
        default_value = "0"
    )]
    from: String,
    #[cfg(not(feature = "dsl"))]
    #[arg(
        short,
        long,
        help = "possible format: [xxx, xx.xxs, xx:xx.xx, end]",
        default_value = "0"
    )]
    from: Time,
    #[cfg(feature = "dsl")]
    #[arg(
        short,
        long,
        value_name = "expr",
        help = "time expression",
        default_value = "end"
    )]
    to: String,
    #[cfg(not(feature = "dsl"))]
    #[arg(
        short,
        long,
        help = "possible format: [xxx, xx.xxs, xx:xx.xx, end]",
        default_value = "end"
    )]
    to: Time,
    #[arg(
        long,
        value_name = "Auto|num",
        help = "thread count for codec",
        default_value = "auto"
    )]
    thread_count: ThreadCount,
    #[arg(long, help = "filename format", default_value = "frame-%d.jpg")]
    format: String,
    #[arg(help = "Output path", default_value = ".")]
    output: String,
}

#[cfg(feature = "dsl")]
macro_rules! err {
    ($info:expr) => {{
        println!("{} {}", "error:".bright_red(), $info);
        std::process::exit(1);
    }};
    ($info:expr, $code:literal) => {{
        use colored::Colorize;
        println!("{} {}", "error:".bright_red(), $info);
        std::process::exit($code);
    }};
}

#[unsafe(no_mangle)]
pub extern "C" fn parse() -> *mut ArgParseResultContext {
    let cli = Cli::parse();
    #[cfg(feature = "dsl")]
    {
        let (_, mut from_expr) = tui::handle_error(
            &cli.from,
            "from",
            lexer::parse_expr(cli.from.as_str().into()),
        );
        lexer::optimize_expr(&mut from_expr);
        let from_expr = lexer::check_expr(&from_expr)
            .map_err(|err| err!(err, 2))
            .unwrap();

        let (_, mut to_expr) =
            tui::handle_error(&cli.to, "to", lexer::parse_expr(cli.to.as_str().into()));
        lexer::optimize_expr(&mut to_expr);
        let to_expr = lexer::check_expr(&to_expr)
            .map_err(|err| err!(err, 2))
            .unwrap();

        let ref_to = from_expr.items.iter().any(|item| match item {
            lexer::DSLType::Keyword(lexer::DSLKeywords::To) => true,
            _ => false,
        });
        let ref_from = to_expr.items.iter().any(|item| match item {
            lexer::DSLType::Keyword(lexer::DSLKeywords::From) => true,
            _ => false,
        });
        if ref_from && ref_to {
            err!(
                "circular references, arg from ref `to` and arg to ref `from`".bright_white(),
                2
            );
        }

        Box::into_raw(Box::new(ArgParseResultContext {
            input: CString::new(cli.input).unwrap_or_default().into_raw(),
            output: CString::new(cli.output).unwrap_or_default().into_raw(),
            format: CString::new(cli.format).unwrap_or_default().into_raw(),
            thread_count: cli.thread_count.into(),
            start: TimeType::DSL(from_expr),
            end: TimeType::DSL(to_expr),
        }))
    }
    #[cfg(not(feature = "dsl"))]
    Box::into_raw(Box::new(ArgParseResultContext {
        input: CString::new(cli.input).unwrap_or_default().into_raw(),
        output: CString::new(cli.output).unwrap_or_default().into_raw(),
        start: cli.from.into(),
        end: cli.to.into(),
        thread_count: cli.thread_count.into(),
        format: CString::new(cli.format).unwrap_or_default().into_raw(),
    }))
}

#[unsafe(no_mangle)]
pub extern "C" fn get_input(res_ctx: &ArgParseResultContext) -> *const c_char {
    res_ctx.input
}

#[unsafe(no_mangle)]
pub extern "C" fn get_output(res_ctx: &ArgParseResultContext) -> *const c_char {
    res_ctx.output
}

#[unsafe(no_mangle)]
pub extern "C" fn get_thread_count(res_ctx: &ArgParseResultContext) -> u16 {
    res_ctx.thread_count
}

#[unsafe(no_mangle)]
pub extern "C" fn get_format(res_ctx: &ArgParseResultContext) -> *const c_char {
    res_ctx.format
}

#[unsafe(no_mangle)]
pub extern "C" fn get_from_timestamp(res_ctx: &ArgParseResultContext, info: &VideoInfo) -> i64 {
    match res_ctx.start {
        TimeType::Parser(ref per) => match per.kind {
            TimeTypeKind::End => info.end_to_timestamp(),
            TimeTypeKind::Frame => info.frame_to_timestamp(per.value),
            TimeTypeKind::Millisecond => info.milliseconds_to_timestamp(per.value),
        },
        #[cfg(feature = "dsl")]
        TimeType::DSL(ref expr) => {
            let mut pts = 0i64;
            for (op, item) in expr.ops.iter().zip(expr.items.iter()) {
                let item = match item {
                    lexer::DSLType::Keyword(keyword) => match keyword {
                        lexer::DSLKeywords::To => get_to_timestamp(res_ctx, info),
                        lexer::DSLKeywords::End => info.end_to_timestamp(),
                        _ => unreachable!(),
                    },
                    lexer::DSLType::FrameIndex(index) => info.frame_to_timestamp(*index),
                    lexer::DSLType::Timestamp(dur) => {
                        info.milliseconds_to_timestamp(dur.as_millis() as u64)
                    }
                };
                match op {
                    lexer::DSLOp::Add => {
                        pts += item;
                    }
                    lexer::DSLOp::Sub => {
                        pts -= item;
                    }
                }
            }
            pts
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_to_timestamp(res_ctx: &ArgParseResultContext, info: &VideoInfo) -> i64 {
    match res_ctx.end {
        TimeType::Parser(ref per) => match per.kind {
            TimeTypeKind::End => info.end_to_timestamp(),
            TimeTypeKind::Frame => info.frame_to_timestamp(per.value),
            TimeTypeKind::Millisecond => info.milliseconds_to_timestamp(per.value),
        },
        #[cfg(feature = "dsl")]
        TimeType::DSL(ref expr) => {
            let mut pts = 0i64;
            for (op, item) in expr.ops.iter().zip(expr.items.iter()) {
                let item = match item {
                    lexer::DSLType::Keyword(keyword) => match keyword {
                        lexer::DSLKeywords::From => get_from_timestamp(res_ctx, info),
                        lexer::DSLKeywords::End => info.end_to_timestamp(),
                        _ => unreachable!(),
                    },
                    lexer::DSLType::FrameIndex(index) => info.frame_to_timestamp(*index),
                    lexer::DSLType::Timestamp(dur) => {
                        info.milliseconds_to_timestamp(dur.as_millis() as u64)
                    }
                };
                match op {
                    lexer::DSLOp::Add => {
                        pts += item;
                    }
                    lexer::DSLOp::Sub => {
                        pts -= item;
                    }
                }
            }
            pts
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn free_parse(res_ctx: *mut ArgParseResultContext) {
    if res_ctx.is_null() {
        return;
    }
    unsafe {
        _ = Box::from_raw(res_ctx);
    }
}
