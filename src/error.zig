pub const ffmpeg_err = error{
    CannotFoundBestStream,
    CannotFoundCodec,
    CannotAllocateCodecContext,
    GetSwsContextFailed,
    AllocateFrameFailed,
};

pub const cli_err = error{ CannotFoundFile, InvalidRange };

pub const VideoReadFrameError = error{
    EOF,
};
