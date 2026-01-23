const av = @import("cimport.zig").av;

const std = @import("std");

const err = @import("error.zig");
const util = @import("util.zig");

/// ToImage 结构体用于将视频帧转换为图像文件
/// 包含编码器、编解码器上下文和图像缩放上下文
pub const ToImage = struct {
    format: c_int,
    codec: [*c]const av.AVCodec,
    codec_ctx: [*c]av.AVCodecContext,
    sws_ctx: ?*av.SwsContext,

    /// 初始化ToImage实例
    ///
    /// 参数:
    ///   - width: 目标图像宽度
    ///   - height: 目标图像高度
    ///   - src_format: 源像素格式
    ///   - args: 编码器配置参数，包含encoder和format选项
    ///
    /// 返回值:
    ///   - ToImage: 成功时返回初始化的ToImage实例
    ///   - 错误: 失败时返回相应的错误码
    pub fn init(width: c_int, height: c_int, src_format: av.AVPixelFormat, args: struct {
        encoder: c_int = av.AV_CODEC_ID_MJPEG,
        format: c_int = av.AV_PIX_FMT_YUVJ420P,
    }) !ToImage {
        // 查找指定的编码器
        const codec = av.avcodec_find_encoder(args.encoder);
        if (codec == null)
            return err.ffmpeg_err.CannotFoundCodec;

        // 分配编解码器上下文
        var codec_ctx = av.avcodec_alloc_context3(codec);
        errdefer av.avcodec_free_context(&codec_ctx);

        if (codec_ctx == null)
            return err.ffmpeg_err.CannotAllocateCodecContext;

        // 设置编解码器参数
        codec_ctx.*.width = width;
        codec_ctx.*.height = height;
        codec_ctx.*.pix_fmt = args.format;
        codec_ctx.*.time_base = .{ .num = 1, .den = 25 };

        // 打开编解码器
        try util.error_handle(av.avcodec_open2(codec_ctx, codec, null));

        // 创建图像缩放上下文
        const sws_ctx = av.sws_getContext(width, height, src_format, width, height, args.format, av.SWS_BILINEAR, null, null, null);
        errdefer av.sws_freeContext(sws_ctx);

        if (sws_ctx == null)
            return err.ffmpeg_err.GetSwsContextFailed;

        return ToImage{ .codec = codec, .format = args.format, .codec_ctx = codec_ctx, .sws_ctx = sws_ctx };
    }

    /// 释放ToImage实例占用的资源
    ///
    /// 参数:
    ///   - self: ToImage实例指针
    pub fn deinit(self: *@This()) void {
        av.avcodec_free_context(&self.codec_ctx);
        av.sws_freeContext(self.sws_ctx);
    }

    /// 将视频帧保存为图像文件
    ///
    /// 参数:
    ///   - self: ToImage实例
    ///   - frame: 源AVFrame指针
    ///   - dir: 目标目录
    ///   - filename: 输出文件名
    ///
    /// 返回值:
    ///   - void: 成功时无返回值
    ///   - 错误: 失败时返回相应的错误码
    pub fn save(self: @This(), frame: [*c]av.AVFrame, dir: std.fs.Dir, filename: []const u8) !void {
        const width = frame.*.width;
        const height = frame.*.height;

        // 分配RGB帧内存
        var rgb_frame = av.av_frame_alloc();
        defer av.av_frame_free(&rgb_frame);

        if (rgb_frame == null)
            return error.AllocateFrameFailed;

        // 设置RGB帧参数
        rgb_frame.*.format = self.format;
        rgb_frame.*.width = width;
        rgb_frame.*.height = height;

        // 分配帧缓冲区
        try util.error_handle(av.av_frame_get_buffer(rgb_frame, 0));

        // 执行图像格式转换和缩放
        _ = av.sws_scale(self.sws_ctx, &frame.*.data, &frame.*.linesize, 0, height, &rgb_frame.*.data, &rgb_frame.*.linesize);

        // 分配数据包
        var pkt = av.av_packet_alloc();
        defer av.av_packet_free(&pkt);

        // 发送帧并接收编码后的数据包
        var ret = av.avcodec_send_frame(self.codec_ctx, rgb_frame);
        if (ret >= 0) {
            ret = av.avcodec_receive_packet(self.codec_ctx, pkt);
            if (ret >= 0) {
                // 创建输出文件并写入编码数据
                var file = try dir.createFile(filename, .{});
                defer file.close();
                const size: usize = @intCast(pkt.*.size);
                try file.writeAll(pkt.*.data[0..size]);
                av.av_packet_unref(pkt);
            }
        }
    }
};
