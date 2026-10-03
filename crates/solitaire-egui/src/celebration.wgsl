// One draw call, twelve bursts. Particle motion and light are computed on GPU.
struct Scene { values: vec4<f32> }
@group(0) @binding(0) var<uniform> scene: Scene;

struct Spark {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) fade: f32,
    @location(3) ring: f32,
}

fn hash(value: f32) -> f32 {
    return fract(sin(value * 127.1 + 311.7) * 43758.5453);
}

@vertex
fn spark_vertex(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> Spark {
    let corners = array<vec2<f32>, 6>(
        vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(-1.0, 1.0),
        vec2(-1.0, 1.0), vec2(1.0, -1.0), vec2(1.0, 1.0)
    );
    let uv = corners[vertex];
    let burst = instance / 96u;
    let particle = instance % 96u;
    let time = scene.values.x;
    let size = scene.values.yz;
    let scale = min(size.x, size.y) / 650.0;
    let start = f32(burst / 4u) * 1.65 + f32(burst % 4u) * 0.25;
    let age = max(0.0, time - start);
    let id = f32(instance) + 3.0;
    let angle = f32(particle) * 2.399963 + hash(id) * 0.3;
    let velocity = vec2(cos(angle), sin(angle)) * (140.0 + hash(id + 5.0) * 240.0) * scale;
    let origin = vec2(0.13 + hash(f32(burst) + 1.0) * 0.74,
                      0.17 + hash(f32(burst) + 8.0) * 0.36) * size;
    let motion = velocity * age + vec2(0.0, 120.0 * age * age * scale);
    let direction = normalize(velocity + vec2(0.0, 240.0 * age * scale));
    let side = vec2(-direction.y, direction.x);
    var point = origin + motion + (direction * uv.x * 12.0 + side * uv.y * 5.0) * scale;
    var fade = (1.0 - smoothstep(0.7, 2.5, age)) * smoothstep(0.0, 0.06, age);
    var ring = 0.0;
    if particle == 0u {
        point = origin + uv * (25.0 + age * 210.0) * scale;
        fade = (1.0 - smoothstep(0.05, 0.75, age)) * smoothstep(0.0, 0.025, age);
        ring = 1.0;
    }
    if time < start { fade = 0.0; }
    var color = vec3(1.0, 0.76, 0.32);
    if burst % 4u == 1u { color = vec3(0.25, 0.95, 0.84); }
    if burst % 4u == 2u { color = vec3(1.0, 0.43, 0.53); }
    if burst % 4u == 3u { color = vec3(0.7, 0.62, 1.0); }
    var spark: Spark;
    spark.position = vec4(point.x / size.x * 2.0 - 1.0, 1.0 - point.y / size.y * 2.0, 0.0, 1.0);
    spark.uv = uv;
    spark.color = color;
    spark.fade = fade;
    spark.ring = ring;
    return spark;
}

@fragment
fn spark_fragment(spark: Spark) -> @location(0) vec4<f32> {
    let radius = length(spark.uv);
    var light = exp(-radius * radius * 18.0) + exp(-radius * radius * 3.0) * 0.16;
    if spark.ring > 0.5 {
        light = exp(-abs(radius - 0.72) * 42.0) * 0.36 + exp(-radius * radius * 6.0) * 0.08;
    }
    let color = mix(spark.color, vec3(1.0), exp(-radius * radius * 40.0) * 0.75);
    return vec4(color, min(1.0, light * spark.fade * 2.0));
}
