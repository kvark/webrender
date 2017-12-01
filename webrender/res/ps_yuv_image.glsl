/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#include shared,prim_shared

#ifdef WR_DX11
    struct v2p {
        vec4 Position : SV_Position;
        flat vec4 vClipMaskUvBounds : vClipMaskUvBounds;
        vec3 vClipMaskUv : vClipMaskUv;
        flat vec2 vTextureOffsetY : vTextureOffsetY; // Offset of the y plane into the texture atlas.
        flat vec2 vTextureOffsetU : vTextureOffsetU; // Offset of the u plane into the texture atlas.
        flat vec2 vTextureOffsetV : vTextureOffsetV; // Offset of the v plane into the texture atlas.
        flat vec2 vTextureSizeY : vTextureSizeY;   // Size of the y plane in the texture atlas.
        flat vec2 vTextureSizeUv : vTextureSizeUv;  // Size of the u and v planes in the texture atlas.
        flat vec2 vStretchSize : vStretchSize;
        flat vec2 vHalfTexelY : vHalfTexelY;     // Normalized length of the half of a Y texel.
        flat vec2 vHalfTexelUv : vHalfTexelUv;    // Normalized length of the half of u and v texels.
        flat vec3 vLayers : vLayers;

#ifdef WR_FEATURE_TRANSFORM
        vec3 vLocalPos: vLocalPos;
        flat vec4 vLocalRect : vLocalRect;
        flat vec4 vLocalBounds : vLocalBounds;
#else
        vec2 vLocalPos: vLocalPos;
#endif //WR_FEATURE_TRANSFORM
    };
#else
// If this is in WR_FEATURE_TEXTURE_RECT mode, the rect and size use non-normalized
// texture coordinates. Otherwise, it uses normalized texture coordinates. Please
// check GL_TEXTURE_RECTANGLE.
flat varying vec2 vTextureOffsetY; // Offset of the y plane into the texture atlas.
flat varying vec2 vTextureOffsetU; // Offset of the u plane into the texture atlas.
flat varying vec2 vTextureOffsetV; // Offset of the v plane into the texture atlas.
flat varying vec2 vTextureSizeY;   // Size of the y plane in the texture atlas.
flat varying vec2 vTextureSizeUv;  // Size of the u and v planes in the texture atlas.
flat varying vec2 vStretchSize;
flat varying vec2 vHalfTexelY;     // Normalized length of the half of a Y texel.
flat varying vec2 vHalfTexelUv;    // Normalized length of the half of u and v texels.
flat varying vec3 vLayers;

#ifdef WR_FEATURE_TRANSFORM
varying vec3 vLocalPos;
flat varying vec4 vLocalRect;
#else
varying vec2 vLocalPos;
#endif //WR_FEATURE_TRANSFORM
#endif //WR_DX11

#ifdef WR_VERTEX_SHADER
struct YuvImage {
    vec2 size;
};

YuvImage fetch_yuv_image(int address) {
    vec4 data = fetch_from_resource_cache_1(address);
    YuvImage yuv_image;
    yuv_image.size = data.xy;
    return yuv_image;
}

