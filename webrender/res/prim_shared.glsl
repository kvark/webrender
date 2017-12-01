/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#include rect

#define EXTEND_MODE_CLAMP  0
#define EXTEND_MODE_REPEAT 1

#define LINE_STYLE_SOLID        0
#define LINE_STYLE_DOTTED       1
#define LINE_STYLE_DASHED       2
#define LINE_STYLE_WAVY         3

#define SUBPX_DIR_NONE        0
#define SUBPX_DIR_HORIZONTAL  1
#define SUBPX_DIR_VERTICAL    2

uniform sampler2DArray sCacheA8;
uniform sampler2DArray sCacheRGBA8;

// An A8 target for standalone tasks that is available to all passes.
uniform sampler2DArray sSharedCacheA8;

#ifdef WR_DX11
SamplerState sCacheA8_;
SamplerState sCacheRGBA8_;
SamplerState sSharedCacheA8_;
#endif //WR_DX11

#ifdef WR_FEATURE_TEXTURE_2D
uniform sampler2D sColor1;
uniform sampler2D sColor2;
#elif defined WR_FEATURE_TEXTURE_RECT
uniform sampler2DRect sColor1;
uniform sampler2DRect sColor2;
#elif defined WR_FEATURE_TEXTURE_EXTERNAL
uniform samplerExternalOES sColor1;
uniform samplerExternalOES sColor2;
#else
uniform sampler2DArray sColor1;
uniform sampler2DArray sColor2;
#endif

#ifdef WR_DX11
SamplerState sColor1_;
SamplerState sColor2_;
#endif //WR_DX11

vec2 clamp_rect(vec2 point, RectWithSize rect) {
    return clamp(point, rect.p0, rect.p0 + rect.size);
}

float distance_to_line(vec2 p0, vec2 perp_dir, vec2 p) {
    vec2 dir_to_p0 = p0 - p;
    return dot(normalize(perp_dir), dir_to_p0);
}

// TODO: convert back to RectWithEndPoint if driver issues are resolved, if ever.
#ifndef WR_DX11
flat varying vec4 vClipMaskUvBounds;
varying vec3 vClipMaskUv;
#ifdef WR_FEATURE_TRANSFORM
    flat varying vec4 vLocalBounds;
#endif
#endif

// TODO(gw): This is here temporarily while we have
//           both GPU store and cache. When the GPU
//           store code is removed, we can change the
//           PrimitiveInstance instance structure to
//           use 2x unsigned shorts as vertex attributes
//           instead of an int, and encode the UV directly
//           in the vertices.
ivec2 get_resource_cache_uv(int address) {
    return ivec2(address % WR_MAX_VERTEX_TEXTURE_WIDTH,
                 address / WR_MAX_VERTEX_TEXTURE_WIDTH);
}

uniform HIGHP_SAMPLER_FLOAT sampler2D sResourceCache;

struct ResourceCacheData2 {
    vec4 data0;
    vec4 data1;
};

ResourceCacheData2  fetch_from_resource_cache_2_direct(ivec2 address) {
    ResourceCacheData2 result;
    result.data0 = TEXEL_FETCH(sResourceCache, address, 0, ivec2(0, 0));
    result.data1 = TEXEL_FETCH(sResourceCache, address, 0, ivec2(1, 0));
    return result;
}

ResourceCacheData2  fetch_from_resource_cache_2(int address) {
    ivec2 uv = get_resource_cache_uv(address);
    ResourceCacheData2 result;
    result.data0 = TEXEL_FETCH(sResourceCache, uv, 0, ivec2(0, 0));
    result.data1 = TEXEL_FETCH(sResourceCache, uv, 0, ivec2(1, 0));
    return result;
}

#ifdef WR_VERTEX_SHADER

#define VECS_PER_LAYER              9
#define VECS_PER_RENDER_TASK        3
#define VECS_PER_PRIM_HEADER        2
#define VECS_PER_TEXT_RUN           2
#define VECS_PER_GRADIENT           3
#define VECS_PER_GRADIENT_STOP      2

uniform HIGHP_SAMPLER_FLOAT sampler2D sLayers;
uniform HIGHP_SAMPLER_FLOAT sampler2D sRenderTasks;

// Instanced attributes
#if defined(WR_DX11) && !defined(WR_CLIP_SHADER)
struct a2v {
    vec3 pos : aPosition;
    ivec4 data0 : aDataA;
    ivec4 data1 : aDataB;
    uint vertexId: SV_VertexId;
};
#elif !defined(WR_DX11)
in ivec4 aDataA;
in ivec4 aDataB;
#endif //WR_DX11 && !WR_CLIP_SHADER

