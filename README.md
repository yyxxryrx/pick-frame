# pick-frame

一个简单的视频帧提取工具

此工具通过使用 `avformat_seek_file` 跳转到目标帧来实现快速定位，因此通常具有较快的速度。

## 依赖项

- zig
- rust
- vcpkg
- ffmpeg

## 构建

> 需要安装 vcpkg 并设置 `VCPKG_ROOT` 环境变量

> 同样需要安装 `ffmpeg`

### 1. 安装 ffmpeg

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
zig build --release
```

## 使用

```bash
Usage: pick-frame.exe [OPTIONS] --input <INPUT> [OUTPUT]

Arguments:
  [OUTPUT]  Output path [default: .]

Options:
  -i, --input <INPUT>  The video path
  -f, --from <FROM>    possible format: [xxx, xx:xx.xx, end] [default: 0]
  -t, --to <TO>        possible format: [xxx, xx:xx.xx, end] [default: end]
  -h, --help           Print help
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

# pick frames from 01:10:10.100 to 01:10:20.200 to output directory
pick-frame.exe -i video.mp4 -f 01:10:10.100 -t 01:10:20.200 output
```
