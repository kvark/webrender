/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::{mem, ptr, slice};
use webrender_traits::{PathCommand};

#[cfg(feature = "skia")]
use skia;
use freetype::freetype::*;


pub struct PathPicture {
    outline: FT_Outline,
}

pub struct PathRenderer {
    lib: FT_Library,
}

impl PathRenderer {
    pub fn new() -> PathRenderer {
        let mut lib: FT_Library = ptr::null_mut();
        let result = unsafe {
            FT_Init_FreeType(&mut lib)
        };
        if !result.succeeded() {
            panic!("Unable to initialize FreeType library {}", result);
        }

        // TODO(gw): Check result of this to determine if freetype build supports subpixel.
        let result = unsafe {
            FT_Library_SetLcdFilter(lib, FT_LcdFilter::FT_LCD_FILTER_DEFAULT)
        };
        if !result.succeeded() {
            println!("WARN: Initializing a FreeType library build without subpixel AA enabled!");
        }

        PathRenderer {
            lib: lib,
        }
    }

    pub fn bake(&mut self, commands: &[PathCommand]) -> PathPicture {
        let (num_points, num_contours) = commands.iter().fold((0, 0),
            |(np, nc), com| match com {
                &PathCommand::MoveTo(_) => (np, nc),
                &PathCommand::ClosePath => (np+1, nc+1),
                &PathCommand::LineTo(_) => (np+2, nc+1),
            }
        );
        let mut outline: FT_Outline = unsafe { mem::zeroed() };
        let result = unsafe {
            FT_Outline_New(self.lib, num_points as u32, num_contours as u32, &mut outline)
        };
        if !result.succeeded() {
            panic!("Unable to create FT_Outline {}", result);
        }

        let (points, tags, contours) = unsafe {(
            slice::from_raw_parts_mut(outline.points, num_points),
            slice::from_raw_parts_mut(outline.tags, num_points),
            slice::from_raw_parts_mut(outline.contours, num_contours),
        )};
        let (_, _, in_contour, _) = commands.iter().fold((0, 0, false, FT_Vector{x:0,y:0}),
            |(mut np, mut nc, in_contour, cur), com| match com {
                &PathCommand::MoveTo(p) => {
                    if in_contour {
                        contours[nc as usize] = np-1;
                        nc += 1;
                    }
                    (np, nc, false, FT_Vector {
                        x: p.x as i64, //TODO: rounding
                        y: p.y as i64,
                    })
                },
                &PathCommand::ClosePath => {
                    if in_contour {
                        points[np as usize] = FT_Vector {
                            x: cur.x,
                            y: cur.y,
                        };
                        tags[np as usize] = 0x1;
                        contours[nc as usize] = np;
                        np += 1;
                        nc += 1;
                    }
                    (np, nc, false, cur)
                },
                &PathCommand::LineTo(p) => {
                    if !in_contour {
                        points[np as usize] = cur;
                        tags[np as usize] = 0x1; //TODO
                        np += 1;
                    }
                    points[np as usize] = FT_Vector {
                        x: p.x as i64, //TODO: rounding
                        y: p.y as i64,
                    };
                    tags[np as usize] = 0x1;
                    contours[nc as usize] = np;
                    (np+1, nc+1, true, FT_Vector {
                        x: p.x as i64, //TODO: rounding
                        y: p.y as i64,
                    })
                },
            }
        );
        assert!(!in_contour); //TODO: warning or return error

        PathPicture {
            outline: outline,
        }
    }

    pub fn draw(&mut self, picture: &mut PathPicture, width: u32, height: u32) -> Vec<u8> {
        let mut data = vec![0u8; (width * height) as usize];
        //TODO: use FT_Bitmap_Init ?
        let mut params = FT_Raster_Params {
            target: &mut FT_Bitmap {
                rows: height,
                width: width,
                pitch: width as i32,
                buffer: data.as_mut_ptr() as *mut _,
                num_grays: 0x100,
                pixel_mode: FT_PIXEL_MODE_GRAY,
                palette_mode: 0,
                palette: ptr::null_mut(),
            },
            source: &mut picture.outline as *mut _ as *mut _,
            flags: FT_RASTER_FLAG_AA, //TODO
            gray_spans: ptr::null_mut(),
            black_spans: ptr::null_mut(),
            bit_test: ptr::null_mut(),
            bit_set: ptr::null_mut(),
            user: ptr::null_mut(),
            clip_box: FT_BBox { //TODO
                xMin: 0,
                yMin: 0,
                xMax: 1,
                yMax: 1,
            },
        };
        let result = unsafe {
            FT_Outline_Render(self.lib, &mut picture.outline, &mut params)
        };
        if !result.succeeded() {
            println!("WARN: Failed to render an outline!");
        }
        data
    }

    pub fn clean(&mut self, mut picture: PathPicture) {
        let result = unsafe {
            FT_Outline_Done(self.lib, &mut picture.outline)
        };
        if !result.succeeded() {
            println!("WARN: Failed to delete an outline!");
        }
    }
}

impl Drop for PathRenderer {
    fn drop(&mut self) {
        unsafe {
            FT_Done_FreeType(self.lib);
        }
    }
}