// get_fetch_uv is a macro to work around a macOS Intel driver parsing bug.
// TODO: convert back to a function once the driver issues are resolved, if ever.
// https://github.com/servo/webrender/pull/623
// https://github.com/servo/servo/issues/13953
#define get_fetch_uv(i, vpi)  ivec2(vpi * (i % (WR_MAX_VERTEX_TEXTURE_WIDTH/vpi)), i / (WR_MAX_VERTEX_TEXTURE_WIDTH/vpi))

struct ResourceCacheData8 {
    vec4 data0;
    vec4 data1;
    vec4 data2;
    vec4 data3;
    vec4 data4;
    vec4 data5;
    vec4 data6;
    vec4 data7;
};

ResourceCacheData8 fetch_from_resource_cache_8(int address) {
    ivec2 uv = get_resource_cache_uv(address);
    ResourceCacheData8 result;
    result.data0 = TEXEL_FETCH(sResourceCache, uv, 0, ivec2(0, 0));
    result.data1 = TEXEL_FETCH(sResourceCache, uv, 0, ivec2(1, 0));
    result.data2 = TEXEL_FETCH(sResourceCache, uv, 0, ivec2(2, 0));
    result.data3 = TEXEL_FETCH(sResourceCache, uv, 0, ivec2(3, 0));
    result.data4 = TEXEL_FETCH(sResourceCache, uv, 0, ivec2(4, 0));
    result.data5 = TEXEL_FETCH(sResourceCache, uv, 0, ivec2(5, 0));
    result.data6 = TEXEL_FETCH(sResourceCache, uv, 0, ivec2(6, 0));
    result.data7 = TEXEL_FETCH(sResourceCache, uv, 0, ivec2(7, 0));
    return result;
}

struct ResourceCacheData3 {
    vec4 data0;
    vec4 data1;
    vec4 data2;
};

ResourceCacheData3 fetch_from_resource_cache_3(int address) {
    ivec2 uv = get_resource_cache_uv(address);
    ResourceCacheData3 result;
    result.data0 = TEXEL_FETCH(sResourceCache, uv, 0, ivec2(0, 0));
    result.data1 = TEXEL_FETCH(sResourceCache, uv, 0, ivec2(1, 0));
    result.data2 = TEXEL_FETCH(sResourceCache, uv, 0, ivec2(2, 0));
    return result;
}

struct ResourceCacheData4 {
    vec4 data0;
    vec4 data1;
    vec4 data2;
    vec4 data3;
};

ResourceCacheData4 fetch_from_resource_cache_4_direct(ivec2 address) {
    ResourceCacheData4 result;
    result.data0 = TEXEL_FETCH(sResourceCache, address, 0, ivec2(0, 0));
    result.data1 = TEXEL_FETCH(sResourceCache, address, 0, ivec2(1, 0));
    result.data2 = TEXEL_FETCH(sResourceCache, address, 0, ivec2(2, 0));
    result.data3 = TEXEL_FETCH(sResourceCache, address, 0, ivec2(3, 0));
    return result;
}

ResourceCacheData4 fetch_from_resource_cache_4(int address) {
    ivec2 uv = get_resource_cache_uv(address);
    ResourceCacheData4 result;
    result.data0 = TEXEL_FETCH(sResourceCache, uv, 0, ivec2(0, 0));
    result.data1 = TEXEL_FETCH(sResourceCache, uv, 0, ivec2(1, 0));
    result.data2 = TEXEL_FETCH(sResourceCache, uv, 0, ivec2(2, 0));
    result.data3 = TEXEL_FETCH(sResourceCache, uv, 0, ivec2(3, 0));
    return result;
}

vec4 fetch_from_resource_cache_1_direct(ivec2 address) {
    return texelFetch(sResourceCache, address, 0);
}

vec4 fetch_from_resource_cache_1(int address) {
    ivec2 uv = get_resource_cache_uv(address);
    return texelFetch(sResourceCache, uv, 0);
}

struct Layer {
    mat4 transform;
    mat4 inv_transform;
    RectWithSize local_clip_rect;
};

Layer fetch_layer(int index) {
    Layer layer;

    // Create a UV base coord for each 8 texels.
    // This is required because trying to use an offset
    // of more than 8 texels doesn't work on some versions
    // of OSX.
    ivec2 uv = get_fetch_uv(index, VECS_PER_LAYER);
    ivec2 uv0 = ivec2(uv.x + 0, uv.y);
    ivec2 uv1 = ivec2(uv.x + 8, uv.y);

    layer.transform[0] = TEXEL_FETCH(sLayers, uv0, 0, ivec2(0, 0));
    layer.transform[1] = TEXEL_FETCH(sLayers, uv0, 0, ivec2(1, 0));
    layer.transform[2] = TEXEL_FETCH(sLayers, uv0, 0, ivec2(2, 0));
    layer.transform[3] = TEXEL_FETCH(sLayers, uv0, 0, ivec2(3, 0));

    layer.inv_transform[0] = TEXEL_FETCH(sLayers, uv0, 0, ivec2(4, 0));
    layer.inv_transform[1] = TEXEL_FETCH(sLayers, uv0, 0, ivec2(5, 0));
    layer.inv_transform[2] = TEXEL_FETCH(sLayers, uv0, 0, ivec2(6, 0));
    layer.inv_transform[3] = TEXEL_FETCH(sLayers, uv0, 0, ivec2(7, 0));

    vec4 clip_rect = TEXEL_FETCH(sLayers, uv1, 0, ivec2(0, 0));
    RectWithSize rect;
    rect.p0 = clip_rect.xy;
    rect.size = clip_rect.zw;
    layer.local_clip_rect = rect;

    return layer;
}

