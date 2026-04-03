// 教程说明：
// 这三个常量对应类 Unix 约定的标准文件描述符。

/// 标准输入文件描述符。
pub const STDIN: usize = 0;
/// 标准输出文件描述符。
pub const STDOUT: usize = 1;
/// 标准错误/调试输出文件描述符。
pub const STDDEBUG: usize = 2;

/// 用户态可见的 framebuffer 基本信息。
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameBufferInfo {
    /// 像素宽度
    pub width: u32,
    /// 像素高度
    pub height: u32,
    /// 每行字节数
    pub stride: u32,
    /// 每像素字节数
    pub bytes_per_pixel: u32,
}
