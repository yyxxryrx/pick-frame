const std = @import("std");

const c = @cImport({
    @cInclude("stdio.h");
});

const av = @import("cimport.zig").av;

const base_type = @import("base_type.zig");

const PATH_MAX: usize = 260;

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

pub fn av_err2str(errenum: c_int) []const u8 {
    var buf: [128]u8 = undefined;
    if (av.av_strerror(errenum, &buf, buf.len) != 0)
        return "Unknown error";
    return std.mem.sliceTo(&buf, 0);
}

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
