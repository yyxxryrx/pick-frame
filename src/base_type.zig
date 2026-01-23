const av = @import("cimport.zig").av;

const std = @import("std");

pub const VideoInfo = struct {
    frame_count: usize,
    frame_index: usize,
    duration: u64,
    width: u32,
    height: u32,
    fps: f64,
    fmt: av.AVPixelFormat,
    time_base: av.AVRational,
    start_time: i64,

    // zig fmt: off
    pub fn format(
        self: @This(),
        writer: *std.Io.Writer,
    ) std.Io.Writer.Error!void {
        try writer.print(
            "VideoInfo {{ frame_count: {d}, duration: {d}, width: {d}, height: {d}, fps: {d} }}",
            .{ self.frame_count, self.duration, self.width, self.height, self.fps }
        );
    }
};