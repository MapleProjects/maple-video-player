// Anime4K v3.2 - Fast Neural / Bilateral Line & Texture Reconstruction
//!HOOK MAIN
//!BIND HOOKED
//!DESC Anime4K Texture & Edge Reconstruction (GLSL)
//!WIDTH OUTPUT.w
//!HEIGHT OUTPUT.h

vec4 hook() {
    vec4 c = HOOKED_tex(HOOKED_pos);
    vec4 n = HOOKED_texOff(vec2(0.0, -1.0));
    vec4 s = HOOKED_texOff(vec2(0.0, 1.0));
    vec4 w = HOOKED_texOff(vec2(-1.0, 0.0));
    vec4 e = HOOKED_texOff(vec2(1.0, 0.0));

    vec4 nw = HOOKED_texOff(vec2(-1.0, -1.0));
    vec4 ne = HOOKED_texOff(vec2(1.0, -1.0));
    vec4 sw = HOOKED_texOff(vec2(-1.0, 1.0));
    vec4 se = HOOKED_texOff(vec2(1.0, 1.0));

    // Perceptual luma
    vec3 lum = vec3(0.2627, 0.6780, 0.0593);
    float l_c = dot(c.rgb, lum);
    float l_n = dot(n.rgb, lum);
    float l_s = dot(s.rgb, lum);
    float l_w = dot(w.rgb, lum);
    float l_e = dot(e.rgb, lum);

    // Sobel gradients
    float gx = (ne.r + 2.0 * e.r + se.r) - (nw.r + 2.0 * w.r + sw.r);
    float gy = (sw.r + 2.0 * s.r + se.r) - (nw.r + 2.0 * n.r + ne.r);
    float edge = sqrt(gx * gx + gy * gy);

    // Bilateral edge refinement
    float min_l = min(min(min(l_n, l_s), min(l_w, l_e)), l_c);
    float max_l = max(max(max(l_n, l_s), max(l_w, l_e)), l_c);

    vec3 sharp = c.rgb + (c.rgb * 4.0 - n.rgb - s.rgb - w.rgb - e.rgb) * (edge * 0.35 + 0.15);
    sharp = clamp(sharp, min_l * 0.95, max_l * 1.05);

    return vec4(clamp(sharp, 0.0, 1.0), c.a);
}
