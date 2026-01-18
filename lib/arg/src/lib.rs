use clap::Parser;
use std::{ffi::CString, os::raw::c_char};

#[repr(C)]
#[derive(Debug)]
pub enum TimeTypeKind {
    Value = 0,
    Start = 1,
    End = 2,
}

impl Default for TimeTypeKind {
    fn default() -> Self {
        Self::Value
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
}

#[derive(Debug, Parser)]
struct Cli {
    #[clap(help = "The video path")]
    input: String,
    #[clap(help = "Output path", default_value = ".")]
    output: String,
}

#[unsafe(no_mangle)]
pub extern "C" fn parse() -> *mut ArgParseResult {
    let cli = Cli::parse();
    Box::into_raw(Box::new(ArgParseResult {
        input: CString::new(cli.input).unwrap_or_default().into_raw(),
        output: CString::new(cli.output).unwrap_or_default().into_raw(),
        start: Default::default(),
        end: Default::default(),
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
