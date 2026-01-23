const av = @cImport({
    @cInclude("libavcodec/avcodec.h");
    @cInclude("libswscale/swscale.h");
});

const std = @import("std");

const err = @import("error.zig");
const util = @import("util.zig");

const ToImage = struct {
    format: c_int,
    codec: [*c]const av.AVCodec,
    codec_ctx: [*c]av.AVCodecContext,
    sws_ctx: ?*av.SwsContext,

    pub fn init(width: c_int, height: c_int, src_format: av.AVPixelFormat, args: struct {
        encoder: c_int = av.AV_CODEC_ID_MJPEG,
        format: c_int = av.AV_PIX_FMT_YUVJ420P,
    }) !ToImage {
        const codec = av.avcodec_find_encoder(args.encoder);
        if (codec == null)
            return err.ffmpeg_err.CannotFoundCodec;

        var codec_ctx = av.avcodec_alloc_context3(codec);
        errdefer av.avcodec_free_context(&codec_ctx);

        if (codec_ctx == null)
            return err.ffmpeg_err.CannotAllocateCodecContext;

        codec_ctx.*.width = width;
        codec_ctx.*.height = height;
        codec_ctx.*.pix_fmt = args.format;
        codec_ctx.*.time_base = .{ .num = 1, .den = 25 };

        try util.error_handle(av.avcodec_open2(codec_ctx, codec, null));

        const sws_ctx = av.sws_getContext(width, height, src_format, width, height, args.format, av.SWS_BILINEAR, null, null, null);
        errdefer av.sws_freeContext(sws_ctx);

        if (sws_ctx == null)
            return err.ffmpeg_err.GetSwsContextFailed;

        return ToImage{ .codec = codec, .format = args.format, .codec_ctx = codec_ctx, .sws_ctx = sws_ctx };
    }

    pub fn deinit(self: *@This()) void {
        av.avcodec_free_context(&self.codec_ctx);
        av.sws_freeContext(self.sws_ctx);
    }

    pub fn save(self: @This(), frame: [*c]av.AVFrame, dir: std.fs.Dir, filename: []const u8) !void {
        const width = frame.*.width;
        const height = frame.*.height;

        var rgb_frame = av.av_frame_alloc();
        defer av.av_frame_free(&rgb_frame);

        if (rgb_frame == null)
            return error.AllocateFrameFailed;

        rgb_frame.*.format = self.format;
        rgb_frame.*.width = width;
        rgb_frame.*.height = height;

        try util.error_handle(av.av_frame_get_buffer(rgb_frame, 0));

        _ = av.sws_scale(self.sws_ctx, &frame.*.data, &frame.*.linesize, 0, height, &rgb_frame.*.data, &rgb_frame.*.linesize);

        var pkt = av.av_packet_alloc();
        defer av.av_packet_free(&pkt);

        var ret = av.avcodec_send_frame(self.codec_ctx, rgb_frame);
        if (ret >= 0) {
            ret = av.avcodec_receive_packet(self.codec_ctx, pkt);
            if (ret >= 0) {
                var file = try dir.createFile(filename, .{});
                defer file.close();
                const size: usize = @intCast(pkt.*.size);
                try file.writeAll(pkt.*.data[0..size]);
                av.av_packet_unref(pkt);
            }
        }
    }
};