struct RenderTaskData {
    vec4 data0;
    vec4 data1;
    vec4 data2;
};

RenderTaskData fetch_render_task(int index) {
    RenderTaskData task;

    ivec2 uv = get_fetch_uv(index, VECS_PER_RENDER_TASK);

    task.data0 = TEXEL_FETCH(sRenderTasks, uv, 0, ivec2(0, 0));
    task.data1 = TEXEL_FETCH(sRenderTasks, uv, 0, ivec2(1, 0));
    task.data2 = TEXEL_FETCH(sRenderTasks, uv, 0, ivec2(2, 0));

    return task;
}

/*
 The dynamic picture that this brush exists on. Right now, it
 contains minimal information. In the future, it will describe
 the transform mode of primitives on this picture, among other things.
 */
struct PictureTask {
    RectWithSize target_rect;
    float render_target_layer_index;
    vec2 content_origin;
    vec4 color;
};

PictureTask fetch_picture_task(int address) {
    RenderTaskData task_data = fetch_render_task(address);

    RectWithSize rect;
    rect.p0 = task_data.data0.xy;
    rect.size = task_data.data0.zw;
    PictureTask picture_task;
    picture_task.target_rect = rect;
    picture_task.render_target_layer_index = task_data.data1.x;
    picture_task.content_origin = task_data.data1.yz;
    picture_task.color = task_data.data2;
    return picture_task;
}

struct BlurTask {
    RectWithSize target_rect;
    float render_target_layer_index;
    float blur_radius;
    vec4 color;
};

BlurTask fetch_blur_task(int address) {
    RenderTaskData task_data = fetch_render_task(address);

    RectWithSize rect;
    rect.p0 = task_data.data0.xy;
    rect.size = task_data.data0.zw;
    BlurTask blur_task;
    blur_task.target_rect = rect;
    blur_task.render_target_layer_index = task_data.data1.x;
    blur_task.blur_radius = task_data.data1.y;
    blur_task.color = task_data.data2;
    return blur_task;
}

struct AlphaBatchTask {
    vec2 screen_space_origin;
    vec2 render_target_origin;
    vec2 size;
    float render_target_layer_index;
};

AlphaBatchTask fetch_alpha_batch_task(int index) {
    RenderTaskData data = fetch_render_task(index);

    AlphaBatchTask task;
    task.render_target_origin = data.data0.xy;
    task.size = data.data0.zw;
    task.screen_space_origin = data.data1.xy;
    task.render_target_layer_index = data.data1.z;

    return task;
}

struct ClipArea {
    vec4 task_bounds;
    vec4 screen_origin_target_index;
    vec4 inner_rect;
};

ClipArea fetch_clip_area(int index) {
    ClipArea area;

    if (index == 0x7FFFFFFF) { //special sentinel task index
        area.task_bounds = vec4(0.0, 0.0, 0.0, 0.0);
        area.screen_origin_target_index = vec4(0.0, 0.0, 0.0, 0.0);
        area.inner_rect = vec4(0.0, 0.0, 0.0, 0.0);
    } else {
        RenderTaskData task = fetch_render_task(index);
        area.task_bounds = task.data0;
        area.screen_origin_target_index = task.data1;
        area.inner_rect = task.data2;
    }

    return area;
}

struct Gradient {
    vec4 start_end_point;
    vec4 tile_size_repeat;
    vec4 extend_mode;
};

Gradient fetch_gradient(int address) {
    ResourceCacheData3 data = fetch_from_resource_cache_3(address);
    Gradient result;
    result.start_end_point = data.data0;
    result.tile_size_repeat = data.data1;
    result.extend_mode = data.data2;
    return result;
}

struct GradientStop {
    vec4 color;
    vec4 offset;
};

GradientStop fetch_gradient_stop(int address) {
    ResourceCacheData2 data = fetch_from_resource_cache_2(address);
    GradientStop result;
    result.color = data.data0;
    result.offset = data.data1;
    return result;
}

struct RadialGradient {
    vec4 start_end_center;
    vec4 start_end_radius_ratio_xy_extend_mode;
    vec4 tile_size_repeat;
};

