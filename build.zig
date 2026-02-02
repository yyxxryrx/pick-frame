const std = @import("std");

pub fn build(b: *std.Build) void {
    const allocator = std.heap.page_allocator;
    const os = @import("builtin").os;
    // zig fmt: off
    const target = b.standardTargetOptions(.{
        .default_target = .{
            .abi = if (os.tag == .windows) .msvc else null
        }
    });

    const optimize = b.standardOptimizeOption(.{});

    var cargo_args = std.ArrayList([]const u8).empty;
    defer cargo_args.deinit(allocator);

    cargo_args.appendSlice(allocator, switch (b.release_mode) {
        .off => &. {
            "cargo", "build"
        },
        .small => &.{
            "cargo", "build", "--profile=release-small"
        },
        .safe => &.{
            "cargo", "build", "--profile=release-safe"
        },
        else => &.{
            "cargo", "build", "--release"
        }
    }) catch @panic("err");

    const use_dsl = b.option(bool, "enable-time-expr", "enable time expr") orelse false;
    
    if (use_dsl) {
        cargo_args.append(allocator, "--features") catch @panic("err");
        cargo_args.append(allocator, "dsl") catch @panic("err");
    }

    const cargo_build = b.addSystemCommand(cargo_args.items);
    cargo_build.setCwd(b.path("lib/arg/"));

    const exe = b.addExecutable(.{
        .name = "pick-frame",
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/main.zig"),
            .target = target,
            .optimize = optimize,
        }),
    });

    exe.is_linking_libc = true;
    exe.bundle_ubsan_rt = false;
    exe.bundle_compiler_rt = false;
    exe.root_module.link_libcpp = os.tag != .windows;

    const target_name = switch (b.release_mode) {
        .off => "debug",
        .small => "release-small",
        .safe => "release-safe",
        else => "release"
    };

    exe.root_module.addIncludePath(b.path("lib/arg/include"));
    exe.root_module.addLibraryPath(b.path(b.pathJoin(&.{"lib/arg/target", target_name})));

    exe.root_module.linkSystemLibrary("arg", .{.preferred_link_mode = .static});

    const use_system = b.option(bool, "use-system", "use system library") orelse false;

    const is_dynamic = b.option(bool, "dynamic-link", "dynamic link ffmpeg") orelse false;
    const link_mode: std.builtin.LinkMode = if (is_dynamic) .dynamic else .static;

    if (!use_system) {
        const vcpkg_root = b.option([]const u8, "vcpkg-path", "The path of vcpkg root") orelse (std.process.getEnvVarOwned(b.allocator, "VCPKG_ROOT") catch {
            // 如果读取失败（没设置这个变量），我们打印一个友好的错误提示并退出
            std.debug.print("Error: environment variable 'VCPKG_ROOT' is not set.\n", .{});
            std.debug.print("Please set it to your vcpkg installation path.\n", .{});
            std.debug.print("Example: set VCPKG_ROOT=C:\\Users\\yourname\\vcpkg\n", .{});
            std.process.exit(1);
        });

        // const triplet = if (is_dynamic) "x64-windows" else "x64-windows-static";
        const triplet = b.option([]const u8, "triplet", "vcpkg installed triplet") orelse (switch (os.tag) {
            .windows => if (is_dynamic) "x64-windows" else "x64-windows-static",
            .linux => "x64-linux",
            .macos => if (@import("builtin").cpu.arch.isArm()) "arm64-osx" else "x64-osx",
            else => @panic("unsupported system")
        });
        const vcpkg_include = b.pathJoin(&.{ vcpkg_root, "installed", triplet, "include" });
        const vcpkg_lib = b.pathJoin(&.{ vcpkg_root, "installed", triplet, "lib" });

        exe.root_module.addIncludePath(std.Build.LazyPath{ .cwd_relative = vcpkg_include });
        exe.root_module.addLibraryPath(std.Build.LazyPath{ .cwd_relative = vcpkg_lib });
    }

    exe.root_module.linkSystemLibrary("avdevice", .{.preferred_link_mode = link_mode});
    exe.root_module.linkSystemLibrary("avformat", .{.preferred_link_mode = link_mode});
    exe.root_module.linkSystemLibrary("avfilter", .{.preferred_link_mode = link_mode});
    exe.root_module.linkSystemLibrary("avcodec", .{.preferred_link_mode = link_mode});
    exe.root_module.linkSystemLibrary("swresample", .{.preferred_link_mode = link_mode});
    exe.root_module.linkSystemLibrary("swscale", .{.preferred_link_mode = link_mode});
    exe.root_module.linkSystemLibrary("avutil", .{.preferred_link_mode = link_mode});

    if (os.tag == .windows) {
        exe.root_module.linkSystemLibrary("libx264", .{.preferred_link_mode = link_mode}); // 如果你刚才安装了 [x264]
    } else {
        exe.root_module.linkSystemLibrary("x264", .{.preferred_link_mode = link_mode});
    }

    if (!is_dynamic) {
        exe.root_module.linkSystemLibrary("zlib", .{.preferred_link_mode = link_mode});
        exe.root_module.linkSystemLibrary("bz2", .{.preferred_link_mode = link_mode});     // 有时候 avformat 需要
    }

    if (os.tag == .windows) {
        exe.root_module.linkSystemLibrary("ws2_32", .{});  // 网络 socket
        exe.root_module.linkSystemLibrary("bcrypt", .{});  // 加密
        exe.root_module.linkSystemLibrary("secur32", .{}); // 安全
        exe.root_module.linkSystemLibrary("user32", .{});
        exe.root_module.linkSystemLibrary("gdi32", .{});
        exe.root_module.linkSystemLibrary("ole32", .{});
        exe.root_module.linkSystemLibrary("oleaut32", .{});
        exe.root_module.linkSystemLibrary("advapi32", .{});
        exe.root_module.linkSystemLibrary("shell32", .{});
        exe.root_module.linkSystemLibrary("mfplat", .{});  // Media Foundation (如果是完整版 ffmpeg 可能需要)
        exe.root_module.linkSystemLibrary("mfuuid", .{});
        exe.root_module.linkSystemLibrary("strmiids", .{});
        exe.root_module.linkSystemLibrary("userenv", .{});
    }
    exe.step.dependOn(&cargo_build.step);

    b.installArtifact(exe);

    const run_step = b.step("run", "Run the app");

    const run_cmd = b.addRunArtifact(exe);
    run_step.dependOn(&run_cmd.step);

    run_cmd.step.dependOn(b.getInstallStep());

    if (b.args) |args| {
        run_cmd.addArgs(args);
    }

    const exe_tests = b.addTest(.{
        .root_module = exe.root_module,
    });

    // A run step that will run the second test executable.
    const run_exe_tests = b.addRunArtifact(exe_tests);

    const test_step = b.step("test", "Run tests");
    test_step.dependOn(&run_exe_tests.step);
}
