const std = @import("std");

const av = @import("cimport.zig").av;

const err = @import("error.zig");
const util = @import("util.zig");
const base_type = @import("base_type.zig");
const info = @import("read_video_info.zig");

/// VideoFrame 结构体表示一个视频帧
/// 包装了FFmpeg的AVFrame指针
pub const VideoFrame = struct {
    frame: [*c]av.AVFrame,

    /// 初始化VideoFrame实例
    /// 参数 frame: 指向AVFrame的指针
    /// 返回 VideoFrame实例
    pub fn init(frame: *av.AVFrame) VideoFrame {
        return VideoFrame{ .frame = frame };
    }

    /// 释放VideoFrame资源
    /// 参数 self: VideoFrame实例的指针
    pub fn deinit(self: *@This()) void {
        av.av_frame_free(&self.frame);
    }
};

/// VideoReaderArgs 结构体定义VideoReader的初始化参数
pub const VideoReaderArgs = struct {
    video_info: ?base_type.VideoInfo = null,
    thread_count: u16 = 0,
};

/// VideoReader 结构体用于读取视频文件
/// 提供视频解码和帧读取功能
pub const VideoReader = struct {
    fmt_ctx: ?*av.AVFormatContext = null,
    codec_ctx: ?*av.AVCodecContext = null,
    info: base_type.VideoInfo,

    /// 初始化VideoReader实例
    /// 参数 path: 视频文件路径
    /// 参数 args: VideoReaderArgs结构体，包含初始化参数
    /// 返回 VideoReader实例或错误
    pub fn init(path: []const u8, args: VideoReaderArgs) !VideoReader {
        const video_info = args.video_info orelse try info.get_video_info(path);

        const alloc = std.heap.page_allocator;
        _ = av.avformat_network_init();

        const c_path = try alloc.alloc(u8, path.len + 1);
        defer alloc.free(c_path);

        std.mem.copyForwards(u8, c_path[0..path.len], path);
        c_path[path.len] = 0;

        const c_path_ptr: [*c]const u8 = @ptrCast(c_path.ptr);

        var context: ?*av.AVFormatContext = null;

        // zig fmt: off
        try util.error_handle(
            av.avformat_open_input(
                &context,
                c_path_ptr,
                null,
                null
            )
        );

        // 获取视频流信息
        try util.error_handle(av.avformat_find_stream_info(context, null));

        const index = video_info.frame_index;
        const codec_par = context.?.streams[index].*.codecpar;
        const codec = av.avcodec_find_decoder(codec_par.*.codec_id);

        if (codec == null)
            return error.CannotFoundCodec;

        const codec_context = av.avcodec_alloc_context3(codec);
        try util.error_handle(av.avcodec_parameters_to_context(codec_context, codec_par));
        codec_context.*.thread_count = args.thread_count;

        // 打开解码器
        try util.error_handle(av.avcodec_open2(codec_context, codec, null));

        return VideoReader {
            .fmt_ctx = context,
            .codec_ctx = codec_context,
            .info = video_info,
        };
    }

    /// 读取下一帧视频数据
    /// 参数 self: VideoReader实例
    /// 返回 VideoFrame实例或错误
    pub fn read_frame(self: @This()) err.VideoReadFrameError!VideoFrame {
        const frame = av.av_frame_alloc();

        if (av.avcodec_receive_frame(self.codec_ctx, frame) == 0)
            return VideoFrame.init(frame);
        av.av_frame_unref(frame);

        const index  = self.info.frame_index;
        var pkt = av.av_packet_alloc();
        defer av.av_packet_free(&pkt);
        
        while (av.av_read_frame(self.fmt_ctx, pkt) >= 0) {
            if (pkt.*.stream_index == index) {
                const ret = av.avcodec_send_packet(self.codec_ctx, pkt);
                if (ret < 0 and ret != av.AVERROR(av.EAGAIN)) continue;
                if (av.avcodec_receive_frame(self.codec_ctx, frame) == 0)
                    return VideoFrame.init(frame);
            }
        }
        return err.VideoReadFrameError.EOF;
    }

    /// 跳转到指定时间戳位置
    /// 参数 self: VideoReader实例
    /// 参数 timestamp: 目标时间戳
    /// 返回 void或错误
    pub fn seek(self: @This(), timestamp: i64) !void {
        // zig fmt: off
        try util.error_handle(
            av.avformat_seek_file(
                self.fmt_ctx,
                @intCast(self.info.frame_index),
                std.math.minInt(i64),
                timestamp,
                std.math.maxInt(i64),
                av.AVSEEK_FLAG_BACKWARD
            )
        );

        // 清空解码器缓冲区
        av.avcodec_flush_buffers(self.codec_ctx);
    }

    /// 释放VideoReader资源
    /// 参数 self: VideoReader实例的指针
    pub fn deinit(self: *@This()) void {
        av.avformat_close_input(&self.fmt_ctx);
        _ = av.avformat_network_deinit();
    }
};
