#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::globals

#import bevy_render::view::View
@group(0) @binding(0) 
var<uniform> view: View;

@group(2) @binding(100)
var<uniform> color: vec4<f32>;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {

    let white = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    let black = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    let black_transparent = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    let uv = (in.uv * 2.0) - 1.0;
    let time = globals.time;

    let uv_scrolled = vec2<f32>(uv.x + time * 0.1, uv.y + time * -0.4);

    let uv_distorted = vec2<f32>(
        uv.x + sin(uv_scrolled.y * 0.1 + time * 0.2) * 0.1 + cos(uv_scrolled.x * 1.0 + time * 0.2) * 0.1,
        uv.y + cos(uv_scrolled.x * 0.1 + time * 0.2) * 0.1 + sin(uv_scrolled.y * 1.0 + time * 0.2) * 0.1
    );

    let distorted_uv4 = rotate(uv_distorted, time * 0.2);

    var total = 0.0;
    var frequency = 2.1;
    var amplitude = 1.0;
    var max = 0.0;

    for (var i = 0; i < 2; i = i + 1) {

        if i == 0 {
            total += perlin((uv_scrolled + time * 0.4) * frequency);
        } else {
            total += perlin((uv_scrolled + time * 0.1) * frequency);
        }

        max += amplitude;

        amplitude *= 0.5;
        frequency *= 2.0;
    }

    let smoke = total / max;

    let outer_core = 1.0 - soft_circle(distorted_uv4, -1.0, 1.75);
    let inner_core = 1.0 - soft_circle(distorted_uv4, -0.2, 1.0);

    let final_alpha = mix(
        black_transparent,
        (color * inner_core * (1.0 - smoke)) + white * (1.0 - smoke),
        outer_core * (1.0 - smoke)
    );

    return vec4<f32>(final_alpha.rgb * color.a, final_alpha.a * color.a);
}

fn rotate(uv: vec2<f32>, angle: f32) -> vec2<f32> {
    let s = sin(angle);
    let c = cos(angle);
    let x = uv.x * c - uv.y * s;
    let y = uv.x * s + uv.y * c;
    return vec2<f32>(x, y);
}

fn permute4(x: vec4f) -> vec4f { return ((x * 34. + 1.) * x) % vec4f(289.); }
fn fade2(t: vec2f) -> vec2f { return t * t * t * (t * (t * 6. - 15.) + 10.); }

fn soft_circle(uv: vec2<f32>, radius: f32, softness: f32) -> f32 {
    return smoothstep(radius - softness, radius + softness, length(uv));
}

// taken from: https://gist.github.com/munrocket/236ed5ba7e409b8bdf1ff6eca5dcdc39
fn perlin(P: vec2f) -> f32 {
    var Pi: vec4f = floor(P.xyxy) + vec4f(0., 0., 1., 1.);
    let Pf = fract(P.xyxy) - vec4f(0., 0., 1., 1.);
    Pi = Pi % vec4f(289.); // To avoid truncation effects in permutation
    let ix = Pi.xzxz;
    let iy = Pi.yyww;
    let fx = Pf.xzxz;
    let fy = Pf.yyww;
    let i = permute4(permute4(ix) + iy);
    var gx: vec4f = 2. * fract(i * 0.0243902439) - 1.; // 1/41 = 0.024...
    let gy = abs(gx) - 0.5;
    let tx = floor(gx + 0.5);
    gx = gx - tx;
    var g00: vec2f = vec2f(gx.x, gy.x);
    var g10: vec2f = vec2f(gx.y, gy.y);
    var g01: vec2f = vec2f(gx.z, gy.z);
    var g11: vec2f = vec2f(gx.w, gy.w);
    let norm = 1.79284291400159 - 0.85373472095314 * vec4f(dot(g00, g00), dot(g01, g01), dot(g10, g10), dot(g11, g11));
    g00 = g00 * norm.x;
    g01 = g01 * norm.y;
    g10 = g10 * norm.z;
    g11 = g11 * norm.w;
    let n00 = dot(g00, vec2f(fx.x, fy.x));
    let n10 = dot(g10, vec2f(fx.y, fy.y));
    let n01 = dot(g01, vec2f(fx.z, fy.z));
    let n11 = dot(g11, vec2f(fx.w, fy.w));
    let fade_xy = fade2(Pf.xy);
    let n_x = mix(vec2f(n00, n01), vec2f(n10, n11), vec2f(fade_xy.x));
    let n_xy = mix(n_x.x, n_x.y, fade_xy.y);
    return 2.3 * n_xy;
}