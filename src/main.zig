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

const PATH_MAX: usize = 260;

/// 主函数入口点，解析命令行参数并执行视频帧提取操作
pub fn main() !void {
    const args = arg.parse();
    defer arg.free_parse(args);

    try run(args);
}

/// 执行视频帧提取的主要逻辑函数
///
/// 参数:
///   - args: 命令行参数解析结果指针
/// 返回值:
///   - void: 成功时无返回值，失败时返回错误
fn run(args: [*c]arg.ArgParseResult) !void {
    var buffer: [1024]u8 = undefined;
    var stdout_writer = std.fs.File.stdout().writer(&buffer);
    const stdout = &stdout_writer.interface;

    try stdout.print("input: {s}, output: {s}", .{ args.*.input, args.*.output });
    try stdout.flush();

    const input: []const u8 = std.mem.sliceTo(args.*.input, 0);
    const output: []const u8 = std.mem.sliceTo(args.*.output, 0);
    const format: []const u8 = std.mem.sliceTo(args.*.format, 0);

    // 检查输入文件是否存在
    std.fs.cwd().access(input, .{}) catch return errs.cli_err.CannotFoundFile;

    const out = try std.fs.cwd().makeOpenPath(output, .{});
    const info = try read_info.get_video_info(input);
    try stdout.print("info: {f}\n", .{info});
    try stdout.flush();

    // 根据起始时间类型转换为时间戳
    const from = switch (args.*.start.kind) {
        arg.Frame => util.frame_to_timestamp(args.*.start.value, &info),
        arg.Millisecond => util.milliseconds_to_timestamp(args.*.start.value, &info),
        arg.End => std.math.maxInt(i64),
        else => unreachable,
    };

    // 根据结束时间类型转换为时间戳
    const to = switch (args.*.end.kind) {
        arg.Frame => util.frame_to_timestamp(args.*.end.value, &info),
        arg.Millisecond => util.milliseconds_to_timestamp(args.*.end.value, &info),
        arg.End => std.math.maxInt(i64),
        else => unreachable,
    };

    if (from > to)
        return errs.cli_err.InvalidRange;

    std.debug.print("start: {d} end: {d}\n", .{ from, to });

    // 初始化视频读取器和图像保存器
    var reader = try video_reader.VideoReader.init(input, .{
        .video_info = info,
        .thread_count = args.*.thread_count,
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