RadialGradient fetch_radial_gradient(int address) {
    ResourceCacheData3 data = fetch_from_resource_cache_3(address);
    RadialGradient result;
    result.start_end_center = data.data0;
    result.start_end_radius_ratio_xy_extend_mode = data.data1;
    result.tile_size_repeat = data.data2;
    return result;
}

struct Glyph {
    vec2 offset;
};

Glyph fetch_glyph(int specific_prim_address,
                  int glyph_index,
                  int subpx_dir) {
    // Two glyphs are packed in each texel in the GPU cache.
    int glyph_address = specific_prim_address +
                        VECS_PER_TEXT_RUN +
                        glyph_index / 2;
    vec4 data = fetch_from_resource_cache_1(glyph_address);
    // Select XY or ZW based on glyph index.
    // We use "!= 0" instead of "== 1" here in order to work around a driver
    // bug with equality comparisons on integers.
    vec2 glyph = mix(data.xy, data.zw, bvec2(glyph_index % 2 == 1, glyph_index % 2 == 1));

    // In subpixel mode, the subpixel offset has already been
    // accounted for while rasterizing the glyph.
    switch (subpx_dir) {
        case SUBPX_DIR_NONE:
            break;
        case SUBPX_DIR_HORIZONTAL:
            // Glyphs positioned [-0.125, 0.125] get a
            // subpx position of zero. So include that
            // offset in the glyph position to ensure
            // we round to the correct whole position.
            glyph.x = floor(glyph.x + 0.125);
            break;
        case SUBPX_DIR_VERTICAL:
            glyph.y = floor(glyph.y + 0.125);
            break;
    }

    Glyph result;
    result.offset = glyph;
    return result;
}

struct PrimitiveInstance {
    int prim_address;
    int specific_prim_address;
    int render_task_index;
    int clip_task_index;
    int layer_index;
    int z;
    int user_data0;
    int user_data1;
    int user_data2;
};

PrimitiveInstance fetch_prim_instance(ivec4 aDataA, ivec4 aDataB) {
    PrimitiveInstance pi;

    pi.prim_address = aDataA.x;
    pi.specific_prim_address = pi.prim_address + VECS_PER_PRIM_HEADER;
    pi.render_task_index = aDataA.y;
    pi.clip_task_index = aDataA.z;
    pi.layer_index = aDataA.w;
    pi.z = aDataB.x;
    pi.user_data0 = aDataB.y;
    pi.user_data1 = aDataB.z;
    pi.user_data2 = aDataB.w;

    return pi;
}

struct CompositeInstance {
    int render_task_index;
    int src_task_index;
    int backdrop_task_index;
    int user_data0;
    int user_data1;
    float z;
};

CompositeInstance fetch_composite_instance(ivec4 aDataA, ivec4 aDataB) {
    CompositeInstance ci;

    ci.render_task_index = aDataA.x;
    ci.src_task_index = aDataA.y;
    ci.backdrop_task_index = aDataA.z;
    ci.z = float(aDataA.w);

    ci.user_data0 = aDataB.x;
    ci.user_data1 = aDataB.y;

    return ci;
}

struct Primitive {
    Layer layer;
    ClipArea clip_area;
#ifdef PRIMITIVE_HAS_PICTURE_TASK
    PictureTask task;
#else
    AlphaBatchTask task;
#endif
    RectWithSize local_rect;
    RectWithSize local_clip_rect;
    int specific_prim_address;
    int user_data0;
    int user_data1;
    int user_data2;
    float z;
};

struct PrimitiveGeometry {
    RectWithSize local_rect;
    RectWithSize local_clip_rect;
};

PrimitiveGeometry fetch_primitive_geometry(int address) {
    ResourceCacheData2 data = fetch_from_resource_cache_2(address);
    PrimitiveGeometry result;
    RectWithSize rect1;
    rect1.p0 = data.data0.xy;
    rect1.size = data.data0.zw;
    result.local_rect = rect1;
    RectWithSize rect2;
    rect2.p0 = data.data1.xy;
    rect2.size = data.data1.zw;
    result.local_clip_rect = rect2;
    return result;
}

Primitive load_primitive(ivec4 aDataA, ivec4 aDataB) {
    PrimitiveInstance pi = fetch_prim_instance(aDataA, aDataB);

    Primitive prim;

    prim.layer = fetch_layer(pi.layer_index);
    prim.clip_area = fetch_clip_area(pi.clip_task_index);
#ifdef PRIMITIVE_HAS_PICTURE_TASK
    prim.task = fetch_picture_task(pi.render_task_index);
#else
    prim.task = fetch_alpha_batch_task(pi.render_task_index);
#endif

    PrimitiveGeometry geom = fetch_primitive_geometry(pi.prim_address);
    prim.local_rect = geom.local_rect;
    prim.local_clip_rect = geom.local_clip_rect;

    prim.specific_prim_address = pi.specific_prim_address;
    prim.user_data0 = pi.user_data0;
    prim.user_data1 = pi.user_data1;
    prim.user_data2 = pi.user_data2;
    prim.z = float(pi.z);

    return prim;
}

