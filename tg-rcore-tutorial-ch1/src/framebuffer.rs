//! Framebuffer 像素绘制工具。

/// 屏幕上的一个像素坐标。
#[derive(Copy, Clone)]
pub(crate) struct Point {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

impl Point {
    /// 创建一个点。
    pub(crate) const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// RGB 颜色。
#[derive(Copy, Clone)]
pub(crate) struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    /// 创建一个 RGB 颜色。
    pub(crate) const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// 简单的 BGRA framebuffer 视图。
pub(crate) struct FrameBuffer<'a> {
    pixels: &'a mut [u8],
    width: usize,
    height: usize,
}

impl<'a> FrameBuffer<'a> {
    /// 创建一个 framebuffer 包装器。
    pub(crate) fn new(pixels: &'a mut [u8], width: usize, height: usize) -> Self {
        assert!(pixels.len() >= width.saturating_mul(height).saturating_mul(4));
        Self {
            pixels,
            width,
            height,
        }
    }

    /// 返回 framebuffer 的像素尺寸。
    pub(crate) fn size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// 用单色清屏。
    pub(crate) fn clear(&mut self, color: Color) {
        for pixel in self.pixels.chunks_exact_mut(4) {
            pixel[0] = color.b;
            pixel[1] = color.g;
            pixel[2] = color.r;
            pixel[3] = 0xff;
        }
    }

    /// 填充一个凸多边形。
    pub(crate) fn fill_convex_polygon(&mut self, vertices: &[Point], color: Color) {
        if vertices.len() < 3 {
            return;
        }
        for i in 1..vertices.len() - 1 {
            self.fill_triangle(vertices[0], vertices[i], vertices[i + 1], color);
        }
    }

    /// 填充一个三角形。
    fn fill_triangle(&mut self, a: Point, b: Point, c: Point, color: Color) {
        let area = orient(a, b, c);
        if area == 0 {
            return;
        }

        let min_x = a.x.min(b.x).min(c.x).max(0) as usize;
        let max_x = a.x.max(b.x).max(c.x).min(self.width as i32 - 1) as usize;
        let min_y = a.y.min(b.y).min(c.y).max(0) as usize;
        let max_y = a.y.max(b.y).max(c.y).min(self.height as i32 - 1) as usize;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let p = Point::new(x as i32, y as i32);
                let w0 = orient(b, c, p);
                let w1 = orient(c, a, p);
                let w2 = orient(a, b, p);
                if (area > 0 && w0 >= 0 && w1 >= 0 && w2 >= 0)
                    || (area < 0 && w0 <= 0 && w1 <= 0 && w2 <= 0)
                {
                    self.set_pixel(x, y, color);
                }
            }
        }
    }

    /// 设置单个像素。
    fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        let offset = (y * self.width + x) * 4;
        self.pixels[offset] = color.b;
        self.pixels[offset + 1] = color.g;
        self.pixels[offset + 2] = color.r;
        self.pixels[offset + 3] = 0xff;
    }
}

/// 计算有向面积，用于判断点与边的相对位置。
fn orient(a: Point, b: Point, c: Point) -> i64 {
    let abx = (b.x - a.x) as i64;
    let aby = (b.y - a.y) as i64;
    let acx = (c.x - a.x) as i64;
    let acy = (c.y - a.y) as i64;
    abx * acy - aby * acx
}
