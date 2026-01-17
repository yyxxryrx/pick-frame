# pick-frame

## Build

> requires vcpkg installed and set `VCPKG_ROOT` environment variable

### 1. Install ffmpeg

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