// Return the intersection of the plane (set up by "normal" and "point")
// with the ray (set up by "ray_origin" and "ray_dir"),
// writing the resulting scaler into "t".
bool ray_plane(vec3 normal, vec3 point, vec3 ray_origin, vec3 ray_dir, out float t)
{
    float denom = dot(normal, ray_dir);
    if (abs(denom) > 1e-6) {
        vec3 d = point - ray_origin;
        t = dot(d, normal) / denom;
        return t >= 0.0;
    }

    return false;
}

// Apply the inverse transform "inv_transform"
// to the reference point "ref" in CSS space,
// producing a local point on a layer plane,
// set by a base point "a" and a normal "n".
vec4 untransform(vec2 ref, vec3 n, vec3 a, mat4 inv_transform) {
    vec3 p = vec3(ref, -10000.0);
    vec3 d = vec3(0, 0, 1.0);

    float t = 0.0;
    // get an intersection of the layer plane with Z axis vector,
    // originated from the "ref" point
    ray_plane(n, a, p, d, t);
    float z = p.z + d.z * t; // Z of the visible point on the layer

    vec4 r = mul(vec4(ref, z, 1.0), inv_transform);
    return r;
}

// Given a CSS space position, transform it back into the layer space.
vec4 get_layer_pos(vec2 pos, Layer layer) {
    // get a point on the layer plane
    vec4 ah = mul(vec4(0.0, 0.0, 0.0, 1.0), layer.transform);
    vec3 a = ah.xyz / ah.w;
    // get the normal to the layer plane
    vec3 n = mul(vec3(0.0, 0.0, 1.0), transpose(mat3(layer.inv_transform[0].xyz, layer.inv_transform[1].xyz, layer.inv_transform[2].xyz)));
    return untransform(pos, n, a, layer.inv_transform);
}

// Compute a snapping offset in world space (adjusted to pixel ratio),
// given local position on the layer and a snap rectangle.
vec2 compute_snap_offset(vec2 local_pos,
                         RectWithSize local_clip_rect,
                         Layer layer,
                         RectWithSize snap_rect) {
    // Ensure that the snap rect is at *least* one device pixel in size.
    // TODO(gw): It's not clear to me that this is "correct". Specifically,
    //           how should it interact with sub-pixel snap rects when there
    //           is a layer transform with scale present? But it does fix
    //           the test cases we have in Servo that are failing without it
    //           and seem better than not having this at all.
    snap_rect.size = max(snap_rect.size, vec2(1.0 / uDevicePixelRatio, 1.0 / uDevicePixelRatio));

    // Transform the snap corners to the world space.
    vec4 world_snap_p0 = mul(vec4(snap_rect.p0, 0.0, 1.0), layer.transform);
    vec4 world_snap_p1 = mul(vec4(snap_rect.p0 + snap_rect.size, 0.0, 1.0), layer.transform);
    // Snap bounds in world coordinates, adjusted for pixel ratio. XY = top left, ZW = bottom right
    vec4 world_snap = uDevicePixelRatio * vec4(world_snap_p0.xy, world_snap_p1.xy) /
                                          vec4(world_snap_p0.ww, world_snap_p1.ww);
    /// World offsets applied to the corners of the snap rectangle.
    vec4 snap_offsets = floor(world_snap + 0.5) - world_snap;

    /// Compute the position of this vertex inside the snap rectangle.
    vec2 normalized_snap_pos = (local_pos - snap_rect.p0) / snap_rect.size;
    /// Compute the actual world offset for this vertex needed to make it snap.
    return mix(snap_offsets.xy, snap_offsets.zw, normalized_snap_pos);
}

struct VertexInfo {
    vec2 local_pos;
    vec2 screen_pos;
};

VertexInfo write_vertex(vec3 aPosition,
                        RectWithSize instance_rect,
                        RectWithSize local_clip_rect,
                        float z,
                        Layer layer,
                        AlphaBatchTask task,
                        RectWithSize snap_rect
#ifdef WR_DX11
                        , out vec4 gl_Position
#endif //WR_DX11
                        ) {

    // Select the corner of the local rect that we are processing.
    vec2 local_pos = instance_rect.p0 + instance_rect.size * aPosition.xy;

    // Clamp to the two local clip rects.
    vec2 clamped_local_pos = clamp_rect(clamp_rect(local_pos, local_clip_rect),
                                        layer.local_clip_rect);

    /// Compute the snapping offset.
    vec2 snap_offset = compute_snap_offset(clamped_local_pos, local_clip_rect, layer, snap_rect);

    // Transform the current vertex to the world cpace.
    vec4 world_pos = mul(vec4(clamped_local_pos, 0.0, 1.0), layer.transform);

    // Convert the world positions to device pixel space.
    vec2 device_pos = world_pos.xy / world_pos.w * uDevicePixelRatio;

    // Apply offsets for the render task to get correct screen location.
    vec2 final_pos = device_pos + snap_offset -
                     task.screen_space_origin +
                     task.render_target_origin;

    gl_Position = mul(vec4(final_pos, z, 1.0), uTransform);

    VertexInfo vi;
    vi.local_pos = clamped_local_pos;
    vi.screen_pos = device_pos;
    return vi;
}

