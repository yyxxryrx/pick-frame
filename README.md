# pick-frame

[![Source](https://img.shields.io/badge/Source-GitHub-blue?logo=github)](https://github.com/yyxxryrx/pick-frame)
[![MIT License](https://img.shields.io/badge/License-MIT-green)](LICENSE)
[![Mirror](https://img.shields.io/badge/Mirror-GitLab-orange?logo=gitlab)](https://gitlab.com/yyxxryrx/pick-frame)

一个简单的视频帧提取工具

此工具通过使用 `avformat_seek_file` 跳转到目标帧来实现快速定位，因此通常具有较快的速度。

还未完成的：

> 当前还是开发中，语法随时会变，而且也不一定完全实现了所有的功能

- 硬件加速
- 支持 **conan** 而不仅是 **vcpkg**

已经完成的：

- 时间表达式，例如 `end - 10s + 1f` 这种，详细见 [lexer.rs](lib/arg/src/lexer.rs) 和 [tui.rs](lib/arg/src/tui.rs)
- 跨平台，而不仅限 Windows

## 依赖项

- zig
- rust
- vcpkg
- ffmpeg

如需查看具体依赖信息，请看 [详细依赖项](#详细依赖项)

## 下载

> 目前只提供静态编译版本，如需动态编译，请自行编译项目，谢谢

[Release](https://github.com/yyxxryrx/pick-frame/releases/latest)

## 构建 - 静态编译

> 需要安装 vcpkg 并设置 `VCPKG_ROOT` 环境变量，或者设置 `-Dvcpkg-path` 参数

> 同样需要安装 `ffmpeg`

### 1. 安装 ffmpeg（静态库）

> 如果已安装 ffmpeg，可以跳过此步骤

```bash
vcpkg install ffmpeg[core,avcodec,avdevice,avformat,avfilter,swresample,swscale,x264,gpl]:x64-windows-static
```

### 2. 构建

```bash
zig build
```

如果要构建发布版本

```bash
zig build --release=[mode]
```

例如构建 `ReleaseFast` 模式

```bash
zig build --release=fast
```

## 构建 - 动态编译

> 需要安装 vcpkg 并设置 `VCPKG_ROOT` 环境变量，或者设置 `-Dvcpkg-path` 参数

> 同样需要安装 `ffmpeg`

### 1. 安装 ffmpeg（动态库）

> 如果已安装 ffmpeg，可以跳过此步骤

```bash
vcpkg install ffmpeg[core,avcodec,avdevice,avformat,avfilter,swresample,swscale,x264,gpl]:x64-windows
```

### 2. 构建

```bash
zig build -Ddynamic-link=true
```

如果要构建发布版本

```bash
zig build -Ddynamic-link=true --release=[mode]
```

例如构建 `ReleaseFast` 模式

```bash
zig build -Ddynamic-link=true --release=fast
```

### 3. 关于时间表达式

如果要使用时间表达式，请在构建时加上 `-Denable-time-expr=true` 参数，例如

```bash
zig build -Denable-time-expr=true --release=fast
```

该选项默认为 `false`

该选项允许使用时间表达式，该表达式有三个关键字

> 不允许循环引用
>
> 不允许重复使用

- `end`: 表示视频的结束
- `from`: 表示引用 `--from` 参数的时间
- `to`: 表示引用 `--to` 参数的时间

和两种类型

> 以下所有的 `0` 表示任意数字
> `[]` 表示可选部分

- `0f`: 表示帧数
- `0[.0]s`: 秒数
- `[0:]0:0[.0]`: 时间

特殊部分：

- `0.0`: 等价于 `0.0s`

支持两种运算符：

- `+`: 加
- `-`: 减

不支持隐式关键词，例如：`-10s` 不等价 `end - 10s`，会报错

因为不清楚 `-10s` 是 `end - 10s` 还是 `to - 10s` 还是 `from - 10s`，所以需要手动加上关键字

### 4. 使用系统FFmpeg库

> 使用此方法构建无须 **vcpkg** 和 `VCPKG_ROOT` 环境变量

首先确保你已在系统里安装的 **FFmpeg** 和 **pkg-config**

然后运行

```bash
zig build -Duse-system=true
```

进行构建，其他选项同上

## 使用

```bash
Usage: pick-frame.exe [OPTIONS] --input <INPUT> [OUTPUT]

Arguments:
  [OUTPUT]  Output path [default: .]

Options:
  -i, --input <INPUT>            The video path
  -f, --from <FROM>              possible format: [xxx, xx.xxs, xx:xx.xx, end] [default: 0]
  -t, --to <TO>                  possible format: [xxx, xx.xxs, xx:xx.xx, end] [default: end]
      --thread-count <Auto|num>  thread count for codec [default: auto]
      --format <FORMAT>          filename format [default: frame-%d.jpg]
  -h, --help                     Print help
```

## 示例

```bash
# pick all frames to current directory
pick-frame.exe -i video.mp4

# pick frames from fist frame to 10:10
pick-frame.exe -i video.mp4 -f 0 -t 10:10

# pick frames from 10:10 to 10:20
pick-frame.exe -i video.mp4 -f 10:10 -t 10:20

# pick frames from 10:10 to end
pick-frame.exe -i video.mp4 -f 10:10

# pick frames from start to 10s
pick-frame.exe -i vidoe.mp4 -t 10s

# pick frames from 01:10:10.100 to 01:10:20.200 to output directory
pick-frame.exe -i video.mp4 -f 01:10:10.100 -t 01:10:20.200 output
```

## 详细依赖项

| 序号 | 依赖名称   | 被哪个语言依赖 | 是否可选 |
| :--- | ---------- | -------------- | :------: |
| 1    | FFmpeg     | Zig            |    否    |
| 2    | clap       | Rust           |    否    |
| 3    | nom        | Rust           |    是    |
| 4    | nom_locate | Rust           |    是    |
| 5    | colored    | Rust           |    是    |

## 许可证

本项目采用MIT许可协议授权 — 详情请参阅[LICENSE](LICENSE)文件。
