/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Rendering logic related to the vertex shaders and their states, uncluding
//!  - Vertex Array Objects
//!  - vertex layout descriptors
//!  - textures bound at vertex stage

use std::{
    borrow::BorrowMut,
    convert::TryInto,
    marker::PhantomData,
    mem,
    num::NonZeroUsize,
    ptr,
};
use api::units::*;
use crate::{
    batch::{InstanceBufferIndex, InstanceList, InstanceRange},
    device::{
        Device, Texture, TextureFilter, TextureUploader, UploadPBOPool,
        VertexDescriptor, VertexUsageHint, VAO, VBOId,
    },
    frame_builder::Frame,
    gpu_types as gt,
    internal_types::Swizzle,
    render_target::{GradientJob, LineDecorationJob},
    render_task::RenderTaskData,
};
use super::VERTICES_PER_INSTANCE;

pub const VERTEX_TEXTURE_EXTRA_ROWS: i32 = 10;

pub const MAX_VERTEX_TEXTURE_WIDTH: usize = webrender_build::MAX_VERTEX_TEXTURE_WIDTH;

pub mod desc {
    use crate::device::{VertexAttribute, VertexAttributeKind, VertexDescriptor};

    pub const PRIM_INSTANCES: VertexDescriptor = VertexDescriptor {
        vertex_attributes: &[VertexAttribute {
            name: "aPosition",
            count: 2,
            kind: VertexAttributeKind::U8Norm,
        }],
        instance_attributes: &[VertexAttribute {
            name: "aData",
            count: 4,
            kind: VertexAttributeKind::I32,
        }],
    };

    pub const BLUR: VertexDescriptor = VertexDescriptor {
        vertex_attributes: &[VertexAttribute {
            name: "aPosition",
            count: 2,
            kind: VertexAttributeKind::U8Norm,
        }],
        instance_attributes: &[
            VertexAttribute {
                name: "aBlurRenderTaskAddress",
                count: 1,
                kind: VertexAttributeKind::U16,
            },
            VertexAttribute {
                name: "aBlurSourceTaskAddress",
                count: 1,
                kind: VertexAttributeKind::U16,
            },
            VertexAttribute {
                name: "aBlurDirection",
                count: 1,
                kind: VertexAttributeKind::I32,
            },
        ],
    };

    pub const LINE: VertexDescriptor = VertexDescriptor {
        vertex_attributes: &[VertexAttribute {
            name: "aPosition",
            count: 2,
            kind: VertexAttributeKind::U8Norm,
        }],
        instance_attributes: &[
            VertexAttribute {
                name: "aTaskRect",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aLocalSize",
                count: 2,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aWavyLineThickness",
                count: 1,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aStyle",
                count: 1,
                kind: VertexAttributeKind::I32,
            },
            VertexAttribute {
                name: "aAxisSelect",
                count: 1,
                kind: VertexAttributeKind::F32,
            },
        ],
    };