#ifdef WR_FEATURE_TRANSFORM

struct TransformVertexInfo {
    vec3 local_pos;
    vec2 screen_pos;
};

float cross2(vec2 v0, vec2 v1) {
    return v0.x * v1.y - v0.y * v1.x;
}

// Return intersection of line (p0,p1) and line (p2,p3)
vec2 intersect_lines(vec2 p0, vec2 p1, vec2 p2, vec2 p3) {
    vec2 d0 = p0 - p1;
    vec2 d1 = p2 - p3;

    float s0 = cross2(p0, p1);
    float s1 = cross2(p2, p3);

    float d = cross2(d0, d1);
    float nx = s0 * d1.x - d0.x * s1;
    float ny = s0 * d1.y - d0.y * s1;

    return vec2(nx / d, ny / d);
}

TransformVertexInfo write_transform_vertex(int vertex_id,
                                           RectWithSize instance_rect,
                                           RectWithSize local_clip_rect,
                                           float z,
                                           Layer layer,
                                           AlphaBatchTask task,
                                           RectWithSize snap_rect
#ifdef WR_DX11
                                           , out vec4 gl_Position
                                           , out vec4 vLocalBounds
#endif //WR_DX11
                                           ) {
    RectWithEndpoint local_rect = to_rect_with_endpoint(instance_rect);

    vec2 current_local_pos, prev_local_pos, next_local_pos;

    // Clamp to the two local clip rects.
    local_rect.p0 = clamp_rect(clamp_rect(local_rect.p0, local_clip_rect), layer.local_clip_rect);
    local_rect.p1 = clamp_rect(clamp_rect(local_rect.p1, local_clip_rect), layer.local_clip_rect);

    // Select the current vertex and the previous/next vertices,
    // based on the vertex ID that is known based on the instance rect.
    switch (vertex_id) {
        case 0:
            current_local_pos = vec2(local_rect.p0.x, local_rect.p0.y);
            next_local_pos = vec2(local_rect.p0.x, local_rect.p1.y);
            prev_local_pos = vec2(local_rect.p1.x, local_rect.p0.y);
            break;
        case 1:
            current_local_pos = vec2(local_rect.p1.x, local_rect.p0.y);
            next_local_pos = vec2(local_rect.p0.x, local_rect.p0.y);
            prev_local_pos = vec2(local_rect.p1.x, local_rect.p1.y);
            break;
        case 2:
            current_local_pos = vec2(local_rect.p0.x, local_rect.p1.y);
            prev_local_pos = vec2(local_rect.p0.x, local_rect.p0.y);
            next_local_pos = vec2(local_rect.p1.x, local_rect.p1.y);
            break;
        case 3:
            current_local_pos = vec2(local_rect.p1.x, local_rect.p1.y);
            prev_local_pos = vec2(local_rect.p0.x, local_rect.p1.y);
            next_local_pos = vec2(local_rect.p1.x, local_rect.p0.y);
            break;
        default:
            current_local_pos = vec2(local_rect.p0.x, local_rect.p0.y);
            next_local_pos = vec2(local_rect.p0.x, local_rect.p1.y);
            prev_local_pos = vec2(local_rect.p1.x, local_rect.p0.y);
            break;
    }

    // Transform them to world space
    vec4 current_world_pos = mul(vec4(current_local_pos, 0.0, 1.0), layer.transform);
    vec4 prev_world_pos = mul(vec4(prev_local_pos, 0.0, 1.0), layer.transform);
    vec4 next_world_pos = mul(vec4(next_local_pos, 0.0, 1.0), layer.transform);

    // Convert to device space
    vec2 current_device_pos = uDevicePixelRatio * current_world_pos.xy / current_world_pos.w;
    vec2 prev_device_pos = uDevicePixelRatio * prev_world_pos.xy / prev_world_pos.w;
    vec2 next_device_pos = uDevicePixelRatio * next_world_pos.xy / next_world_pos.w;

    // Get the normals of each of the vectors between the current and next/prev vertices.
    const float amount = 2.0;
    vec2 dir_prev = normalize(current_device_pos - prev_device_pos);
    vec2 dir_next = normalize(current_device_pos - next_device_pos);
    vec2 norm_prev = vec2(-dir_prev.y,  dir_prev.x);
    vec2 norm_next = vec2( dir_next.y, -dir_next.x);

    // Push those lines out along the normal by a specific amount of device pixels.
    vec2 adjusted_prev_p0 = current_device_pos + norm_prev * amount;
    vec2 adjusted_prev_p1 = prev_device_pos + norm_prev * amount;
    vec2 adjusted_next_p0 = current_device_pos + norm_next * amount;
    vec2 adjusted_next_p1 = next_device_pos + norm_next * amount;

    // Intersect those adjusted lines to find the actual vertex position.
    vec2 device_pos = intersect_lines(adjusted_prev_p0,
                                      adjusted_prev_p1,
                                      adjusted_next_p0,
                                      adjusted_next_p1);

    vec4 layer_pos = get_layer_pos(device_pos / uDevicePixelRatio, layer);

    // Apply offsets for the render task to get correct screen location.
    vec2 final_pos = device_pos - //Note: `snap_rect` is not used
                     task.screen_space_origin +
                     task.render_target_origin;

    gl_Position = mul(vec4(final_pos, z, 1.0), uTransform);

    vLocalBounds = vec4(local_rect.p0, local_rect.p1);

    TransformVertexInfo tvi;
    tvi.local_pos = layer_pos.xyw;
    tvi.screen_pos = device_pos;
    return tvi;
}

