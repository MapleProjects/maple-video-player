// Advanced Gradient Smoothing and Bit-Depth Band Extension Shader
// Removes 8-bit quantization steps, color banding and compression contouring
//!HOOK MAIN
//!BIND HOOKED
//!DESC Advanced Gradient Bit-Depth Band Smoothing & Dither
//!WIDTH OUTPUT.w
//!HEIGHT OUTPUT.h

#define THRESHOLD 0.03
#define RANGE 12.0
#define GRAIN 0.002

float hash(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

float tpdf(vec2 p) {
    float r1 = hash(p);
    float r2 = hash(p + vec2(1.337, 7.331));
    return (r1 + r2 - 1.0);
}

vec4 hook() {
    vec4 c = HOOKED_tex(HOOKED_pos);
    vec2 pt = HOOKED_pt;

    vec3 avg = vec3(0.0);
    float total_w = 0.0;

    for (int i = 0; i < 8; i++) {
        float angle = float(i) * 0.78539816;
        vec2 dir = vec2(cos(angle), sin(angle));
        vec4 sample_c = HOOKED_tex(HOOKED_pos + dir * pt * RANGE);
        vec3 diff = abs(sample_c.rgb - c.rgb);

        float max_diff = max(max(diff.r, diff.g), diff.b);
        if (max_diff < THRESHOLD) {
            float w = 1.0 - (max_diff / THRESHOLD);
            avg += sample_c.rgb * w;
            total_w += w;
        }
    }

    vec3 smoothed = c.rgb;
    if (total_w > 0.001) {
        smoothed = avg / total_w;
    }

    // High frequency TPDF dither to expand to 10-12 bit gradients
    float d = tpdf(HOOKED_pos * 1000.0) * GRAIN;
    vec3 result = clamp(smoothed + vec3(d), 0.0, 1.0);

    return vec4(result, c.a);
}
