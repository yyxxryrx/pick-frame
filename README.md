# pick-frame

## Build

> requires vcpkg installed and set `VCPKG_ROOT` environment variable

> also requires `ffmpeg` installed

### 1. Install ffmpeg

> you can ignore this step if you have ffmpeg installed

```bash
vcpkg install ffmpeg[core,avcodec,avdevice,avformat,avfilter,swresample,swscale,x264,gpl]:x64-windows-static

```

### 2. Build

```bash
zig build
```

if you want to build release version

```bash
zig build --release
```