#endif //WR_FEATURE_TRANSFORM

struct GlyphResource {
    vec4 uv_rect;
    float layer;
    vec2 offset;
    float scale;
};

GlyphResource fetch_glyph_resource(int address) {
    ResourceCacheData2 data = fetch_from_resource_cache_2(address);
    GlyphResource glyph_res;
    glyph_res.uv_rect = data.data0;
    glyph_res.layer = data.data1.x;
    glyph_res.offset = data.data1.yz;
    glyph_res.scale = data.data1.w;
    return glyph_res;
}

struct ImageResource {
    vec4 uv_rect;
    float layer;
};

ImageResource fetch_image_resource(int address) {
    ResourceCacheData2 data = fetch_from_resource_cache_2(address);
    ImageResource img_res;
    img_res.uv_rect = data.data0;
    img_res.layer = data.data1.x;
    return img_res;
}

ImageResource fetch_image_resource_direct(ivec2 address) {
    ResourceCacheData2 data = fetch_from_resource_cache_2_direct(address);
    ImageResource img_res;
    img_res.uv_rect = data.data0;
    img_res.layer = data.data1.x;
    return img_res;
}

struct Rectangle {
    vec4 color;
};

Rectangle fetch_rectangle(int address) {
    vec4 data = fetch_from_resource_cache_1(address);
    Rectangle rect;
    rect.color = data;
    return rect;
}

struct TextRun {
    vec4 color;
    vec2 offset;
    int subpx_dir;
};

TextRun fetch_text_run(int address) {
    ResourceCacheData2 data = fetch_from_resource_cache_2(address);
    TextRun result;
    result.color = data.data0;
    result.offset = data.data1.xy;
    result.subpx_dir = int(data.data1.z);
    return result;
}

struct Image {
    vec4 stretch_size_and_tile_spacing;  // Size of the actual image and amount of space between
                                         //     tiled instances of this image.
    vec4 sub_rect;                          // If negative, ignored.
};

Image fetch_image(int address) {
    ResourceCacheData2 data = fetch_from_resource_cache_2(address);
    Image img;
    img.stretch_size_and_tile_spacing = data.data0;
    img.sub_rect = data.data1;
    return img;
}

void write_clip(vec2 global_pos,
                ClipArea area
#ifdef WR_DX11
                , out vec4 vClipMaskUvBounds
                , out vec3 vClipMaskUv
#endif //WR_DX11
                ) {
    vec2 texture_size = vec2(textureSize(sSharedCacheA8, 0).xy);
    vec2 uv = global_pos + area.task_bounds.xy - area.screen_origin_target_index.xy;
    vClipMaskUvBounds = area.task_bounds / texture_size.xyxy;
    vClipMaskUv = vec3(uv / texture_size, area.screen_origin_target_index.z);
}
#endif //WR_VERTEX_SHADER

#ifdef WR_FRAGMENT_SHADER

