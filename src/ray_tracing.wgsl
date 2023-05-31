const CHUNK_SIZE: i32 = 4;

@group(0)
@binding(0)
var output_texture: texture_storage_2d<rgba8unorm, write>;

struct Camera {
    position: vec4<f32>,
    forward: vec4<f32>,
    right: vec4<f32>,
    up: vec4<f32>,
    fov: f32,
    max_distance: f32,
}

@group(1)
@binding(0)
var<uniform> camera: Camera;

struct Material {
    color: vec3<f32>,
}

struct Materials {
    count: u32,
    data: array<Material>,
}

@group(2)
@binding(0)
var<storage> materials: Materials;

struct Voxel {
    material: u32,
}

@group(2)
@binding(1)
var<storage> chunk: array<Voxel>;

struct Ray {
    origin: vec4<f32>,
    direction: vec4<f32>,
}

struct Hit {
    hit: bool,
    block_index: u32,
    distance: f32,
    position: vec4<f32>,
    normal: vec4<f32>,
}

fn get_block_index(position: vec4<i32>) -> u32 {
    // TODO: update this when there are multiple chunks
    return u32(position.x + position.y * CHUNK_SIZE + position.z * CHUNK_SIZE * CHUNK_SIZE + position.w * CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE);
}

fn get_intersection(ray: Ray) -> Hit {
    var hit: Hit;
    hit.hit = false;

    let ray_step_size_per_unit_axis = vec4<f32>(
        length(ray.direction / ray.direction.x),
        length(ray.direction / ray.direction.y),
        length(ray.direction / ray.direction.z),
        length(ray.direction / ray.direction.w),
    );
    var map_check = vec4<i32>(floor(ray.origin));
    var step: vec4<i32>;
    var ray_lengths_per_axis: vec4<f32>;
    for (var i = 0u; i < 4u; i += 1u) {
        if ray.direction[i] < 0.0 {
            step[i] = -1;
            ray_lengths_per_axis[i] = (ray.origin[i] - f32(map_check[i])) * ray_step_size_per_unit_axis[i];
        } else {
            step[i] = 1;
            ray_lengths_per_axis[i] = (f32(map_check[i] + 1) - ray.origin[i]) * ray_step_size_per_unit_axis[i];
        }
    }

    var distance = 0.0;
    while distance < camera.max_distance {
        // TODO: find out if this is causing a black line through the middle of the screen when looking exactly forward
        // for whatever reason, setting the initial value to 1 seems to stop the issue
        var smallest_length = 0u;
        for (var i = 0u; i < 4u; i += 1u) {
            if step[i] != 0 && ray_lengths_per_axis[i] < ray_lengths_per_axis[smallest_length] {
                smallest_length = i;
            }
        }

        map_check[smallest_length] += step[smallest_length];
        distance = ray_lengths_per_axis[smallest_length];
        ray_lengths_per_axis[smallest_length] += ray_step_size_per_unit_axis[smallest_length];

        if all(map_check >= vec4<i32>(0)) && all(map_check < vec4<i32>(CHUNK_SIZE)) {
            let index = get_block_index(map_check);
            let material = chunk[index].material;
            if material != u32(-1) {
                hit.hit = true;
                hit.block_index = index;
                hit.distance = distance;
                hit.position = ray.origin + ray.direction * distance;
                hit.normal = vec4<f32>(0.0);
                hit.normal[smallest_length] = -f32(step[smallest_length]);
                return hit;
            }
        }
    }

    return hit;
}

fn ray_trace(ray: Ray) -> vec3<f32> {
    let hit = get_intersection(ray);
    if hit.hit {
        return materials.data[chunk[hit.block_index].material].color;
    } else {
        return vec3<f32>(0.0);
    }
}

@compute
@workgroup_size(16, 16)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
) {
    let size = vec2<i32>(textureDimensions(output_texture));
    let coords = vec2<i32>(global_id.xy);
    if coords.x >= size.x || coords.y >= size.y {
        return;
    }

    let aspect = f32(size.x) / f32(size.y);
    let theta = tan(camera.fov / 2.0);
    let uv = vec2<f32>(coords) / vec2<f32>(size);
    let normalized_uv = vec2<f32>(uv.x, 1.0 - uv.y) * 2.0 - 1.0;

    var ray: Ray;
    ray.origin = camera.position;
    ray.direction = normalize(
        camera.right * (normalized_uv.x * aspect * theta) + camera.up * (normalized_uv.y * theta) + camera.forward,
    );

    let color = ray_trace(ray);
    textureStore(output_texture, coords.xy, vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0));
}