    pub const GRADIENT: VertexDescriptor = VertexDescriptor {
        vertex_attributes: &[VertexAttribute {
            name: "aPosition",
            count: 2,
            kind: VertexAttributeKind::U8Norm,
        }],
        instance_attributes: &[
            VertexAttribute {
                name: "aTaskRect",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aStops",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            // TODO(gw): We should probably pack these as u32 colors instead
            //           of passing as full float vec4 here. It won't make much
            //           difference in real world, since these are only invoked
            //           rarely, when creating the cache.
            VertexAttribute {
                name: "aColor0",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aColor1",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aColor2",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aColor3",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aAxisSelect",
                count: 1,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aStartStop",
                count: 2,
                kind: VertexAttributeKind::F32,
            },
        ],
    };

    pub const BORDER: VertexDescriptor = VertexDescriptor {
        vertex_attributes: &[VertexAttribute {
            name: "aPosition",
            count: 2,
            kind: VertexAttributeKind::U8Norm,
        }],
        instance_attributes: &[
            VertexAttribute {
                name: "aTaskOrigin",
                count: 2,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aRect",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aColor0",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aColor1",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aFlags",
                count: 1,
                kind: VertexAttributeKind::I32,
            },
            VertexAttribute {
                name: "aWidths",
                count: 2,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aRadii",
                count: 2,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aClipParams1",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aClipParams2",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
        ],
    };

    pub const SCALE: VertexDescriptor = VertexDescriptor {
        vertex_attributes: &[VertexAttribute {
            name: "aPosition",
            count: 2,
            kind: VertexAttributeKind::U8Norm,
        }],
        instance_attributes: &[
            VertexAttribute {
                name: "aScaleTargetRect",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aScaleSourceRect",
                count: 4,
                kind: VertexAttributeKind::I32,
            },
            VertexAttribute {
                name: "aScaleSourceLayer",
                count: 1,
                kind: VertexAttributeKind::I32,
            },
        ],
    };

    pub const CLIP_RECT: VertexDescriptor = VertexDescriptor {
        vertex_attributes: &[VertexAttribute {
            name: "aPosition",
            count: 2,
            kind: VertexAttributeKind::U8Norm,
        }],
        instance_attributes: &[
            // common clip attributes
            VertexAttribute {
                name: "aClipDeviceArea",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aClipOrigins",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aDevicePixelScale",
                count: 1,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aTransformIds",
                count: 2,
                kind: VertexAttributeKind::I32,
            },
            // specific clip attributes
            VertexAttribute {
                name: "aClipLocalPos",
                count: 2,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aClipLocalRect",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aClipMode",
                count: 1,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aClipRect_TL",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aClipRadii_TL",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aClipRect_TR",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aClipRadii_TR",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aClipRect_BL",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aClipRadii_BL",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aClipRect_BR",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aClipRadii_BR",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
        ],
    };

    pub const CLIP_BOX_SHADOW: VertexDescriptor = VertexDescriptor {
        vertex_attributes: &[VertexAttribute {
            name: "aPosition",
            count: 2,
            kind: VertexAttributeKind::U8Norm,
        }],
        instance_attributes: &[
            // common clip attributes
            VertexAttribute {
                name: "aClipDeviceArea",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aClipOrigins",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aDevicePixelScale",
                count: 1,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aTransformIds",
                count: 2,
                kind: VertexAttributeKind::I32,
            },
            // specific clip attributes
            VertexAttribute {
                name: "aClipDataResourceAddress",
                count: 2,
                kind: VertexAttributeKind::U16,
            },
            VertexAttribute {
                name: "aClipSrcRectSize",
                count: 2,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aClipMode",
                count: 1,
                kind: VertexAttributeKind::I32,
            },
            VertexAttribute {
                name: "aStretchMode",
                count: 2,
                kind: VertexAttributeKind::I32,
            },
            VertexAttribute {
                name: "aClipDestRect",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
        ],
    };

    pub const CLIP_IMAGE: VertexDescriptor = VertexDescriptor {
        vertex_attributes: &[VertexAttribute {
            name: "aPosition",
            count: 2,
            kind: VertexAttributeKind::U8Norm,
        }],
        instance_attributes: &[
            // common clip attributes
            VertexAttribute {
                name: "aClipDeviceArea",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aClipOrigins",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aDevicePixelScale",
                count: 1,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aTransformIds",
                count: 2,
                kind: VertexAttributeKind::I32,
            },
            // specific clip attributes
            VertexAttribute {
                name: "aClipTileRect",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aClipDataResourceAddress",
                count: 2,
                kind: VertexAttributeKind::U16,
            },
            VertexAttribute {
                name: "aClipLocalRect",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
        ],
    };

    pub const GPU_CACHE_UPDATE: VertexDescriptor = VertexDescriptor {
        vertex_attributes: &[
            VertexAttribute {
                name: "aPosition",
                count: 2,
                kind: VertexAttributeKind::U16Norm,
            },
            VertexAttribute {
                name: "aValue",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
        ],
        instance_attributes: &[],
    };

    pub const RESOLVE: VertexDescriptor = VertexDescriptor {
        vertex_attributes: &[VertexAttribute {
            name: "aPosition",
            count: 2,
            kind: VertexAttributeKind::U8Norm,
        }],
        instance_attributes: &[VertexAttribute {
            name: "aRect",
            count: 4,
            kind: VertexAttributeKind::F32,
        }],
    };

    pub const SVG_FILTER: VertexDescriptor = VertexDescriptor {
        vertex_attributes: &[VertexAttribute {
            name: "aPosition",
            count: 2,
            kind: VertexAttributeKind::U8Norm,
        }],
        instance_attributes: &[
            VertexAttribute {
                name: "aFilterRenderTaskAddress",
                count: 1,
                kind: VertexAttributeKind::U16,
            },
            VertexAttribute {
                name: "aFilterInput1TaskAddress",
                count: 1,
                kind: VertexAttributeKind::U16,
            },
            VertexAttribute {
                name: "aFilterInput2TaskAddress",
                count: 1,
                kind: VertexAttributeKind::U16,
            },
            VertexAttribute {
                name: "aFilterKind",
                count: 1,
                kind: VertexAttributeKind::U16,
            },
            VertexAttribute {
                name: "aFilterInputCount",
                count: 1,
                kind: VertexAttributeKind::U16,
            },
            VertexAttribute {
                name: "aFilterGenericInt",
                count: 1,
                kind: VertexAttributeKind::U16,
            },
            VertexAttribute {
                name: "aFilterExtraDataAddress",
                count: 2,
                kind: VertexAttributeKind::U16,
            },
        ],
    };

    pub const VECTOR_STENCIL: VertexDescriptor = VertexDescriptor {
        vertex_attributes: &[VertexAttribute {
            name: "aPosition",
            count: 2,
            kind: VertexAttributeKind::U8Norm,
        }],
        instance_attributes: &[
            VertexAttribute {
                name: "aFromPosition",
                count: 2,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aCtrlPosition",
                count: 2,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aToPosition",
                count: 2,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aFromNormal",
                count: 2,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aCtrlNormal",
                count: 2,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aToNormal",
                count: 2,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aPathID",
                count: 1,
                kind: VertexAttributeKind::U16,
            },
            VertexAttribute {
                name: "aPad",
                count: 1,
                kind: VertexAttributeKind::U16,
            },
        ],
    };

    pub const VECTOR_COVER: VertexDescriptor = VertexDescriptor {
        vertex_attributes: &[VertexAttribute {
            name: "aPosition",
            count: 2,
            kind: VertexAttributeKind::U8Norm,
        }],
        instance_attributes: &[
            VertexAttribute {
                name: "aTargetRect",
                count: 4,
                kind: VertexAttributeKind::I32,
            },
            VertexAttribute {
                name: "aStencilOrigin",
                count: 2,
                kind: VertexAttributeKind::I32,
            },
            VertexAttribute {
                name: "aSubpixel",
                count: 1,
                kind: VertexAttributeKind::U16,
            },
            VertexAttribute {
                name: "aPad",
                count: 1,
                kind: VertexAttributeKind::U16,
            },
        ],
    };

    pub const COMPOSITE: VertexDescriptor = VertexDescriptor {
        vertex_attributes: &[VertexAttribute {
            name: "aPosition",
            count: 2,
            kind: VertexAttributeKind::U8Norm,
        }],
        instance_attributes: &[
            VertexAttribute {
                name: "aDeviceRect",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aDeviceClipRect",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aColor",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aParams",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aUvRect0",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aUvRect1",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aUvRect2",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aTextureLayers",
                count: 3,
                kind: VertexAttributeKind::F32,
            },
        ],
    };

    pub const CLEAR: VertexDescriptor = VertexDescriptor {
        vertex_attributes: &[VertexAttribute {
            name: "aPosition",
            count: 2,
            kind: VertexAttributeKind::U8Norm,
        }],
        instance_attributes: &[
            VertexAttribute {
                name: "aRect",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
            VertexAttribute {
                name: "aColor",
                count: 4,
                kind: VertexAttributeKind::F32,
            },
        ],
    };
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum VertexArrayKind {
    Primitive,
    Blur,
    ClipImage,
    ClipRect,
    ClipBoxShadow,
    VectorStencil,
    VectorCover,
    Border,
    Scale,
    LineDecoration,
    Gradient,
    Resolve,
    SvgFilter,
    Composite,
    Clear,
}

pub struct VertexDataTexture<T> {
    texture: Option<Texture>,
    format: api::ImageFormat,
    _marker: PhantomData<T>,
}

impl<T> VertexDataTexture<T> {
    pub fn new(format: api::ImageFormat) -> Self {
        Self {
            texture: None,
            format,
            _marker: PhantomData,
        }
    }

    /// Returns a borrow of the GPU texture. Panics if it hasn't been initialized.
    pub fn texture(&self) -> &Texture {
        self.texture.as_ref().unwrap()
    }

    /// Returns an estimate of the GPU memory consumed by this VertexDataTexture.
    pub fn size_in_bytes(&self) -> usize {
        self.texture.as_ref().map_or(0, |t| t.size_in_bytes())
    }

    pub fn update<'a>(
        &'a mut self,
        device: &mut Device,
        texture_uploader: &mut TextureUploader<'a>,
        data: &mut Vec<T>,
    ) {
        debug_assert!(mem::size_of::<T>() % 16 == 0);
        let texels_per_item = mem::size_of::<T>() / 16;
        let items_per_row = MAX_VERTEX_TEXTURE_WIDTH / texels_per_item;
        debug_assert_ne!(items_per_row, 0);

        // Ensure we always end up with a texture when leaving this method.
        let mut len = data.len();
        if len == 0 {
            if self.texture.is_some() {
                return;
            }
            data.reserve(items_per_row);
            len = items_per_row;
        } else {
            // Extend the data array to have enough capacity to upload at least
            // a multiple of the row size.  This ensures memory safety when the
            // array is passed to OpenGL to upload to the GPU.
            let extra = len % items_per_row;
            if extra != 0 {
                let padding = items_per_row - extra;
                data.reserve(padding);
                len += padding;
            }
        }

        let needed_height = (len / items_per_row) as i32;
        let existing_height = self
            .texture
            .as_ref()
            .map_or(0, |t| t.get_dimensions().height);

        // Create a new texture if needed.
        //
        // These textures are generally very small, which is why we don't bother
        // with incremental updates and just re-upload every frame. For most pages
        // they're one row each, and on stress tests like css-francine they end up
        // in the 6-14 range. So we size the texture tightly to what we need (usually
        // 1), and shrink it if the waste would be more than `VERTEX_TEXTURE_EXTRA_ROWS`
        // rows. This helps with memory overhead, especially because there are several
        // instances of these textures per Renderer.
        if needed_height > existing_height
            || needed_height + VERTEX_TEXTURE_EXTRA_ROWS < existing_height
        {
            // Drop the existing texture, if any.
            if let Some(t) = self.texture.take() {
                device.delete_texture(t);
            }

            let texture = device.create_texture(
                api::ImageBufferKind::Texture2D,
                self.format,
                MAX_VERTEX_TEXTURE_WIDTH as i32,
                // Ensure height is at least two to work around
                // https://bugs.chromium.org/p/angleproject/issues/detail?id=3039
                needed_height.max(2),
                TextureFilter::Nearest,
                None,
                1,
            );
            self.texture = Some(texture);
        }

        // Note: the actual width can be larger than the logical one, with a few texels
        // of each row unused at the tail. This is needed because there is still hardware
        // (like Intel iGPUs) that prefers power-of-two sizes of textures ([1]).
        //
        // [1] https://software.intel.com/en-us/articles/opengl-performance-tips-power-of-two-textures-have-better-performance
        let logical_width = if needed_height == 1 {
            data.len() * texels_per_item
        } else {
            MAX_VERTEX_TEXTURE_WIDTH - (MAX_VERTEX_TEXTURE_WIDTH % texels_per_item)
        };

        let rect = DeviceIntRect::new(
            DeviceIntPoint::zero(),
            DeviceIntSize::new(logical_width as i32, needed_height),
        );

        debug_assert!(len <= data.capacity(), "CPU copy will read out of bounds");
        texture_uploader.upload(
            device,
            self.texture(),
            rect,
            0,
            None,
            None,
            data.as_ptr(),
            len,
        );
    }

    pub fn deinit(mut self, device: &mut Device) {
        if let Some(t) = self.texture.take() {
            device.delete_texture(t);
        }
    }
}

pub struct VertexDataTextures {
    prim_header_f_texture: VertexDataTexture<gt::PrimitiveHeaderF>,
    prim_header_i_texture: VertexDataTexture<gt::PrimitiveHeaderI>,
    transforms_texture: VertexDataTexture<gt::TransformData>,
    render_task_texture: VertexDataTexture<RenderTaskData>,
}

impl VertexDataTextures {
    pub fn new() -> Self {
        VertexDataTextures {
            prim_header_f_texture: VertexDataTexture::new(api::ImageFormat::RGBAF32),
            prim_header_i_texture: VertexDataTexture::new(api::ImageFormat::RGBAI32),
            transforms_texture: VertexDataTexture::new(api::ImageFormat::RGBAF32),
            render_task_texture: VertexDataTexture::new(api::ImageFormat::RGBAF32),
        }
    }

    pub fn update(&mut self, device: &mut Device, pbo_pool: &mut UploadPBOPool, frame: &mut Frame) {
        let mut texture_uploader = device.upload_texture(pbo_pool);
        self.prim_header_f_texture.update(
            device,
            &mut texture_uploader,
            &mut frame.prim_headers.headers_float,
        );
        self.prim_header_i_texture.update(
            device,
            &mut texture_uploader,
            &mut frame.prim_headers.headers_int,
        );
        self.transforms_texture
            .update(device, &mut texture_uploader, &mut frame.transform_palette);
        self.render_task_texture.update(
            device,
            &mut texture_uploader,
            &mut frame.render_tasks.task_data,
        );

        // Flush and drop the texture uploader now, so that
        // we can borrow the textures to bind them.
        texture_uploader.flush(device);

        device.bind_texture(
            super::TextureSampler::PrimitiveHeadersF,
            &self.prim_header_f_texture.texture(),
            Swizzle::default(),
        );
        device.bind_texture(
            super::TextureSampler::PrimitiveHeadersI,
            &self.prim_header_i_texture.texture(),
            Swizzle::default(),
        );
        device.bind_texture(
            super::TextureSampler::TransformPalette,
            &self.transforms_texture.texture(),
            Swizzle::default(),
        );
        device.bind_texture(
            super::TextureSampler::RenderTasks,
            &self.render_task_texture.texture(),
            Swizzle::default(),
        );
    }

    pub fn size_in_bytes(&self) -> usize {
        self.prim_header_f_texture.size_in_bytes()
            + self.prim_header_i_texture.size_in_bytes()
            + self.transforms_texture.size_in_bytes()
            + self.render_task_texture.size_in_bytes()
    }

    pub fn deinit(self, device: &mut Device) {
        self.transforms_texture.deinit(device);
        self.prim_header_f_texture.deinit(device);
        self.prim_header_i_texture.deinit(device);
        self.render_task_texture.deinit(device);
    }
}

pub struct VertexContext<T> {
    vao: VAO,
    instance_pool: InstancePool<T>,
    current_instance_buffer: VBOId,
    descriptor: &'static VertexDescriptor,
}

pub struct VertexContextRef<'a> {
    vao: &'a VAO,
    instance_buffers: &'a [VBOId],
    current_instance_buffer: &'a mut VBOId,
    descriptor: &'static VertexDescriptor,
    duplicate_per_vertex: bool,
    usage_hint: VertexUsageHint,
    epoch: usize,
}

impl VertexContextRef<'_> {
    fn bind_impl(&mut self, buffer: VBOId, device: &mut Device) {
        device.bind_vao(self.vao);
        if *self.current_instance_buffer != buffer {
            *self.current_instance_buffer = buffer;
            let divisor = if self.duplicate_per_vertex { 0 } else { 1 };
            device.switch_instance_buffer(buffer, self.descriptor, divisor);
        } else {
            buffer.bind(device.gl());
        }
    }

    pub fn bind(&mut self, index: InstanceBufferIndex, device: &mut Device) -> usize {
        let buffer = self.instance_buffers[index as usize];
        self.bind_impl(buffer, device);
        self.epoch
    }

    pub fn bind_general(&mut self, device: &mut Device) {
        self.bind_impl(self.vao.instance_vbo_id, device);
    }

    pub fn upload_instance_data<T: Copy>(&mut self, instances: &[T], device: &mut Device) {
        debug_assert_eq!(self.vao.instance_stride as usize, mem::size_of::<T>());
        assert_eq!(*self.current_instance_buffer, self.vao.instance_vbo_id);

        if self.duplicate_per_vertex {
            println!("Mapping {:?} for {} instances", self.vao.instance_vbo_id, instances.len() * VERTICES_PER_INSTANCE);
            let ptr = device.initialize_mapped_vertex_buffer(
                self.vao.instance_vbo_id,
                instances.len() * VERTICES_PER_INSTANCE * mem::size_of::<T>(),
                self.usage_hint,
            );
            assert!(!ptr.is_null());
            unsafe {
                InstancePool::fill(ptr as *mut T, instances, self.duplicate_per_vertex)
            };
            println!("Unmapping {:?}", self.vao.instance_vbo_id);
            device.unmap_vertex_buffer();
        } else {
            device.update_vbo_data(self.vao.instance_vbo_id, instances, self.usage_hint);
        }
    }
}

impl<T: Copy> VertexContext<T> {
    fn new(
        device: &mut Device,
        descriptor: &'static VertexDescriptor,
        base_vao: &VAO,
        usage_hint: VertexUsageHint,
    ) -> Self {
        let vao = device.create_vao_with_new_instances(descriptor, base_vao);
        let instanced = base_vao.instance_divisor != 0;
        VertexContext {
            current_instance_buffer: vao.instance_vbo_id.clone(),
            vao,
            instance_pool: InstancePool::new(0x100, usage_hint, !instanced),
            descriptor,
        }
    }

    fn deinit(self, device: &mut Device) {
        self.instance_pool.deinit(device);
        device.delete_vao(self.vao);
    }

    fn to_ref(&mut self) -> VertexContextRef {
        assert!(self.instance_pool.mapped_chunks.is_empty());
        VertexContextRef {
            vao: &self.vao,
            instance_buffers: &self.instance_pool.used_chunks,
            current_instance_buffer: &mut self.current_instance_buffer,
            descriptor: self.descriptor,
            duplicate_per_vertex: self.instance_pool.duplicate_per_vertex,
            usage_hint: self.instance_pool.usage_hint,
            epoch: self.instance_pool.epoch,
        }
    }

    pub fn bake_instances<I>(&mut self, device: &mut Device, list_iter: I)
    where
        I: IntoIterator,
        I::Item: BorrowMut<InstanceList<T>>
    {
        for mut list in list_iter {
            let mut list = list.borrow_mut();
            if !list.data.is_empty() {
                let range = self.instance_pool.add(&list.data, device);
                list.range = Some(range);
                list.data.clear();
            }
        }
    }
}

pub struct VertexContextHub {
    pub prim: VertexContext<gt::PrimitiveInstanceData>,
    pub blur: VertexContext<gt::BlurInstance>,
    pub clip_rect: VertexContext<gt::ClipMaskInstanceRect>,
    pub clip_box_shadow: VertexContext<gt::ClipMaskInstanceBoxShadow>,
    pub clip_image: VertexContext<gt::ClipMaskInstanceImage>,
    pub border: VertexContext<gt::BorderInstance>,
    pub line: VertexContext<LineDecorationJob>,
    pub scale: VertexContext<gt::ScalingInstance>,
    pub gradient: VertexContext<GradientJob>,
    resolve: VertexContext<()>, // not used
    pub svg_filter: VertexContext<gt::SvgFilterInstance>,
    pub composite: VertexContext<gt::CompositeInstance>,
    pub clear: VertexContext<gt::ClearInstance>,
}

impl VertexContextHub {
    pub fn new(
        device: &mut Device,
        indexed_quads: Option<NonZeroUsize>,
        usage_hint: VertexUsageHint,
    ) -> Self {
        const QUAD_INDICES: [u16; 6] = [0, 1, 2, 2, 1, 3];
        const QUAD_VERTICES: [[u8; 2]; VERTICES_PER_INSTANCE] = [[0, 0], [0xFF, 0], [0, 0xFF], [0xFF, 0xFF]];

        let instance_divisor = if indexed_quads.is_some() { 0 } else { 1 };
        let prim_vao = device.create_vao(&desc::PRIM_INSTANCES, instance_divisor);

        device.bind_vao(&prim_vao);
        match indexed_quads {
            Some(count) => {
                assert!(count.get() < u16::MAX as usize);
                let quad_indices = (0 .. count.get() as u16)
                    .flat_map(|instance| QUAD_INDICES.iter().map(move |&index| instance * 4 + index))
                    .collect::<Vec<_>>();
                device.update_vao_indices(&prim_vao, &quad_indices, VertexUsageHint::Static);
                let quad_vertices = (0 .. count.get() as u16)
                    .flat_map(|_| QUAD_VERTICES.iter().cloned())
                    .collect::<Vec<_>>();
                device.update_vao_main_vertices(&prim_vao, &quad_vertices, VertexUsageHint::Static);
            }
            None => {
                device.update_vao_indices(&prim_vao, &QUAD_INDICES, VertexUsageHint::Static);
                device.update_vao_main_vertices(&prim_vao, &QUAD_VERTICES, VertexUsageHint::Static);
            }
        }

        VertexContextHub {
            blur: VertexContext::new(device, &desc::BLUR, &prim_vao, usage_hint),
            clip_rect: VertexContext::new(device, &desc::CLIP_RECT, &prim_vao, usage_hint),
            clip_box_shadow: VertexContext::new(device, &desc::CLIP_BOX_SHADOW, &prim_vao, usage_hint),
            clip_image: VertexContext::new(device, &desc::CLIP_IMAGE, &prim_vao, usage_hint),
            border: VertexContext::new(device, &desc::BORDER, &prim_vao, usage_hint),
            scale: VertexContext::new(device, &desc::SCALE, &prim_vao, usage_hint),
            line: VertexContext::new(device, &desc::LINE, &prim_vao, usage_hint),
            gradient: VertexContext::new(device, &desc::GRADIENT, &prim_vao, usage_hint),
            resolve: VertexContext::new(device, &desc::RESOLVE, &prim_vao, usage_hint),
            svg_filter: VertexContext::new(device, &desc::SVG_FILTER, &prim_vao, usage_hint),
            composite: VertexContext::new(device, &desc::COMPOSITE, &prim_vao, usage_hint),
            clear: VertexContext::new(device, &desc::CLEAR, &prim_vao, usage_hint),
            prim: VertexContext {
                current_instance_buffer: prim_vao.instance_vbo_id.clone(),
                vao: prim_vao,
                instance_pool: {
                    let chunk_size = indexed_quads.map_or(0, |count| count.get() / 2);
                    InstancePool::new(chunk_size, usage_hint, indexed_quads.is_some())
                },
                descriptor: &desc::PRIM_INSTANCES,
            },
        }
    }

    pub fn deinit(self, device: &mut Device) {
        self.prim.deinit(device);
        self.resolve.deinit(device);
        self.clip_rect.deinit(device);
        self.clip_box_shadow.deinit(device);
        self.clip_image.deinit(device);
        self.gradient.deinit(device);
        self.blur.deinit(device);
        self.line.deinit(device);
        self.border.deinit(device);
        self.scale.deinit(device);
        self.svg_filter.deinit(device);
        self.composite.deinit(device);
        self.clear.deinit(device);
    }

    pub fn get(&mut self, kind: VertexArrayKind) -> VertexContextRef {
        match kind {
            VertexArrayKind::Primitive => self.prim.to_ref(),
            VertexArrayKind::ClipImage => self.clip_image.to_ref(),
            VertexArrayKind::ClipRect => self.clip_rect.to_ref(),
            VertexArrayKind::ClipBoxShadow => self.clip_box_shadow.to_ref(),
            VertexArrayKind::Blur => self.blur.to_ref(),
            VertexArrayKind::VectorStencil | VertexArrayKind::VectorCover => unreachable!(),
            VertexArrayKind::Border => self.border.to_ref(),
            VertexArrayKind::Scale => self.scale.to_ref(),
            VertexArrayKind::LineDecoration => self.line.to_ref(),
            VertexArrayKind::Gradient => self.gradient.to_ref(),
            VertexArrayKind::Resolve => self.resolve.to_ref(),
            VertexArrayKind::SvgFilter => self.svg_filter.to_ref(),
            VertexArrayKind::Composite => self.composite.to_ref(),
            VertexArrayKind::Clear => self.clear.to_ref(),
        }
    }

    pub fn reset_instance_pools(&mut self) {
        self.prim.instance_pool.reset();
        self.resolve.instance_pool.reset();
        self.clip_rect.instance_pool.reset();
        self.clip_box_shadow.instance_pool.reset();
        self.clip_image.instance_pool.reset();
        self.gradient.instance_pool.reset();
        self.blur.instance_pool.reset();
        self.line.instance_pool.reset();
        self.border.instance_pool.reset();
        self.scale.instance_pool.reset();
        self.svg_filter.instance_pool.reset();
        self.composite.instance_pool.reset();
        self.clear.instance_pool.reset();
    }

    pub fn finish_populating_instances(&mut self, device: &mut Device) {
        self.prim.instance_pool.finish(device);
        self.resolve.instance_pool.finish(device);
        self.clip_rect.instance_pool.finish(device);
        self.clip_box_shadow.instance_pool.finish(device);
        self.clip_image.instance_pool.finish(device);
        self.gradient.instance_pool.finish(device);
        self.blur.instance_pool.finish(device);
        self.line.instance_pool.finish(device);
        self.border.instance_pool.finish(device);
        self.scale.instance_pool.finish(device);
        self.svg_filter.instance_pool.finish(device);
        self.composite.instance_pool.finish(device);
        self.clear.instance_pool.finish(device);
    }
}

struct MappedChunk<T> {
    ptr: *mut T,
    buffer_index: InstanceBufferIndex,
    size: usize,
}

pub struct InstancePool<T> {
    chunk_size: usize,
    mapped_chunks: Vec<MappedChunk<T>>,
    used_chunks: Vec<VBOId>,
    ready_chunks: Vec<VBOId>,
    usage_hint: VertexUsageHint,
    duplicate_per_vertex: bool,
    epoch: usize,
}

impl<T: Copy> InstancePool<T> {
    pub fn new(chunk_size: usize, usage_hint: VertexUsageHint, duplicate_per_vertex: bool) -> Self {
        InstancePool {
            chunk_size,
            mapped_chunks: Vec::new(),
            used_chunks: Vec::new(),
            ready_chunks: Vec::new(),
            usage_hint,
            duplicate_per_vertex,
            epoch: 0,
        }
    }

    unsafe fn fill(ptr: *mut T, data: &[T], duplicate_per_vertex: bool) {
        debug_assert_eq!(ptr.align_offset(mem::align_of::<T>()), 0);
        if duplicate_per_vertex {
            for (i, v) in data.iter().enumerate() {
                //Note: this respects VERTICES_PER_INSTANCE
                *ptr.add((i<<2) + 0) = *v;
                *ptr.add((i<<2) + 1) = *v;
                *ptr.add((i<<2) + 2) = *v;
                *ptr.add((i<<2) + 3) = *v;
            }
        } else {
            ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len())
        }
    }

    fn add(&mut self, instances: &[T], device: &mut Device) -> InstanceRange {
        let total_size = self.chunk_size;
        let repeat_count = if self.duplicate_per_vertex { VERTICES_PER_INSTANCE } else { 1 };
        let extra_size = instances.len() * repeat_count;
        if let Some(ref mut mc) = self.mapped_chunks.iter_mut().find(|mc| mc.size + extra_size <= total_size) {
            unsafe {
                Self::fill(mc.ptr.add(mc.size), instances, self.duplicate_per_vertex);
            }
            mc.size += extra_size;
            let full_instance_count = mc.size / repeat_count;
            return InstanceRange {
                buffer_index: mc.buffer_index,
                sub_range: full_instance_count - instances.len() .. full_instance_count,
                #[cfg(debug_assertions)]
                epoch: self.epoch,
            }
        }

        let buffer = match self.ready_chunks.pop() {
            Some(buffer) => buffer,
            None => device.create_vbo_raw(),
        };

        let buffer_index: InstanceBufferIndex = self.used_chunks.len().try_into().unwrap();
        self.used_chunks.push(buffer);
        if self.chunk_size <= extra_size && !self.duplicate_per_vertex {
            device.update_vbo_data(buffer, instances, self.usage_hint);
        } else {
            println!("Mapping {:?} for {} instances", buffer, self.chunk_size.max(extra_size));
            let ptr = device.initialize_mapped_vertex_buffer(
                buffer,
                self.chunk_size.max(extra_size) * mem::size_of::<T>(),
                self.usage_hint,
            );
            assert!(!ptr.is_null());
            unsafe {
                Self::fill(ptr as *mut T, instances, self.duplicate_per_vertex);
            }
            if self.chunk_size <= extra_size {
                println!("Unmapping {:?}", buffer);
                device.unmap_vertex_buffer();
            } else {
                self.mapped_chunks.push(MappedChunk {
                    ptr: ptr as *mut T,
                    buffer_index,
                    size: extra_size,
                });
            }
        }
        InstanceRange {
            buffer_index,
            sub_range: 0 .. instances.len(),
            #[cfg(debug_assertions)]
            epoch: self.epoch,
        }
    }

    pub fn finish(&mut self, device: &mut Device) {
        for mc in self.mapped_chunks.drain(..) {
            let buffer = self.used_chunks[mc.buffer_index as usize];
            buffer.bind(device.gl());
            println!("Unmapping {:?}", buffer);
            device.unmap_vertex_buffer();
        }
    }

    pub fn reset(&mut self) {
        assert!(self.mapped_chunks.is_empty());
        self.ready_chunks.extend(self.used_chunks.drain(..));
        self.epoch += 1;
    }

    fn deinit(mut self, device: &mut Device) {
        self.finish(device);
        self.reset();
        for buffer in self.ready_chunks.drain(..) {
            device.delete_vbo_raw(buffer);
        }
    }
}
