/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::{mem, ptr};
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
        let max_points = commands.len() as u32;
        let max_countours = commands.len() as u32;
        let mut outline: FT_Outline = unsafe { mem::zeroed() };
        let result = unsafe {
            FT_New_Outline(self.lib, max_points, max_countours, &mut outline)
        };
        if !result.succeeded() {
            panic!("Unable to create FT_Outline {}", result);
        }
        PathPicture {
            outline: outline,
        }
    }

    pub fn draw(&mut self, _picture: &PathPicture, _width: u32, _height: u32) -> () {
        unimplemented!()
    }

    pub fn clean(&mut self, mut picture: PathPicture) {
        let result = unsafe {
            FT_Done_Outline(self.lib, &mut picture.outline)
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
