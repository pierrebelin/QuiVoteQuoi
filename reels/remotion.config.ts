import { Config } from '@remotion/cli/config'

// Formats exigés par Instagram (Reel) et TikTok : H.264, yuv420p, 1080 × 1920 (T9).
Config.setCodec('h264')
Config.setPixelFormat('yuv420p')
Config.setVideoImageFormat('jpeg')
Config.setJpegQuality(90)
Config.setOverwriteOutput(true)
