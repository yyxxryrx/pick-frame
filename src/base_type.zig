const av = @import("cimport.zig").av;

const std = @import("std");

/// VideoInfo 结构体存储视频的基本信息
/// 包含帧数、尺寸、帧率等关键视频属性
pub const VideoInfo = struct {
    /// 视频总帧数
    frame_count: usize,
    /// 当前帧索引
    frame_index: usize,
    /// 视频持续时间（以时间基为单位）
    duration: u64,
    /// 视频宽度（像素）
    width: u32,
    /// 视频高度（像素）
    height: u32,
    /// 视频帧率
    fps: f64,
    /// 像素格式
    fmt: av.AVPixelFormat,
    /// 时间基，用于时间戳转换
    time_base: av.AVRational,
    /// 视频开始时间
    start_time: i64,

    // zig fmt: off
    /// 格式化输出VideoInfo结构体的内容
    /// 
    /// 参数:
    ///   - self: VideoInfo实例的引用
    ///   - writer: 用于输出的Writer对象
    /// 
    /// 返回值:
    ///   - void 或 Writer错误
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