#ifndef WR_DX11
void main(void) {
#else
void main(in a2v IN, out v2p OUT) {
    vec3 aPosition = IN.pos;
    ivec4 aDataA = IN.data0;
    ivec4 aDataB = IN.data1;
    int gl_VertexID = IN.vertexId;
#endif //WR_DX11
    Primitive prim = load_primitive(aDataA, aDataB);
#ifdef WR_FEATURE_TRANSFORM
    TransformVertexInfo vi = write_transform_vertex(gl_VertexID,
                                                    prim.local_rect,
                                                    prim.local_clip_rect,
                                                    prim.z,
                                                    prim.layer,
                                                    prim.task,
                                                    prim.local_rect
#ifdef WR_DX11
                                                    , OUT.Position
                                                    , OUT.vLocalBounds
#endif //WR_DX11
                                                    );
    SHADER_OUT(vLocalPos, vi.local_pos);
    SHADER_OUT(vLocalRect, vec4(prim.local_rect.p0, prim.local_rect.p0 + prim.local_rect.size));
#else
    VertexInfo vi = write_vertex(aPosition,
                                 prim.local_rect,
                                 prim.local_clip_rect,
                                 prim.z,
                                 prim.layer,
                                 prim.task,
                                 prim.local_rect
#ifdef WR_DX11
                                 , OUT.Position
#endif //WR_DX11
                                 );
    SHADER_OUT(vLocalPos, vi.local_pos - prim.local_rect.p0);
#endif

    write_clip(vi.screen_pos,
               prim.clip_area
#ifdef WR_DX11
               , OUT.vClipMaskUvBounds
               , OUT.vClipMaskUv
#endif //WR_DX11
               );

    ImageResource y_rect = fetch_image_resource(prim.user_data0);
    SHADER_OUT(vLayers, vec3(y_rect.layer, 0.0, 0.0));

#ifndef WR_FEATURE_INTERLEAVED_Y_CB_CR  // only 1 channel
    ImageResource u_rect = fetch_image_resource(prim.user_data1);
    SHADER_OUT(vLayers.y, u_rect.layer);
#ifndef WR_FEATURE_NV12 // 2 channel
    ImageResource v_rect = fetch_image_resource(prim.user_data2);
    SHADER_OUT(vLayers.z, v_rect.layer);
#endif
#endif

    // If this is in WR_FEATURE_TEXTURE_RECT mode, the rect and size use
    // non-normalized texture coordinates.
#ifdef WR_FEATURE_TEXTURE_RECT
    vec2 y_texture_size_normalization_factor = vec2(1, 1);
#else
    vec2 y_texture_size_normalization_factor = vec2(textureSize(sColor0, 0));
#endif
    vec2 y_st0 = y_rect.uv_rect.xy / y_texture_size_normalization_factor;
    vec2 y_st1 = y_rect.uv_rect.zw / y_texture_size_normalization_factor;

    SHADER_OUT(vTextureSizeY, y_st1 - y_st0);
    SHADER_OUT(vTextureOffsetY, y_st0);

#ifndef WR_FEATURE_INTERLEAVED_Y_CB_CR
    // This assumes the U and V surfaces have the same size.
#ifdef WR_FEATURE_TEXTURE_RECT
    vec2 uv_texture_size_normalization_factor = vec2(1, 1);
#else
    vec2 uv_texture_size_normalization_factor = vec2(textureSize(sColor1, 0));
#endif
    vec2 u_st0 = u_rect.uv_rect.xy / uv_texture_size_normalization_factor;
    vec2 u_st1 = u_rect.uv_rect.zw / uv_texture_size_normalization_factor;

#ifndef WR_FEATURE_NV12
    vec2 v_st0 = v_rect.uv_rect.xy / uv_texture_size_normalization_factor;
#endif

    SHADER_OUT(vTextureSizeUv, u_st1 - u_st0);
    SHADER_OUT(vTextureOffsetU, u_st0);
#ifndef WR_FEATURE_NV12
    SHADER_OUT(vTextureOffsetV, v_st0);
#endif
#endif

    YuvImage image = fetch_yuv_image(prim.specific_prim_address);
    SHADER_OUT(vStretchSize, image.size);

    SHADER_OUT(vHalfTexelY, vec2(0.5, 0.5) / y_texture_size_normalization_factor);
#ifndef WR_FEATURE_INTERLEAVED_Y_CB_CR
    SHADER_OUT(vHalfTexelUv, vec2(0.5, 0.5) / uv_texture_size_normalization_factor);
#endif
}
#endif

#ifdef WR_FRAGMENT_SHADER
#if !defined(WR_FEATURE_YUV_REC601) && !defined(WR_FEATURE_YUV_REC709)
#define WR_FEATURE_YUV_REC601
#endif

// The constants added to the Y, U and V components are applied in the fragment shader.
#if defined(WR_FEATURE_YUV_REC601)
// From Rec601:
// [R]   [1.1643835616438356,  0.0,                 1.5960267857142858   ]   [Y -  16]
// [G] = [1.1643835616438358, -0.3917622900949137, -0.8129676472377708   ] x [U - 128]
// [B]   [1.1643835616438356,  2.017232142857143,   8.862867620416422e-17]   [V - 128]
//
// For the range [0,1] instead of [0,255].
//
// The matrix is stored in column-major.
static const mat3 YuvColorMatrix = mat3(
    1.16438,  1.16438, 1.16438,
    0.0,     -0.39176, 2.01723,
    1.59603, -0.81297, 0.0
);
#elif defined(WR_FEATURE_YUV_REC709)
// From Rec709:
// [R]   [1.1643835616438356,  4.2781193979771426e-17, 1.7927410714285714]   [Y -  16]
// [G] = [1.1643835616438358, -0.21324861427372963,   -0.532909328559444 ] x [U - 128]
// [B]   [1.1643835616438356,  2.1124017857142854,     0.0               ]   [V - 128]
//
// For the range [0,1] instead of [0,255]:
//
// The matrix is stored in column-major.
static const mat3 YuvColorMatrix = mat3(
    1.16438,  1.16438,  1.16438,
    0.0    , -0.21325,  2.11240,
    1.79274, -0.53291,  0.0
);
#endif

#ifndef WR_DX11
void main(void) {
#else
void main(in v2p IN, out p2f OUT) {
    vec4 vClipMaskUvBounds = IN.vClipMaskUvBounds;
    vec3 vClipMaskUv = IN.vClipMaskUv;
    vec2 vTextureOffsetY = IN.vTextureOffsetY;
    vec2 vTextureOffsetU = IN.vTextureOffsetU;
    vec2 vTextureOffsetV = IN.vTextureOffsetV;
    vec2 vTextureSizeY = IN.vTextureSizeY;
    vec2 vTextureSizeUv = IN.vTextureSizeUv;
    vec2 vStretchSize = IN.vStretchSize;
    vec2 vHalfTexelY = IN.vHalfTexelY;
    vec2 vHalfTexelUv = IN.vHalfTexelUv;
    vec3 vLayers = IN.vLayers;
#ifdef WR_FEATURE_TRANSFORM
    vec3 vLocalPos = IN.vLocalPos;
    vec4 vLocalRect = IN.vLocalRect;
    vec4 vLocalBounds = IN.vLocalBounds;
#else
    vec2 vLocalPos = IN.vLocalPos;
#endif //WR_FEATURE_TRANSFORM
#endif //WR_DX11
#ifdef WR_FEATURE_TRANSFORM
    float alpha = 0.0;
    vec2 pos = init_transform_fs(vLocalPos, vLocalBounds, alpha);

    // We clamp the texture coordinate calculation here to the local rectangle boundaries,
    // which makes the edge of the texture stretch instead of repeat.
    vec2 relative_pos_in_rect = clamp(pos, vLocalRect.xy, vLocalRect.zw) - vLocalRect.xy;
#else
    float alpha = 1.0;;
    vec2 relative_pos_in_rect = vLocalPos;
#endif

    alpha *= do_clip(vClipMaskUvBounds, vClipMaskUv);

    // We clamp the texture coordinates to the half-pixel offset from the borders
    // in order to avoid sampling outside of the texture area.
    vec2 st_y = vTextureOffsetY + clamp(
        relative_pos_in_rect / vStretchSize * vTextureSizeY,
        vHalfTexelY, vTextureSizeY - vHalfTexelY);
#ifndef WR_FEATURE_INTERLEAVED_Y_CB_CR
    vec2 uv_offset = clamp(
        relative_pos_in_rect / vStretchSize * vTextureSizeUv,
        vHalfTexelUv, vTextureSizeUv - vHalfTexelUv);
    // NV12 only uses 2 textures. The sColor0 is for y and sColor1 is for uv.
    // The texture coordinates of u and v are the same. So, we could skip the
    // st_v if the format is NV12.
    vec2 st_u = vTextureOffsetU + uv_offset;
#endif

    vec3 yuv_value;
#ifdef WR_FEATURE_INTERLEAVED_Y_CB_CR
    // "The Y, Cb and Cr color channels within the 422 data are mapped into
    // the existing green, blue and red color channels."
    // https://www.khronos.org/registry/OpenGL/extensions/APPLE/APPLE_rgb_422.txt
    yuv_value = TEX_SAMPLE(sColor0, vec3(st_y, vLayers.x)).gbr;
#elif defined(WR_FEATURE_NV12)
    yuv_value.x = TEX_SAMPLE(sColor0, vec3(st_y, vLayers.x)).r;
    yuv_value.yz = TEX_SAMPLE(sColor1, vec3(st_u, vLayers.y)).rg;
#else
    // The yuv_planar format should have this third texture coordinate.
    vec2 st_v = vTextureOffsetV + uv_offset;

    yuv_value.x = TEX_SAMPLE(sColor0, vec3(st_y, vLayers.x)).r;
    yuv_value.y = TEX_SAMPLE(sColor1, vec3(st_u, vLayers.y)).r;
    yuv_value.z = TEX_SAMPLE(sColor2, vec3(st_v, vLayers.z)).r;
#endif

    // See the YuvColorMatrix definition for an explanation of where the constants come from.
    vec3 yuv_val = yuv_value - vec3(0.06275, 0.50196, 0.50196);
    vec3 rgb = mul(yuv_val, YuvColorMatrix);
    SHADER_OUT(Target0, vec4(rgb, alpha));
}
#endif
