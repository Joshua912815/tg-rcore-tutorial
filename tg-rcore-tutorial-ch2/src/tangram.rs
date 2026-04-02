//! Tangram “OS” 图案的分步场景描述。

use crate::framebuffer::{Color, FrameBuffer, Point};

const LOGICAL_WIDTH: i32 = 1024;
const LOGICAL_HEIGHT: i32 = 768;

const BACKGROUND: Color = Color::rgb(250, 250, 250);
const RED: Color = Color::rgb(217, 18, 6);
const YELLOW: Color = Color::rgb(255, 202, 20);
const MAGENTA: Color = Color::rgb(219, 72, 213);
const BLUE: Color = Color::rgb(62, 31, 236);
const CYAN: Color = Color::rgb(35, 183, 224);
const GREEN: Color = Color::rgb(92, 249, 20);
const ORANGE: Color = Color::rgb(255, 146, 0);

const fn p(x: i32, y: i32) -> Point {
    Point::new(x, y)
}

const O_RED_TRIANGLE: [Point; 3] = [p(66, 89), p(189, 89), p(66, 212)];
const O_YELLOW_QUAD: [Point; 4] = [p(66, 212), p(189, 89), p(189, 337), p(66, 460)];
const O_MAGENTA_TRIANGLE: [Point; 3] = [p(189, 89), p(438, 89), p(438, 337)];
const O_BLUE_QUAD: [Point; 4] = [p(313, 212), p(438, 337), p(438, 583), p(313, 460)];
const O_GREEN_DIAMOND: [Point; 4] = [p(189, 583), p(313, 460), p(438, 583), p(313, 706)];
const O_CYAN_QUAD: [Point; 4] = [p(66, 460), p(189, 583), p(313, 706), p(66, 706)];

const S_CYAN_TRIANGLE: [Point; 3] = [p(622, 212), p(746, 89), p(746, 337)];
const S_BLUE_TOP_TRIANGLE: [Point; 3] = [p(746, 89), p(870, 89), p(870, 212)];
const S_MAGENTA_TOP_QUAD: [Point; 4] = [p(870, 65), p(994, 65), p(994, 138), p(870, 212)];
const S_GREEN_SQUARE: [Point; 4] = [p(746, 337), p(870, 337), p(870, 460), p(746, 460)];
const S_MAGENTA_RIGHT_TRIANGLE: [Point; 3] = [p(870, 337), p(994, 460), p(870, 583)];
const S_ORANGE_PARALLELOGRAM: [Point; 4] =
    [p(622, 583), p(746, 583), p(808, 706), p(684, 706)];
const S_BLUE_BOTTOM_TRIANGLE: [Point; 3] = [p(746, 583), p(870, 583), p(808, 706)];

struct Piece {
    color: Color,
    vertices: &'static [Point],
}

const PIECES: &[Piece] = &[
    Piece {
        color: RED,
        vertices: &O_RED_TRIANGLE,
    },
    Piece {
        color: YELLOW,
        vertices: &O_YELLOW_QUAD,
    },
    Piece {
        color: MAGENTA,
        vertices: &O_MAGENTA_TRIANGLE,
    },
    Piece {
        color: BLUE,
        vertices: &O_BLUE_QUAD,
    },
    Piece {
        color: GREEN,
        vertices: &O_GREEN_DIAMOND,
    },
    Piece {
        color: CYAN,
        vertices: &O_CYAN_QUAD,
    },
    Piece {
        color: CYAN,
        vertices: &S_CYAN_TRIANGLE,
    },
    Piece {
        color: BLUE,
        vertices: &S_BLUE_TOP_TRIANGLE,
    },
    Piece {
        color: MAGENTA,
        vertices: &S_MAGENTA_TOP_QUAD,
    },
    Piece {
        color: GREEN,
        vertices: &S_GREEN_SQUARE,
    },
    Piece {
        color: MAGENTA,
        vertices: &S_MAGENTA_RIGHT_TRIANGLE,
    },
    Piece {
        color: ORANGE,
        vertices: &S_ORANGE_PARALLELOGRAM,
    },
    Piece {
        color: BLUE,
        vertices: &S_BLUE_BOTTOM_TRIANGLE,
    },
];

/// 返回 Tangram 分块数量。
pub(crate) fn piece_count() -> usize {
    PIECES.len()
}

/// 清空场景背景。
pub(crate) fn clear_scene(framebuffer: &mut FrameBuffer<'_>) {
    framebuffer.clear(BACKGROUND);
}

/// 绘制指定编号的一块 Tangram。
pub(crate) fn draw_piece(framebuffer: &mut FrameBuffer<'_>, piece_id: usize) {
    let Some(piece) = PIECES.get(piece_id) else {
        return;
    };

    let (width, height) = framebuffer.size();
    let width = width as i32;
    let height = height as i32;

    let width_scaled = width as i64 * LOGICAL_HEIGHT as i64;
    let height_scaled = height as i64 * LOGICAL_WIDTH as i64;
    let (scale_num, scale_den) = if width_scaled <= height_scaled {
        (width as i64, LOGICAL_WIDTH as i64)
    } else {
        (height as i64, LOGICAL_HEIGHT as i64)
    };

    let draw_width = (LOGICAL_WIDTH as i64 * scale_num / scale_den) as i32;
    let draw_height = (LOGICAL_HEIGHT as i64 * scale_num / scale_den) as i32;
    let offset_x = (width - draw_width) / 2;
    let offset_y = (height - draw_height) / 2;

    let mut transformed = [Point::new(0, 0); 4];
    for (dst, src) in transformed.iter_mut().zip(piece.vertices.iter()) {
        *dst = transform(*src, scale_num, scale_den, offset_x, offset_y);
    }
    framebuffer.fill_convex_polygon(&transformed[..piece.vertices.len()], piece.color);
}

fn transform(point: Point, scale_num: i64, scale_den: i64, offset_x: i32, offset_y: i32) -> Point {
    Point::new(
        offset_x + (point.x as i64 * scale_num / scale_den) as i32,
        offset_y + (point.y as i64 * scale_num / scale_den) as i32,
    )
}
