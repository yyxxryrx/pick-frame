const std = @import("std");

const arg = @cImport({
    @cInclude("arg.h");
});

const util = @import("util.zig");
const errs = @import("error.zig");
const to_img = @import("frame_to_image.zig");
const read_info = @import("read_video_info.zig");
const video_reader = @import("read_video_frame.zig");

// const PATH_MAX = blk: {
//     const ci = @cImport({
//         @cInclude("limits.h");
//     });
//     break :blk @as(u16, @intCast(ci.PATH_MAX));
// };

const PATH_MAX: usize = switch (@import("builtin").os.tag) {
    .windows => 260,
    .linux => 4096,
    .macos => 1024,
    else => @compileError("unsupported os"),
};

pub fn main() !void {
    const arg_ctx = arg.parse();
    defer arg.free_parse(arg_ctx);

    var buffer: [1024]u8 = undefined;
    var stdout_writer = std.fs.File.stdout().writer(&buffer);
    const stdout = &stdout_writer.interface;

    try stdout.print("input: {s}, output: {s}", .{ arg.get_input(arg_ctx), arg.get_output(arg_ctx) });
    try stdout.flush();

    const input: []const u8 = std.mem.sliceTo(arg.get_input(arg_ctx), 0);
    const output: []const u8 = std.mem.sliceTo(arg.get_output(arg_ctx), 0);
    const format: []const u8 = std.mem.sliceTo(arg.get_format(arg_ctx), 0);

    // 检查输入文件是否存在
    std.fs.cwd().access(input, .{}) catch return errs.cli_err.CannotFoundFile;

    const out = try std.fs.cwd().makeOpenPath(output, .{});
    const info = try read_info.get_video_info(input);
    try stdout.print("info: {f}\n", .{info});
    try stdout.flush();

    // zig fmt: off
    const arg_info = arg.create_video_info(
        info.fps, 
        @intCast(info.time_base.den), 
        @intCast(info.time_base.num), 
        info.start_time, 
        @intCast(info.duration)
    );
    defer arg.free_video_info(arg_info);

    // 根据起始时间类型转换为时间戳
    const from = arg.get_from_timestamp(
        arg_ctx,
        arg_info
    );

    // 根据结束时间类型转换为时间戳
    const to = arg.get_to_timestamp(arg_ctx, arg_info);

    if (from > to)
        return errs.cli_err.InvalidRange;
    
    if (from < 0)
        return errs.cli_err.InvalidRange;
    
    if (to > info.duration)
        return errs.cli_err.InvalidRange;

    std.debug.print("start: {d} end: {d}\n", .{ from, to });
    std.debug.print("start: {d}\n", .{util.frame_to_timestamp(1, &info)});

    // 初始化视频读取器和图像保存器
    var reader = try video_reader.VideoReader.init(input, .{
        .video_info = info,
        .thread_count = arg.get_thread_count(arg_ctx),
    });
    defer reader.deinit();
    var saver = try to_img.ToImage.init(@bitCast(info.width), @bitCast(info.height), info.fmt, .{});
    defer saver.deinit();

    try reader.seek(from);

    var frame_index = util.timestamp_to_frame(from, &info);

    // 循环读取视频帧并保存为图片
    while (true) {
        var frame = reader.read_frame() catch |err| {
            switch (err) {
                errs.VideoReadFrameError.EOF => break,
                else => return err,
            }
        };
        defer frame.deinit();

        if (frame.frame.*.pts > to)
            break;

        if (frame.frame.*.pts < from)
            continue;

        var buf: [PATH_MAX]u8 = undefined;
        try util.format_str(format, &buf, @as(c_ulonglong, @intCast(frame_index)));
        const name: []const u8 = std.mem.sliceTo(&buf, 0);

        try stdout.print("Save: {s}\n", .{name});
        try stdout.flush();

        try saver.save(frame.frame, out, name);
        frame_index += 1;
    }
}
