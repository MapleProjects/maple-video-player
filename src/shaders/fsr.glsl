// AMD FidelityFX Super Resolution v1.0.2 (EASU + RCAS)

// Pass 1: Edge-Adaptive Spatial Upsampling (Reconstruction)
//!HOOK MAIN
//!BIND HOOKED
//!SAVE EASUTEX
//!DESC AMD FidelityFX FSR 1.0 (EASU Edge-Adaptive Reconstruction)
//!WIDTH OUTPUT.w
//!HEIGHT OUTPUT.h

#define FSR_EASU_DIR_THRESHOLD 0.25
#define FSR_EASU_DERING 1

vec4 hook() {
    vec2 pp = HOOKED_pos * HOOKED_size - vec2(0.5);
    vec2 fp = floor(pp);
    pp -= fp;

    vec4 f = HOOKED_tex(vec2((fp + vec2(0.5, 0.5)) * HOOKED_pt));
    vec4 g = HOOKED_tex(vec2((fp + vec2(1.5, 0.5)) * HOOKED_pt));
    vec4 j = HOOKED_tex(vec2((fp + vec2(0.5, 1.5)) * HOOKED_pt));
    vec4 k = HOOKED_tex(vec2((fp + vec2(1.5, 1.5)) * HOOKED_pt));

    // Directional edge estimation
    vec3 lum = vec3(0.2627, 0.6780, 0.0593);
    float lf = dot(f.rgb, lum);
    float lg = dot(g.rgb, lum);
    float lj = dot(j.rgb, lum);
    float lk = dot(k.rgb, lum);

    float dirX = (lg - lf) + (lk - lj);
    float dirY = (lj - lf) + (lk - lg);
    float edgeStrength = abs(dirX) + abs(dirY);

    vec4 top = mix(f, g, pp.x);
    vec4 bot = mix(j, k, pp.x);
    vec4 mid = mix(top, bot, pp.y);

    if (edgeStrength > FSR_EASU_DIR_THRESHOLD) {
        float dirLen = max(length(vec2(dirX, dirY)), 0.0001);
        vec2 dirNorm = vec2(dirX, dirY) / dirLen;
        vec4 edgeSample1 = HOOKED_tex(HOOKED_pos + dirNorm * HOOKED_pt * 0.5);
        vec4 edgeSample2 = HOOKED_tex(HOOKED_pos - dirNorm * HOOKED_pt * 0.5);
        mid = mix(mid, (edgeSample1 + edgeSample2) * 0.5, 0.45);
    }

#if (FSR_EASU_DERING == 1)
    vec4 min_val = min(min(f, g), min(j, k));
    vec4 max_val = max(max(f, g), max(j, k));
    mid = clamp(mid, min_val, max_val);
#endif

    return mid;
}

// Pass 2: Robust Contrast Adaptive Sharpening (RCAS)
//!HOOK MAIN
//!BIND EASUTEX
//!DESC AMD FidelityFX FSR 1.0 (RCAS Sharpening)
//!WIDTH EASUTEX.w
//!HEIGHT EASUTEX.h

#define SHARPNESS 0.80
#define FSR_RCAS_DENOISE 0.20
#define FSR_RCAS_LIMIT (0.25 - (1.0 / 16.0))

float APrxMedRcpF1(float a) {
    float b = uintBitsToFloat(uint(0x7ef19fff) - floatBitsToUint(max(a, 0.00001)));
    return b * (-b * a + 2.0);
}

vec4 hook() {
    vec4 b_rgba = EASUTEX_texOff(vec2(0.0, -1.0));
    vec4 d_rgba = EASUTEX_texOff(vec2(-1.0, 0.0));
    vec4 e_rgba = EASUTEX_tex(EASUTEX_pos);
    vec4 f_rgba = EASUTEX_texOff(vec2(1.0, 0.0));
    vec4 h_rgba = EASUTEX_texOff(vec2(0.0, 1.0));

    vec3 lum = vec3(0.2627, 0.6780, 0.0593);
    float b = dot(b_rgba.rgb, lum);
    float d = dot(d_rgba.rgb, lum);
    float e = dot(e_rgba.rgb, lum);
    float f = dot(f_rgba.rgb, lum);
    float h = dot(h_rgba.rgb, lum);

    float mn = min(min(min(b, d), f), min(h, e));
    float mx = max(max(max(b, d), f), max(h, e));

    vec2 peakC = vec2(1.0, -4.0);
    float hitMin = min(mn, e) / (4.0 * mx + 0.00001);
    float hitMax = (peakC.x - max(mx, e)) / (4.0 * mn + peakC.y);
    float lobe = max(-hitMin, hitMax);
    lobe = max(float(-FSR_RCAS_LIMIT), min(lobe, 0.0)) * (SHARPNESS * 1.5);

    // Denoise attenuation
    float contrast = mx - mn;
    float denoiseAtten = clamp(contrast / max(FSR_RCAS_DENOISE, 0.001), 0.0, 1.0);
    lobe *= denoiseAtten;

    float rcpL = APrxMedRcpF1(4.0 * lobe + 1.0);
    vec3 result = (lobe * b_rgba.rgb + lobe * d_rgba.rgb + lobe * h_rgba.rgb + lobe * f_rgba.rgb + e_rgba.rgb) * rcpL;
    return vec4(clamp(result, 0.0, 1.0), e_rgba.a);
}
