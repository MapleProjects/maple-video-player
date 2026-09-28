// AMD FidelityFX Contrast Adaptive Sharpening (CAS)
// Ported for MPV / Libplacebo GLSL User Shader
//!HOOK MAIN
//!BIND HOOKED
//!DESC FidelityFX Contrast Adaptive Sharpening (CAS)
//!WIDTH OUTPUT.w
//!HEIGHT OUTPUT.h

#define SHARPNESS 0.60 // Default sharpness [0.0 - 2.0]

vec4 hook() {
    vec4 e = HOOKED_tex(HOOKED_pos);
    vec4 b = HOOKED_texOff(vec2(0.0, -1.0));
    vec4 d = HOOKED_texOff(vec2(-1.0, 0.0));
    vec4 f = HOOKED_texOff(vec2(1.0, 0.0));
    vec4 h = HOOKED_texOff(vec2(0.0, 1.0));

    // RGB to luma (Rec. 709 / Rec. 2020 perceptual weights)
    vec3 luma_w = vec3(0.2627, 0.6780, 0.0593);
    float b_l = dot(b.rgb, luma_w);
    float d_l = dot(d.rgb, luma_w);
    float e_l = dot(e.rgb, luma_w);
    float f_l = dot(f.rgb, luma_w);
    float h_l = dot(h.rgb, luma_w);

    // Min and max luma in cross neighborhood
    float mn = min(min(min(b_l, d_l), min(f_l, h_l)), e_l);
    float mx = max(max(max(b_l, d_l), max(f_l, h_l)), e_l);

    // Smooth reciprocal with responsive sharpness weighting
    float amp = clamp(min(mn, 1.0 - mx) / max(mx - mn, 0.0001), 0.0, 0.25);
    float w = -sqrt(amp) * (SHARPNESS * 0.45);

    // Filter RGB
    vec3 col = (b.rgb + d.rgb + f.rgb + h.rgb) * w + e.rgb;
    col = clamp(col / (4.0 * w + 1.0), 0.0, 1.0);

    return vec4(col, e.a);
}