/// Find the appropriate half range to apply the AA smoothstep over.
/// This range represents a coefficient to go from one CSS pixel to half a device pixel.
float compute_aa_range(vec2 position) {
    // The constant factor is chosen to compensate for the fact that length(fw) is equal
    // to sqrt(2) times the device pixel ratio in the typical case. 0.5/sqrt(2) = 0.35355.
    //
    // This coefficient is chosen to ensure that any sample 0.5 pixels or more inside of
    // the shape has no anti-aliasing applied to it (since pixels are sampled at their center,
    // such a pixel (axis aligned) is fully inside the border). We need this so that antialiased
    // curves properly connect with non-antialiased vertical or horizontal lines, among other things.
    //
    // Using larger aa steps is quite common when rendering shapes with distance fields.
    // It gives a smoother (although blurrier look) by extending the range that is smoothed
    // to produce the anti aliasing. In our case, however, extending the range inside of
    // the shape causes noticeable artifacts at the junction between an antialiased corner
    // and a straight edge.
    // We may want to adjust this constant in specific scenarios (for example keep the principled
    // value for straight edges where we want pixel-perfect equivalence with non antialiased lines
    // when axis aligned, while selecting a larger and smoother aa range on curves).
    return 0.35355 * length(fwidth(position));
}

/// Return the blending coefficient to for distance antialiasing.
///
/// 0.0 means inside the shape, 1.0 means outside.
float distance_aa(float aa_range, float signed_distance) {
    return 1.0 - smoothstep(-aa_range, aa_range, signed_distance);
}

#ifdef WR_FEATURE_TRANSFORM
float signed_distance_rect(vec2 pos, vec2 p0, vec2 p1) {
    vec2 d = max(p0 - pos, pos - p1);
    return length(max(vec2(0.0, 0.0), d)) + min(0.0, max(d.x, d.y));
}

vec2 init_transform_fs(vec3 local_pos, vec4 vLocalBounds, out float fragment_alpha) {
    fragment_alpha = 1.0;
    vec2 pos = local_pos.xy / local_pos.z;

    // Now get the actual signed distance.
    float d = signed_distance_rect(pos, vLocalBounds.xy, vLocalBounds.zw);

    // Find the appropriate distance to apply the AA smoothstep over.
    float aa_range = compute_aa_range(pos.xy);

    // Only apply AA to fragments outside the signed distance field.
    fragment_alpha = distance_aa(aa_range, d);

    return pos;
}
#endif //WR_FEATURE_TRANSFORM

float do_clip(vec4 vClipMaskUvBounds, vec3 vClipMaskUv) {
    // anything outside of the mask is considered transparent
    bvec4 inside = lessThanEqual(
        vec4(vClipMaskUvBounds.xy, vClipMaskUv.xy),
        vec4(vClipMaskUv.xy, vClipMaskUvBounds.zw));
    // check for the dummy bounds, which are given to the opaque objects
    return ((vClipMaskUvBounds.x == vClipMaskUvBounds.z)
            && (vClipMaskUvBounds.y == vClipMaskUvBounds.w)) ? 1.0:
        all(inside) ? textureLod(sSharedCacheA8, vClipMaskUv, 0.0).r : 0.0;
}

#ifdef WR_FEATURE_DITHERING
uniform sampler2D sDither;
vec4 dither(vec4 color, vec4 frag_coord) {
    const int matrix_mask = 7;

    ivec2 pos = ivec2(frag_coord.xy) & ivec2(matrix_mask, matrix_mask);
    float noise_normalized = (texelFetch(sDither, pos, 0).r * 255.0 + 0.5) / 64.0;
    float noise = (noise_normalized - 0.5) / 256.0; // scale down to the unit length

    return color + vec4(noise, noise, noise, 0);
}
#else
vec4 dither(vec4 color, vec4 frag_coord) {
    return color;
}
#endif //WR_FEATURE_DITHERING

vec4 sample_gradient(int address, float offset, float gradient_repeat, vec4 frag_coord) {
    // Modulo the offset if the gradient repeats.
    float x = mix(offset, fract(offset), gradient_repeat);

    // Calculate the color entry index to use for this offset:
    //     offsets < 0 use the first color entry, 0
    //     offsets from [0, 1) use the color entries in the range of [1, N-1)
    //     offsets >= 1 use the last color entry, N-1
    //     so transform the range [0, 1) -> [1, N-1)

    // TODO(gw): In the future we might consider making the size of the
    // LUT vary based on number / distribution of stops in the gradient.
    const int GRADIENT_ENTRIES = 128;
    x = 1.0 + x * float(GRADIENT_ENTRIES);

    // Calculate the texel to index into the gradient color entries:
    //     floor(x) is the gradient color entry index
    //     fract(x) is the linear filtering factor between start and end
    int lut_offset = 2 * int(floor(x));     // There is a [start, end] color per entry.

    // Ensure we don't fetch outside the valid range of the LUT.
    lut_offset = clamp(lut_offset, 0, 2 * (GRADIENT_ENTRIES + 1));

    // Fetch the start and end color.
    ResourceCacheData2 texels = fetch_from_resource_cache_2(address + lut_offset);

    // Finally interpolate and apply dithering
    return dither(mix(texels.data0, texels.data1, fract(x)), frag_coord);
}

#endif //WR_FRAGMENT_SHADER
