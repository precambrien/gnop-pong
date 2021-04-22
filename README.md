# Gnop-pong &middot;
Gnop-pong is a "virtual reality" pong game using a camera + projector setup. The game is projected on a flat surface on which the camera detect shapes, allowing to play with hands or objects. 

## Usage:

`cargo run`  
press any key to escape.

### Demo
<p align="center">
    <img src="https://github.com/precambrien/precambrien_public/blob/main/gnop-pong-demo.gif" alt="animated">
</p>
In this demo, the projector and camera are attached to the ceiling and a piece of white cloth is laid on the floor below. The game was started in single player mode. The projector displays the ball and the score while the camera detects shape collisions.
 

### Options:
`-r`: projector resolution in WxH format (default is 1920x1080)  
`-s`: solo mode (single player)  
`-f`: fullscreen mode: game is projected at the full projector resolution (no smaller playing area)  
`-d` or `-dd`: debug/verbose level  

## Detailled steps:
* camera calibration and undistortion using a projected chessboard
* detection of the projector area by displaying a white full screen
* (optional) detection of a smaller playing area that will demarcate the game boundaries. It can be a sheet placed on the ground or a painted rectangle. However, this area must be rectangular (4 corners) and included in the projector area.
* detection of moving shapes using a canny threshold and a contour detection
* minimalist game display of scores and a ball at the adapted scale.

## Requirements 
* Rust (https://www.rust-lang.org/tools/install)
* OpenCV V4 and `clang`,`llvm` packages needed for rust bindings (follow https://github.com/twistedfall/opencv-rust instructions). 
* a projector
* a camera. The demo was made with a Raspberry Pi with a picamera taped to the front of the projector. You may need to adapt the camera driver used by openCV's `VideoCapture` according to your model.
