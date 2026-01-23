const ffmpeg_err = error{
    CannotFoundBestStream,
    CannotFoundCodec,
    CannotAllocateCodecContext,
    GetSwsContextFailed,
    AllocateFrameFailed,
};

const cli_err = error{ CannotFoundFile, InvalidRange };

const VideoReadFrameError = error{
    EOF,
};
