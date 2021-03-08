use getopts::{Matches, Options};
use opencv::{calib3d::*, core::*, highgui::*, imgproc::*, prelude::*, types::*, videoio::*};
use std::cmp::*;
use std::{env, thread, time};

use gnop_pong::calibration::*;
use gnop_pong::detection::*;
use gnop_pong::game::*;
use gnop_pong::utils::*;

const DEFAULT_SCREEN_WIDTH: i32 = 1920;
const DEFAULT_SCREEN_HEIGHT: i32 = 1080;
const DEFAULT_CAM_WIDTH: i32 = 640;
const DEFAULT_CAM_HEIGHT: i32 = 480;

#[derive(Debug)]
pub struct Args {
    projector_res: Size,
    flag_fullscreen: bool,
    flag_solo: bool,
    dbg_level: usize,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();
    opts.optopt(
        "r",
        "res",
        "projector resolution (width x height) \n default resolution: 1920x1080",
        "WxH",
    );
    opts.optflag(
        "f",
        "fullscreen",
        "game projected on full screen, no smaller playing area detection",
    );
    opts.optflag("s", "solo", "single player");
    opts.optflagmulti("d", "", "debug execution \n -d shows some debug info \n -dd save detected contours's shapes in a video file (MJPG codec)");
    opts.optflag("h", "help", "prints usage");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };
    if matches.opt_present("h") {
        print_usage(&args[0].clone(), opts);
        return;
    }
    let args = parse_args(&matches);

    if args.dbg_level >= 1 {
        println!(
            "Projector resolution: {}x{}, fullscreen: {:?}, single-player: {:?}",
            args.projector_res.width,
            args.projector_res.height,
            args.flag_fullscreen,
            args.flag_solo
        );
    }

    run(&args).unwrap();
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn parse_args(matches: &Matches) -> Args {
    // Extracts the two firsts i32 separated by 'x', default resolution is used if any error encountered
    let default_res = format!("{}x{}", DEFAULT_SCREEN_WIDTH, DEFAULT_SCREEN_HEIGHT);
    let (width, height) = match matches.opt_default("r", &default_res) {
        Some(s) => {
            let mut parts = s.split("x").map(|s| s.parse::<i32>());
            match (parts.next(), parts.next()) {
                (Some(Ok(h)), Some(Ok(w))) => (h, w),
                _ => ((DEFAULT_SCREEN_WIDTH, DEFAULT_SCREEN_HEIGHT)),
            }
        }
        None => (DEFAULT_SCREEN_WIDTH, DEFAULT_SCREEN_HEIGHT),
    };

    let args = Args {
        projector_res: Size { width, height },
        flag_fullscreen: matches.opt_present("f"),
        flag_solo: matches.opt_present("s"),
        dbg_level: usize::min(2, matches.opt_count("d")),
    };

    args
}

fn run(args: &Args) -> opencv::Result<()> {
    let projector_res = args.projector_res;
    let camera_res = Size {
        width: DEFAULT_CAM_WIDTH,
        height: DEFAULT_CAM_HEIGHT,
    };

    let c = camera_calibrate(camera_res).unwrap();
    let mut valid_pix_roi = Rect::new(0, 0, 0, 0);
    let optimal_matrix = get_optimal_new_camera_matrix(
        &c.camera_matrix,
        &c.distortion_coeffs,
        camera_res,
        0.0,
        camera_res,
        &mut valid_pix_roi,
        false,
    )?;

    let mut cam = VideoCapture::new(0, CAP_V4L2)?;
    let opened = VideoCapture::is_opened(&cam)?;
    if !opened {
        panic!("Unable to open default camera");
    }
    thread::sleep(time::Duration::from_millis(2)); // camera warm up

    // projector and playing area detections
    let (screen, area) = match get_unwarped_areas(&mut cam, projector_res, args.flag_fullscreen) {
        Ok((s, p)) => (s, p),
        Err(r) => panic!(r.to_string()),
    };

    // game init
    let game_res = Size {
        width: ((area.size.width as f64 / screen.size.width as f64) * projector_res.width as f64)
            as i32,
        height: ((area.size.height as f64 / screen.size.height as f64)
            * projector_res.height as f64) as i32,
    };
    let mut game = Game::new(game_res, args.flag_solo);
    if args.dbg_level >= 1 {
        println!(
            "Starting game at {}x{} resolution",
            game_res.width, game_res.height
        );
    }

    // game mat is placed inside a smaller roi on the output mat
    let mut output_mat =
        Mat::zeros(projector_res.height, projector_res.width, CV_8UC3)?.to_mat()?;
    output_mat.set_to(&Scalar::new(0.0, 0.0, 0.0, 0.0), &no_array()?)?;
    let mut game_mat = Mat::zeros(game_res.height, game_res.width, CV_8UC3)?.to_mat()?;
    let game_roi = get_game_roi(projector_res, game_res, &screen, &area);
    let mut region = opencv::prelude::Mat::roi(&output_mat, game_roi)?;

    // ratio (unwarped image -> game pixels) for shape scaling
    let x_ratio = (game_res.width as f64 / area.unwarped_size.width as f64) as f64;
    let y_ratio = (game_res.height as f64 / area.unwarped_size.height as f64) as f64;

    let mut writer = if args.dbg_level == 2 {
        let fourcc = VideoWriter::fourcc('M' as u8, 'J' as u8, 'P' as u8, 'G' as u8)?;
        Some(VideoWriter::new(
            "./debug.avi",
            fourcc,
            15.0,
            area.size,
            true,
        )?)
    } else {
        None
    };

    loop {
        let mut frame = Mat::default()?;
        cam.read(&mut frame)?;

        let mut undistorted = Mat::default()?;
        undistort(
            &frame,
            &mut undistorted,
            &c.camera_matrix,
            &c.distortion_coeffs,
            &optimal_matrix,
        )?;
        let mut unwarped = Mat::default()?;
        warp_perspective(
            &frame,
            &mut unwarped,
            &area.unwarped_mat,
            area.size,
            INTER_LINEAR,
            BORDER_CONSTANT,
            Scalar::default(),
        )?;

        let mut scaled_shapes = VectorOfRotatedRect::new();
        let shapes = shape_detect(&mut unwarped)?;
        for shape in 0..shapes.len() {
            let shape = shapes.get(shape)?;
            let scaled = scale_shape(&shape, x_ratio, y_ratio)?;
            scaled_shapes.push(scaled);
            if args.dbg_level == 2 {
                let mut sc_vertices: [Point2f; 4] = [
                    Point2f::default(),
                    Point2f::default(),
                    Point2f::default(),
                    Point2f::default(),
                ];
                shape.points(&mut sc_vertices)?;
                for i in 0..4 {
                    line(
                        &mut unwarped,
                        sc_vertices[i].to::<i32>().unwrap(),
                        sc_vertices[(i + 1) % 4].to::<i32>().unwrap(),
                        Scalar::new(0.0, 0.0, 0.0, 0.0),
                        1,
                        LINE_8,
                        0,
                    )?;
                }
            }
        }
        if let Some(ref mut w) = writer {
            w.write(&unwarped)?;
        }

        game.update(&scaled_shapes)?;
        game.draw(&mut game_mat)?;
        game_mat.copy_to(&mut region)?;
        show_frame("game", &output_mat)?;

        let key = wait_key(10)?;
        if key > 0 && key != 255 {
            break;
        }
    }
    destroy_frame("game")?;
    cam.release()?;
    if let Some(mut writer) = writer {
        writer.release()?;
    }

    Ok(())
}
