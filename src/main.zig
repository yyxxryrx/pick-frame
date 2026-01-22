const std = @import("std");
const c = @cImport({
    @cInclude("libavformat/avformat.h");
    @cInclude("libavcodec/avcodec.h");
    @cInclude("libavutil/avutil.h");
    @cInclude("libswscale/swscale.h");
});

const arg = @cImport({
    @cInclude("arg.h");
});

const ffmpeg_err = error{
    CannotFoundBestStream,
    CannotFoundCodec,
    CannotAllocateCodecContext,
    GetSwsContextFailed,
    AllocateFrameFailed,
};

const cli_err = error{ CannotFoundFile, InvalidRange };

const VideoInfo = struct {
    frame_count: usize,
    frame_index: usize,
    duration: u64,
    width: u32,
    height: u32,
    fps: f64,
    fmt: c.AVPixelFormat,
    time_base: c.AVRational,
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

fn error_handle(code: c_int) !void {
    if (code == 0)
        return;
    var buffer: [1024]u8 = undefined;
    var stderr_writer = std.fs.File.stderr().writer(&buffer);
    const stderr = &stderr_writer.interface;
    try stderr.print("{s}\n", .{av_err2str(code)});
    try stderr.flush();
    std.process.exit(1);
}

fn get_video_info(path: []const u8) !VideoInfo {
    const alloc = std.heap.page_allocator;

    _ = c.avformat_network_init();
    defer _ = c.avformat_network_deinit();

    const c_path = try alloc.alloc(u8, path.len + 1);
    defer alloc.free(c_path);

    std.mem.copyForwards(u8, c_path[0..path.len], path);
    c_path[path.len] = 0;

    const c_path_ptr: [*c]const u8 = @ptrCast(c_path.ptr);

    var context: ?*c.AVFormatContext = null;

    // zig fmt: off
    try error_handle(
        c.avformat_open_input(
            &context,
            c_path_ptr,
            null,
            null
        )
    );
    defer c.avformat_close_input(&context);

    try error_handle(c.avformat_find_stream_info(context, null));

    const index: usize = @intCast(c.av_find_best_stream(context, c.AVMEDIA_TYPE_VIDEO, -1, -1, null, 0));
    if (index < 0)
        return ffmpeg_err.CannotFoundBestStream;

    const stream = context.?.streams[index];
    const codec_params = stream.*.codecpar;

    const codec = c.avcodec_find_decoder(codec_params.*.codec_id);
    if (codec == null)
        return ffmpeg_err.CannotFoundCodec;

    const codec_context = c.avcodec_alloc_context3(codec);
    if (codec_context == null)
        return ffmpeg_err.CannotAllocateCodecContext;

    try error_handle(c.avcodec_parameters_to_context(codec_context, codec_params));

    const num: f64 = @floatFromInt(stream.*.avg_frame_rate.num);
    const den: f64 = @floatFromInt(stream.*.avg_frame_rate.den);

    return VideoInfo {
        .frame_count = @intCast(stream.*.nb_frames),
        .duration = @intCast(stream.*.duration),
        .width = @intCast(codec_params.*.width),
        .height = @intCast(codec_params.*.height),
        .fps = num / den,
        .frame_index = index,
        .fmt = codec_context.*.pix_fmt,
        .time_base = stream.*.time_base,
        .start_time = stream.*.start_time,
    };
}

fn frame_to_timestamp(frame_index: u64, info: *const VideoInfo) i64 {
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
    if (start_time != c.AV_NOPTS_VALUE) {
        target_ts += start_time;
    }
    return target_ts;
}

fn milliseconds_to_timestamp(ms: u64, info: *const VideoInfo) i64 {
    const time_base = info.time_base;
    const start_time = info.start_time;
    const seconds = @as(f64, @floatFromInt(ms)) / 1000.0;
    const tb_val = @as(f64, @floatFromInt(time_base.num)) / @as(f64, @floatFromInt(time_base.den));
    var target_ts: i64 = @intFromFloat(seconds / tb_val);
    if (start_time != c.AV_NOPTS_VALUE) {
        target_ts += start_time;
    }
    return target_ts;
}

fn timestamp_to_frame(timestamp: i64, info: *const VideoInfo) u64 {
    const fps = info.fps;
    const time_base = info.time_base;
    const start_time = info.start_time;
    var ts = timestamp;
    if (start_time != c.AV_NOPTS_VALUE) {
        ts -= start_time;
    }
    return @as(
        u64, 
        @intFromFloat(
            @divFloor(
                @as(f64, @floatFromInt(ts)) * @as(f64, @floatFromInt(time_base.num)) * fps,
                @as(f64, @floatFromInt(time_base.den))
            )
        )
    );
}

const VideoFrame = struct {
    frame: [*c]c.AVFrame,

    pub fn init(frame: *c.AVFrame) VideoFrame {
        return VideoFrame {
            .frame = frame
        };
    }

    pub fn deinit(self: *@This()) void {
        c.av_frame_free(&self.frame);
    }
};

const VideoReadFrameError = error {
    EOF,
};

const VideoReaderArgs = struct {
    video_info: ?VideoInfo = null,
    thread_count: u16 = 0,
};

const VideoReader = struct {

    fmt_ctx: ?*c.AVFormatContext = null,
    codec_ctx: ?*c.AVCodecContext = null,
    info: VideoInfo,


    pub fn init(path: []const u8, args: VideoReaderArgs) !VideoReader {
        const info = args.video_info orelse try get_video_info(path);

        const alloc = std.heap.page_allocator;
        _ = c.avformat_network_init();

        const c_path = try alloc.alloc(u8, path.len + 1);
        defer alloc.free(c_path);

        std.mem.copyForwards(u8, c_path[0..path.len], path);
        c_path[path.len] = 0;

        const c_path_ptr: [*c]const u8 = @ptrCast(c_path.ptr);

        var context: ?*c.AVFormatContext = null;

        // zig fmt: off
        try error_handle(
            c.avformat_open_input(
                &context,
                c_path_ptr,
                null,
                null
            )
        );

        try error_handle(c.avformat_find_stream_info(context, null));

        const index = info.frame_index;
        const codec_par = context.?.streams[index].*.codecpar;
        const codec = c.avcodec_find_decoder(codec_par.*.codec_id);

        if (codec == null)
            return error.CannotFoundCodec;

        const codec_context = c.avcodec_alloc_context3(codec);
        try error_handle(c.avcodec_parameters_to_context(codec_context, codec_par));
        codec_context.*.thread_count = args.thread_count;

        try error_handle(c.avcodec_open2(codec_context, codec, null));

        return VideoReader {
            .fmt_ctx = context,
            .codec_ctx = codec_context,
            .info = info,
        };
    }

    pub fn read_frame(self: @This()) VideoReadFrameError!VideoFrame {
        const index  = self.info.frame_index;
        const pkt = c.av_packet_alloc();
        const frame = c.av_frame_alloc();

        while (c.av_read_frame(self.fmt_ctx, pkt) >= 0) {
            if (pkt.*.stream_index == index) {
                if (c.avcodec_send_packet(self.codec_ctx, pkt) < 0) continue;
                if (c.avcodec_receive_frame(self.codec_ctx, frame) == 0)
                    return VideoFrame.init(frame);
            }
        }
        return VideoReadFrameError.EOF;
    }
    
    pub fn seek(self: @This(), timestamp: i64) !void {
        // zig fmt: off
        try error_handle(
            c.avformat_seek_file(
                self.fmt_ctx,
                @intCast(self.info.frame_index),
                std.math.minInt(i64),
                timestamp,
                std.math.maxInt(i64),
                c.AVSEEK_FLAG_BACKWARD
            )
        );

        c.avcodec_flush_buffers(self.codec_ctx);
    }

    pub fn deinit(self: *@This()) void {
        c.avformat_close_input(&self.fmt_ctx);
        _ = c.avformat_network_deinit();
    }
};

pub fn av_err2str(errenum: c_int) []const u8 {
    var buf: [128]u8 = undefined;
    if (c.av_strerror(errenum, &buf, buf.len) != 0)
        return "Unknown error";
    return std.mem.sliceTo(&buf, 0);
}

const ToImage = struct {
    format: c_int,
    codec: [*c]const c.AVCodec,
    codec_ctx: [*c] c.AVCodecContext,
    sws_ctx: ?*c.SwsContext,

    pub fn init(width: c_int, height: c_int, src_format: c.AVPixelFormat, args: struct {
        encoder: c_int = c.AV_CODEC_ID_MJPEG,
        format: c_int = c.AV_PIX_FMT_YUVJ420P,
    }) !ToImage {
        const codec = c.avcodec_find_encoder(args.encoder);
        if (codec == null)
            return ffmpeg_err.CannotFoundCodec;

        var codec_ctx = c.avcodec_alloc_context3(codec);
        errdefer c.avcodec_free_context(&codec_ctx);

        if (codec_ctx == null)
            return ffmpeg_err.CannotAllocateCodecContext;

        codec_ctx.*.width = width;
        codec_ctx.*.height = height;
        codec_ctx.*.pix_fmt = args.format;
        codec_ctx.*.time_base = .{.num = 1, .den = 25};

        try error_handle(c.avcodec_open2(codec_ctx, codec, null));

        const sws_ctx = c.sws_getContext(
            width,
            height,
            src_format,
            width,
            height,
            args.format,
            c.SWS_BILINEAR,
            null,
            null,
            null
        );
        errdefer c.sws_freeContext(sws_ctx);

        if (sws_ctx == null)
            return ffmpeg_err.GetSwsContextFailed;

        return ToImage {
            .codec = codec,
            .format = args.format,
            .codec_ctx = codec_ctx,
            .sws_ctx = sws_ctx
        };
    }

    pub fn deinit(self: *@This()) void {
        c.avcodec_free_context(&self.codec_ctx);
        c.sws_freeContext(self.sws_ctx);
    }


    pub fn save(self: @This(), frame: [*c]c.AVFrame, dir: std.fs.Dir, filename: []const u8) !void {
        const width = frame.*.width;
        const height = frame.*.height;


        var rgb_frame = c.av_frame_alloc();
        defer c.av_frame_free(&rgb_frame);

        if (rgb_frame == null)
            return error.AllocateFrameFailed;

        rgb_frame.*.format = self.format;
        rgb_frame.*.width = width;
        rgb_frame.*.height = height;

        try error_handle(c.av_frame_get_buffer(rgb_frame, 0));

        _ = c.sws_scale(self.sws_ctx, &frame.*.data, &frame.*.linesize, 0, height, &rgb_frame.*.data,& rgb_frame.*.linesize);

        var pkt = c.av_packet_alloc();
        defer c.av_packet_free(&pkt);

        var ret = c.avcodec_send_frame(self.codec_ctx, rgb_frame);
        if (ret >= 0) {
            ret = c.avcodec_receive_packet(self.codec_ctx, pkt);
            if (ret >= 0) {
                var file = try dir.createFile(filename, .{});
                defer file.close();
                const size: usize = @intCast(pkt.*.size);
                try file.writeAll(pkt.*.data[0..size]);
                c.av_packet_unref(pkt);
            }
        }
    }
};



pub fn main() !void {
    const args = arg.parse();
    defer arg.free_parse(args);

    try run(args);
}


fn run(args: [*c]arg.ArgParseResult) !void {
    const alloc = std.heap.page_allocator;
    var buffer: [1024]u8 = undefined;
    var stdout_writer = std.fs.File.stdout().writer(&buffer);
    const stdout = &stdout_writer.interface;

    try stdout.print("input: {s}, output: {s}", .{args.*.input, args.*.output});
    try stdout.flush();

    const input: []const u8 = std.mem.sliceTo(args.*.input, 0);
    const output: []const u8 = std.mem.sliceTo(args.*.output, 0);

    std.fs.cwd().access(input, .{}) catch return cli_err.CannotFoundFile;

    const out = try std.fs.cwd().makeOpenPath(output, .{});
    const info = try get_video_info(input);
    try stdout.print("info: {f}\n", .{info});
    try stdout.flush();

    const from = switch (args.*.start.kind) {
        arg.Frame => frame_to_timestamp(args.*.start.value, &info),
        arg.Millisecond => milliseconds_to_timestamp(args.*.start.value, &info),
        arg.End => std.math.maxInt(i64),
        else => unreachable,
    };

    const to = switch (args.*.end.kind) {
        arg.Frame => frame_to_timestamp(args.*.end.value, &info),
        arg.Millisecond => milliseconds_to_timestamp(args.*.end.value, &info),
        arg.End => std.math.maxInt(i64),
        else => unreachable,
    };

    if (from > to)
        return cli_err.InvalidRange;

    std.debug.print("start: {d} end: {d}\n", .{from, to});

    var reader = try VideoReader.init(input, .{
        .video_info = info,
        .thread_count = args.*.thread_count,
    });
    defer reader.deinit();
    var saver = try ToImage.init(@bitCast(info.width), @bitCast(info.height), info.fmt, .{});
    defer saver.deinit();

    try reader.seek(from);

    var frame_index = timestamp_to_frame(from, &info);

    while (true) {
        var frame = reader.read_frame() catch |err| {
            switch (err) {
                VideoReadFrameError.EOF => break,
                else => return err,
            }
        };
        defer frame.deinit();

        if (frame.frame.*.pts > to)
            break;

        if (frame.frame.*.pts < from)
            continue;

        const name = try std.fmt.allocPrint(alloc, "frame-{d}.jpg", .{frame_index});
        defer alloc.free(name);

        try stdout.print("Save: {s}\n", .{name});
        try stdout.flush();

        try saver.save(frame.frame, out, name);
        frame_index += 1;
    }
}