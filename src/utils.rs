use opencv::{core::*, highgui::*, prelude::*, videoio::*};
use std::{thread, time};

pub fn show_frame(name: &str, frame: &Mat) -> opencv::Result<()> {
    let window = name;
    named_window(window, WND_PROP_FULLSCREEN)?;
    set_window_property(name, WND_PROP_FULLSCREEN, WINDOW_FULLSCREEN as f64)?;
    imshow(window, &frame)?;
    Ok(())
}

pub fn destroy_frame(name: &str) -> opencv::Result<()> {
    let window = name;
    destroy_window(window)?;
    Ok(())
}

fn cam_show() -> opencv::Result<()> {
    // shows camera frames on screen until any key is pressed
    let mut cap = VideoCapture::new(0, CAP_V4L2)?;
    let opened = VideoCapture::is_opened(&cap)?;
    if !opened {
        panic!("Unable to open default camera");
    }
    thread::sleep(time::Duration::from_millis(2)); // camera warm up
    loop {
        let mut frame = Mat::default()?;
        cap.read(&mut frame)?;
        imshow("camera", &frame)?;

        let key = wait_key(10)?;
        if key > 0 && key != 255 {
            break;
        }
    }
    cap.release()?;

    destroy_frame("camera")?;

    Ok(())
}

fn cam_write() -> opencv::Result<()> {
    // save camera frames to an .avi output file until any key is pressed
    let mut cap = VideoCapture::new(0, CAP_V4L2)?;
    let opened = VideoCapture::is_opened(&cap)?;
    if !opened {
        panic!("Unable to open default camera");
    }
    let camera_res = Size {
        width: cap.get(CAP_PROP_FRAME_WIDTH)? as i32,
        height: cap.get(CAP_PROP_FRAME_HEIGHT)? as i32,
    };
    let fourcc = VideoWriter::fourcc('M' as u8, 'J' as u8, 'P' as u8, 'G' as u8)?;
    let mut writer = VideoWriter::new("output.avi", fourcc, 15.0, camera_res, true)?;
    thread::sleep(time::Duration::from_millis(2)); // camera warm up
    loop {
        let mut frame = Mat::default()?;
        cap.read(&mut frame)?;
        writer.write(&frame)?;
        imshow("camera", &frame)?;

        let key = wait_key(10)?;
        if key > 0 && key != 255 {
            break;
        }
    }
    cap.release()?;
    writer.release()?;

    destroy_frame("camera")?;

    Ok(())
}
