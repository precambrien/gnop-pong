use opencv::{core::*, highgui::*, imgproc::*, prelude::*, types::*, videoio::*};
use std::cmp::*;
use std::sync::mpsc::channel;
use std::thread;

use crate::errors::{Error, ErrorKind};
use crate::utils::*;

#[derive(Debug, Clone)]
pub struct Area {
    pub origin: Point2f,
    pub size: Size,
    pub unwarped_size: Size,
    pub unwarped_mat: Mat,
}

enum ScreenColor {
    Black,
    White,
}

pub fn get_unwarped_areas(
    cam: &mut VideoCapture,
    projector_res: Size,
    fullscreen: bool,
) -> Result<(Area, Area), Error> {
    let (tx, rx) = channel();
    let calibration = thread::spawn(move || -> opencv::Result<()> {
        let mut calibration =
            Mat::zeros(projector_res.height, projector_res.width, CV_8UC3)?.to_mat()?;
        calibration.set_to(&Scalar::new(255.0, 255.0, 255.0, 0.0), &no_array()?)?;
        loop {
            match rx.try_recv() {
                Ok(Some(ScreenColor::Black)) => {
                    calibration.set_to(&Scalar::new(0.0, 0.0, 0.0, 0.0), &no_array()?)?;
                }
                Ok(Some(ScreenColor::White)) => {
                    calibration.set_to(&Scalar::new(255.0, 255.0, 255.0, 0.0), &no_array()?)?;
                }
                Ok(None) => {
                    destroy_frame("calibration")?;
                    break;
                }
                Err(_) => {}
            }
            show_frame("calibration", &calibration)?;

            let key = wait_key(10)?;
            if key > 0 && key != 255 {
                destroy_frame("calibration")?;
                break;
            }
        }

        Ok(())
    });
    let mut tmp = Mat::default()?;
    for _i in 0..30 {
        cam.read(&mut tmp)?;
    }
    let screen = match detect_playing_area(cam, fullscreen) {
        Err(r) => return Err(r),
        Ok(a) => a,
    };
    let area;
    if !fullscreen {
        // playing area does not equals screen area -> area detection needed
        tx.send(Some(ScreenColor::Black)).unwrap();
        for _i in 0..30 {
            cam.read(&mut tmp)?;
        }
        area = match detect_playing_area(cam, true) {
            Err(_) => panic!("Can't detect area"),
            Ok(a) => a,
        };
        if area.size.width > screen.size.width || area.size.height > screen.size.height {
            return Err(Error::DetectionError(ErrorKind::AreaBiggerThanScreen));
        }
    } else {
        area = screen.clone();
    }

    tx.send(None).unwrap();
    let _res = calibration.join();

    Ok((screen, area))
}

pub fn detect_playing_area(cam: &mut VideoCapture, perspective: bool) -> Result<Area, Error> {
    loop {
        let mut frame = Mat::default()?;
        cam.read(&mut frame)?;
        let mut gray = Mat::default()?;
        let mut blurred = Mat::default()?;
        cvt_color(&frame, &mut gray, COLOR_BGR2GRAY, 0)?;
        gaussian_blur(
            &gray,
            &mut blurred,
            Size::new(3, 3),
            0.0,
            0.0,
            BORDER_DEFAULT,
        )?;
        let mut canny_output = Mat::default()?;
        let threshold_min = 100.0;
        let threshold_max = 255.0;
        canny(
            &blurred,
            &mut canny_output,
            threshold_min,
            threshold_max,
            3,
            false,
        )?;
        let mut contours = VectorOfVectorOfPoint::new();
        find_contours(
            &canny_output,
            &mut contours,
            RETR_EXTERNAL,
            CHAIN_APPROX_SIMPLE,
            Point::new(0, 0),
        )?;
        let mut sorted_contours = contours.to_vec();
        sorted_contours.sort_by(|a, b| {
            contour_area(&b, false)
                .unwrap()
                .partial_cmp(&contour_area(&a, false).unwrap())
                .unwrap_or(Ordering::Equal)
        });
        let sorted = VectorOfVectorOfPoint::from(sorted_contours);

        for index in 0..sorted.len() {
            let c = sorted.get(index)?;
            let perimeter = arc_length(&c, true)?;
            if perimeter > 100.0 {
                let mut polygon = VectorOfPoint::new();
                approx_poly_dp(&c, &mut polygon, 0.02 * perimeter, true)?;
                if polygon.len() == 4 {
                    let roi_corners = order_corners(&polygon);
                    let origin = roi_corners.get(0)?;
                    let org_size = Size::new(
                        (roi_corners.get(1)?.x - roi_corners.get(0)?.x) as i32,
                        (roi_corners.get(3)?.y - roi_corners.get(0)?.y) as i32,
                    );
                    let m;
                    let unwarped_size;
                    if perspective {
                        let dst_corners = get_destination_corners(&roi_corners)?;
                        unwarped_size =
                            Size::new(dst_corners.get(2)?.x as i32, dst_corners.get(2)?.y as i32);
                        let roi_corners_mat = Mat::from_exact_iter(roi_corners.iter())?;
                        let dst_corners_mat = Mat::from_exact_iter(dst_corners.iter())?;
                        m = get_perspective_transform(
                            &roi_corners_mat,
                            &dst_corners_mat,
                            DECOMP_LU,
                        )?;
                    } else {
                        m = Mat::default()?;
                        unwarped_size = Size::new(0, 0);
                    }
                    let area = Area {
                        unwarped_mat: m,
                        unwarped_size: unwarped_size,
                        origin: origin,
                        size: org_size,
                    };
                    return Ok(area);
                }
            }
        }
    }
}

