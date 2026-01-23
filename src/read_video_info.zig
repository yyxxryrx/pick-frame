const std = @import("std");

const av = @import("cimport.zig").av;

const util = @import("util.zig");
const err = @import("error.zig");
const base_type = @import("base_type.zig");

pub fn get_video_info(path: []const u8) !base_type.VideoInfo {
    const alloc = std.heap.page_allocator;

    _ = av.avformat_network_init();
    defer _ = av.avformat_network_deinit();

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
    defer av.avformat_close_input(&context);

    try util.error_handle(av.avformat_find_stream_info(context, null));

    const index: usize = @intCast(av.av_find_best_stream(context, av.AVMEDIA_TYPE_VIDEO, -1, -1, null, 0));
    if (index < 0)
        return err.ffmpeg_err.CannotFoundBestStream;

    const stream = context.?.streams[index];
    const codec_params = stream.*.codecpar;

    const codec = av.avcodec_find_decoder(codec_params.*.codec_id);
    if (codec == null)
        return err.ffmpeg_err.CannotFoundCodec;

    const codec_context = av.avcodec_alloc_context3(codec);
    if (codec_context == null)
        return err.ffmpeg_err.CannotAllocateCodecContext;

    try util.error_handle(av.avcodec_parameters_to_context(codec_context, codec_params));

    const num: f64 = @floatFromInt(stream.*.avg_frame_rate.num);
    const den: f64 = @floatFromInt(stream.*.avg_frame_rate.den);

    return base_type.VideoInfo {
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