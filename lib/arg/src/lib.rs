use clap::Parser;
use std::{ffi::CString, os::raw::c_char, time::Duration};

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

#[repr(C)]
#[derive(Debug, Default)]
pub struct TimeType {
    pub kind: TimeTypeKind,
    pub value: u64,
}

#[repr(C)]
pub struct ArgParseResult {
    pub input: *const c_char,
    pub output: *const c_char,
    pub start: TimeType,
    pub end: TimeType,
    pub thread_count: u16,
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
            let Ok(v)  = sub.parse::<f64>() else {
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

impl From<Time> for TimeType {
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

#[derive(Debug, Clone)]
enum ThreadCount {
    Auto,
    Custom(u16),
}

impl From<ThreadCount> for u16 {
    fn from(value: ThreadCount) -> Self {
        match value {
            ThreadCount::Auto => 0,
            ThreadCount::Custom(v) => v
        }
    }
}

impl std::str::FromStr for ThreadCount {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("auto") {
                Ok(Self::Auto)
        } else {
            s.parse::<u16>().map(Self::Custom).map_err(|err| err.to_string())
        }
    }
}

#[derive(Debug, Parser)]
#[command(about = "A simple video frame picker\n\nTips:\n\t`xxx` is frame index\n\t`xx:xx.xx` is timestamp\n\t`end` is the end of video\n\t`xx.xxs` is seconds-base timestamp")]
struct Cli {
    #[clap(short, long, help = "The video path")]
    input: String,
    #[clap(short, long, help = "possible format: [xxx, xx.xxs, xx:xx.xx, end]", default_value = "0")]
    from: Time,
    #[clap(short, long, help = "possible format: [xxx, xx.xxs, xx:xx.xx, end]", default_value = "end")]
    to: Time,
    #[arg(long, value_name = "Auto|num", help = "thread count for codec", default_value = "auto")]
    thread_count: ThreadCount,
    #[clap(help = "Output path", default_value = ".")]
    output: String,
}

#[unsafe(no_mangle)]
pub extern "C" fn parse() -> *mut ArgParseResult {
    let cli = Cli::parse();
    Box::into_raw(Box::new(ArgParseResult {
        input: CString::new(cli.input).unwrap_or_default().into_raw(),
        output: CString::new(cli.output).unwrap_or_default().into_raw(),
        start: cli.from.into(),
        end: cli.to.into(),
        thread_count: cli.thread_count.into()
    }))
}

#[unsafe(no_mangle)]
pub extern "C" fn free_parse(p_res: *mut ArgParseResult) {
    if p_res.is_null() {
        return;
    }
    unsafe {
        _ = Box::from_raw(p_res);
    }
}
