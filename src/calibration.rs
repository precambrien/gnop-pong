extern crate opencv;
use opencv::{calib3d::*, core::*, highgui::*, imgcodecs::*, prelude::*, types::*, videoio::*};
use std::{thread, time};

use crate::errors::Error;
use crate::utils::*;

const BOARD_VERTICES_W: u8 = 9;
const BOARD_VERTICES_H: u8 = 5;
const BOARD_SQUARE_LENGTH: f32 = 0.13; // in meters
const REQUIRED_MARKERS: usize = 3;

#[derive(Debug)]
pub struct CalibrationData {
    pub camera_matrix: Mat,
    pub distortion_coeffs: Mat,
}

pub fn camera_calibrate(resolution: Size) -> Result<CalibrationData, Error> {
    let board = imread(
        "resources/calibration.png",
        IMREAD_UNCHANGED,
    )?;
    let mut cam = VideoCapture::new(0, CAP_V4L2)?;
    let opened = VideoCapture::is_opened(&cam)?;
    if !opened {
        panic!("Unable to open default camera");
    }
    thread::sleep(time::Duration::from_millis(2)); // camera warm up

    let template_obj_points =
        calculate_obj_points(BOARD_SQUARE_LENGTH, BOARD_VERTICES_W, BOARD_VERTICES_H);
    let mut obj_points = Vec::<VectorOfPoint3f>::new();
    let mut img_points = Vec::<VectorOfPoint2f>::new();

    loop {
        let mut frame = Mat::default()?;
        cam.read(&mut frame)?;

        show_frame("calibration", &board)?;
        let mut corners = VectorOfPoint2f::new();
        let ret = find_chessboard_corners_sb(
            &frame,
            Size::new(BOARD_VERTICES_W as i32, BOARD_VERTICES_H as i32),
            &mut corners,
            0,
        )?;
        if ret {
            obj_points.push(VectorOfPoint3f::from_iter(template_obj_points.clone()));
            img_points.push(corners);
        }

        let key = wait_key(10)?;
        if key > 0 && key != 255 {
            break;
        }

        if img_points.len() >= REQUIRED_MARKERS {
            break;
        }
    }
    destroy_frame("calibration")?;

    let mut rvec = Mat::default()?;
    let mut tvec = Mat::default()?;
    let mut matrix = Mat::default()?;
    let mut dist_coeffs = Mat::default()?;
    calibrate_camera(
        &VectorOfVectorOfPoint3f::from_iter(obj_points),
        &VectorOfVectorOfPoint2f::from_iter(img_points),
        resolution,
        &mut matrix,
        &mut dist_coeffs,
        &mut rvec,
        &mut tvec,
        0,
        TermCriteria::new(
            TermCriteria_Type::COUNT as i32 + TermCriteria_Type::EPS as i32,
            30,
            std::f64::EPSILON,
        )?,
    )?;

    let c = CalibrationData {
        camera_matrix: matrix,
        distortion_coeffs: dist_coeffs,
    };

    Ok(c)
}

fn calculate_obj_points(square_size: f32, width: u8, height: u8) -> Vec<Point3f> {
    let mut obj_points = Vec::with_capacity((width * height) as usize);

    for row in 0..height {
        for col in 0..width {
            obj_points.push(Point3f::new(
                square_size * col as f32,
                square_size * row as f32,
                0.,
            ));
        }
    }

    obj_points
}
