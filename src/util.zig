const std = @import("std");

const c = @cImport({
    @cInclude("stdio.h");
});

const av = @import("cimport.zig").av;

const base_type = @import("base_type.zig");

const PATH_MAX: usize = 260;

/// 格式化字符串并存储到缓冲区中
///
/// 参数:
///   fmt - 格式化字符串模板
///   buffer - 输出缓冲区，大小至少为PATH_MAX
///   args - 可变参数列表，用于格式化
///
/// 返回:
///   void - 成功时无返回值，失败时返回错误
pub fn format_str(fmt: []const u8, buffer: *[PATH_MAX]u8, args: anytype) !void {
    const alloc = std.heap.page_allocator;

    const c_fmt = try alloc.alloc(u8, fmt.len + 1);
    defer alloc.free(c_fmt);

    std.mem.copyForwards(u8, c_fmt[0..fmt.len], fmt);
    c_fmt[fmt.len] = 0;

    const len = c.snprintf(@ptrCast(buffer), PATH_MAX, @ptrCast(c_fmt.ptr), args);
    if (len < 0)
        return error.OutOfMemory;
    if (len >= PATH_MAX)
        return error.OutOfMemory;
}

/// 将FFmpeg错误码转换为可读的错误字符串
///
/// 参数:
///   errenum - FFmpeg错误码
///
/// 返回:
///   []const u8 - 错误描述字符串
pub fn av_err2str(errenum: c_int) []const u8 {
    var buf: [128]u8 = undefined;
    if (av.av_strerror(errenum, &buf, buf.len) != 0)
        return "Unknown error";
    return std.mem.sliceTo(&buf, 0);
}

/// 处理FFmpeg错误码，如果错误则打印错误信息并退出程序
///
/// 参数:
///   code - FFmpeg返回的错误码
///
/// 返回:
///   void - 无返回值，成功时直接返回，失败时程序退出
pub fn error_handle(code: c_int) !void {
    if (code == 0)
        return;
    var buffer: [1024]u8 = undefined;
    var stderr_writer = std.fs.File.stderr().writer(&buffer);
    const stderr = &stderr_writer.interface;
    try stderr.print("{s}\n", .{av_err2str(code)});
    try stderr.flush();
    std.process.exit(1);
}

/// 将帧索引转换为时间戳
///
/// 参数:
///   frame_index - 帧索引号
///   info - 视频信息结构体指针
///
/// 返回:
///   i64 - 对应的时间戳值
pub fn frame_to_timestamp(frame_index: u64, info: *const base_type.VideoInfo) i64 {
    const fps = info.fps;
    const time_base = info.time_base;
    const start_time = info.start_time;
    // 1. 计算相对秒数
    const seconds = @as(f64, @floatFromInt(frame_index)) / fps;

    // 2. 将秒数转换为流的时间基单位 (PTS)
    // 公式：seconds / av_q2d(time_base)
    const tb_val = @as(f64, @floatFromInt(time_base.num)) / @as(f64, @floatFromInt(time_base.den));
    var target_ts: i64 = @intFromFloat(seconds / tb_val);

    // =======================================================
    // 【关键修复】加上流的起始时间 (start_time)
    // 很多视频不是从 0 开始的，如果不加这个，Seek 就会跳回开头
    // =======================================================
    if (start_time != av.AV_NOPTS_VALUE) {
        target_ts += start_time;
    }
    return target_ts;
}

/// 将毫秒数转换为时间戳
///
/// 参数:
///   ms - 毫秒数
///   info - 视频信息结构体指针
///
/// 返回:
///   i64 - 对应的时间戳值
pub fn milliseconds_to_timestamp(ms: u64, info: *const base_type.VideoInfo) i64 {
    const time_base = info.time_base;
    const start_time = info.start_time;
    const seconds = @as(f64, @floatFromInt(ms)) / 1000.0;
    const tb_val = @as(f64, @floatFromInt(time_base.num)) / @as(f64, @floatFromInt(time_base.den));
    var target_ts: i64 = @intFromFloat(seconds / tb_val);
    if (start_time != av.AV_NOPTS_VALUE) {
        target_ts += start_time;
    }
    return target_ts;
}

/// 将时间戳转换为帧索引
///
/// 参数:
///   timestamp - 时间戳值
///   info - 视频信息结构体指针
///
/// 返回:
///   u64 - 对应的帧索引号
pub fn timestamp_to_frame(timestamp: i64, info: *const base_type.VideoInfo) u64 {
    const fps = info.fps;
    const time_base = info.time_base;
    const start_time = info.start_time;
    var ts = timestamp;
    if (start_time != av.AV_NOPTS_VALUE) {
        ts -= start_time;
    }
    return @as(u64, @intFromFloat(@divFloor(@as(f64, @floatFromInt(ts)) * @as(f64, @floatFromInt(time_base.num)) * fps, @as(f64, @floatFromInt(time_base.den)))));
}