fn order_corners(vec: &VectorOfPoint) -> VectorOfPoint2f {
    // orders shape corners clockwise (top left, top right, bottom right, bottom left)
    let mut points = vec.to_vec();
    points.sort_by(|p0, p1| p0.x.cmp(&p1.x));
    let mut leftmost = points[0..2].to_vec();
    let mut rightmost = points[2..4].to_vec();
    leftmost.sort_by(|p0, p1| p0.y.cmp(&p1.y));
    rightmost.sort_by(|p0, p1| p0.y.cmp(&p1.y));
    let mut v = VectorOfPoint2f::with_capacity(4);
    v.push(Point2f::new(leftmost[0].x as f32, leftmost[0].y as f32)); // top left
    v.push(Point2f::new(rightmost[0].x as f32, rightmost[0].y as f32)); // rop right
    v.push(Point2f::new(rightmost[1].x as f32, rightmost[1].y as f32)); // bottom right
    v.push(Point2f::new(leftmost[1].x as f32, leftmost[1].y as f32)); // bottom left

    v
}

fn get_destination_corners(src_vec: &VectorOfPoint2f) -> Result<VectorOfPoint2f, opencv::Error> {
    let mut dst = VectorOfPoint2f::with_capacity(4);
    for _ in 0..4 {
        dst.push(Point2f::default());
    }

    dst.set(0, Point2f::new(0.0, 0.0))?;
    dst.set(
        1,
        Point2f::new(
            (src_vec.get(0)? - src_vec.get(1)?)
                .norm()
                .max((src_vec.get(2)? - src_vec.get(3)?).norm()) as f32,
            0.0,
        ),
    )?;
    dst.set(
        2,
        Point2f::new(
            (src_vec.get(0)? - src_vec.get(1)?)
                .norm()
                .max((src_vec.get(2)? - src_vec.get(3)?).norm()) as f32,
            (src_vec.get(1)? - src_vec.get(2)?)
                .norm()
                .max((src_vec.get(3)? - src_vec.get(0)?).norm()) as f32,
        ),
    )?;
    dst.set(
        3,
        Point2f::new(
            0.0,
            (src_vec.get(1)? - src_vec.get(2)?)
                .norm()
                .max((src_vec.get(3)? - src_vec.get(0)?).norm()) as f32,
        ),
    )?;

    Ok(dst)
}

pub fn shape_detect(img: &Mat) -> Result<VectorOfRotatedRect, opencv::Error> {
    // contour detection
    // mat priming: channels to gray, gaussian blur then canny on fixed threshold
    let mut rect = VectorOfRotatedRect::new();
    let mut gray = Mat::default()?;
    let mut blurred = Mat::default()?;
    let mut canny_output = Mat::default()?;
    cvt_color(&img, &mut gray, COLOR_BGR2GRAY, 0)?;
    gaussian_blur(
        &gray,
        &mut blurred,
        Size::new(3, 3),
        0.0,
        0.0,
        BORDER_DEFAULT,
    )?;

    let threshold_min = 120.0;
    let threshold_max = 255.0;
    canny(
        &blurred,
        &mut canny_output,
        threshold_min,
        threshold_max,
        3,
        false,
    )?;
    let mut contours = VectorOfVectorOfPoint::new();
    find_contours(
        &canny_output,
        &mut contours,
        RETR_EXTERNAL,
        CHAIN_APPROX_SIMPLE,
        Point::new(0, 0),
    )?;
    for index in 0..contours.len() {
        let c = contours.get(index)?;
        rect.push(min_area_rect(&c)?);
    }

    Ok(rect)
}

pub fn scale_shape(
    shape: &RotatedRect,
    x_ratio: f64,
    y_ratio: f64,
) -> Result<RotatedRect, opencv::Error> {
    let scaled_center = Point2f::new(
        shape.center().x * x_ratio as f32,
        shape.center().y * y_ratio as f32,
    );
    let scaled_size = Size2f::new(
        shape.size().width * x_ratio as f32,
        shape.size().height * y_ratio as f32,
    );
    return RotatedRect::new(scaled_center, scaled_size, shape.angle());
}

pub fn get_game_roi(
    projector_res: Size,
    game_res: Size,
    screen: &Area,
    playing_area: &Area,
) -> Rect {
    let pixel_ratio_w = projector_res.width as f64 / (screen.size.width as f64);
    let pixel_ratio_h = projector_res.height as f64 / (screen.size.height as f64);
    let game_origin_x =
        ((playing_area.origin.x as f64 - screen.origin.x as f64) * pixel_ratio_w) as i32;
    let game_origin_y =
        ((playing_area.origin.y as f64 - screen.origin.y as f64) * pixel_ratio_h) as i32;

    Rect::new(
        game_origin_x,
        game_origin_y,
        game_res.width,
        game_res.height,
    )
}
