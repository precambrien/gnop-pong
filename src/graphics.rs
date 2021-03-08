use opencv::{core::*, imgproc::*};

// Fonts and color settings
const BG_COLOR: RGB = RGB { r: 0, g: 0, b: 0 };
const OBJ_COLOR: RGB = RGB {
    r: 255,
    g: 153,
    b: 204,
};
const SCORE_COLOR: RGB = RGB {
    r: 255,
    g: 255,
    b: 0,
};
const DEFAULT_FONT: i32 = FONT_HERSHEY_SIMPLEX;

struct RGB {
    r: u8,
    g: u8,
    b: u8,
}

pub struct Graphics {
    pub bg_color: Scalar,
    pub obj_color: Scalar,
    pub score_color: Scalar,
    pub font: i32,
    pub score_pos_left: Point,
    pub score_pos_right: Point,
}

impl Graphics {
    pub fn init(screen: Size) -> Graphics {
        Graphics {
            bg_color: Scalar::new(BG_COLOR.b as f64, BG_COLOR.g as f64, BG_COLOR.r as f64, 0.0),
            obj_color: Scalar::new(
                OBJ_COLOR.b as f64,
                OBJ_COLOR.g as f64,
                OBJ_COLOR.r as f64,
                0.0,
            ),
            score_color: Scalar::new(
                SCORE_COLOR.b as f64,
                SCORE_COLOR.g as f64,
                SCORE_COLOR.r as f64,
                0.0,
            ),
            font: DEFAULT_FONT,
            score_pos_left: Point::new(3 * screen.width / 8 as i32, screen.height / 8 as i32),
            score_pos_right: Point::new(5 * screen.width / 8 as i32, screen.height / 8 as i32),
        }
    }
}
