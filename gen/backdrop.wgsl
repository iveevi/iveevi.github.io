@binding(2) @group(0) var<storage, read> indices_0 : array<u32>;

struct Node_std430_0
{
    @align(4) x_0 : f32,
    @align(4) y_0 : f32,
    @align(4) amplitude_0 : f32,
};

@binding(1) @group(0) var<storage, read> nodes_0 : array<Node_std430_0>;

struct Params_std140_0
{
    @align(16) width_0 : f32,
    @align(4) height_0 : f32,
    @align(8) time_0 : f32,
    @align(4) blend_0 : f32,
    @align(16) tint_0 : f32,
    @align(4) exposure_0 : f32,
};

@binding(0) @group(0) var<uniform> params_0 : Params_std140_0;
@binding(3) @group(0) var previous_0 : texture_2d<f32>;

@binding(5) @group(0) var linear_sampler_0 : sampler;

@binding(4) @group(0) var current_0 : texture_2d<f32>;

fn position_0( node_0 : u32) -> vec2<f32>
{
    var _S1 : Node_std430_0 = nodes_0[node_0];
    return vec2<f32>(_S1.x_0, _S1.y_0);
}

fn clip_of_0( p_0 : vec2<f32>) -> vec4<f32>
{
    return vec4<f32>(2.0f * p_0.x / params_0.width_0 - 1.0f, 1.0f - 2.0f * p_0.y / params_0.height_0, 0.0f, 1.0f);
}

fn random_0( seed_0 : u32) -> f32
{
    var x_1 : u32 = seed_0 * u32(747796405) + u32(2891336453);
    var x_2 : u32 = ((((x_1 >> ((((x_1 >> (u32(28)))) + u32(4))))) ^ (x_1))) * u32(277803737);
    return f32((((x_2 >> (u32(22)))) ^ (x_2))) / 4.294967296e+09f;
}

fn shade_0( centroid_0 : vec2<f32>,  facet_0 : u32) -> vec3<f32>
{
    var _S2 : vec2<f32> = centroid_0 / vec2<f32>(params_0.width_0, params_0.height_0);
    var _S3 : f32 = 3.14159274101257324f / (2.5f + 6.0f * random_0(facet_0 + u32(131)));
    return saturate(vec3<f32>(params_0.exposure_0) * mix((textureSampleLevel((previous_0), (linear_sampler_0), (_S2), (0.0f))).xyz, (textureSampleLevel((current_0), (linear_sampler_0), (_S2), (0.0f))).xyz, vec3<f32>(params_0.blend_0)) + vec3<f32>((0.60000002384185791f * params_0.tint_0 * random_0(facet_0))) * cos(vec3<f32>(6.28318548202514648f) * (vec3<f32>((random_0(facet_0 + u32(31)) + 0.40000000596046448f / _S3 * asin(sin(_S3 * params_0.time_0 + 6.28318548202514648f * random_0(facet_0 + u32(173)))))) + vec3<f32>(0.0f, 0.3333333432674408f, 0.66666668653488159f))));
}

struct Varying_0
{
    @builtin(position) position_1 : vec4<f32>,
    @interpolate(flat) @location(0) color_0 : vec3<f32>,
};

@vertex
fn vertex_main(@builtin(vertex_index) index_0 : u32) -> Varying_0
{
    var _S4 : u32 = u32(3) * (index_0 / u32(3));
    var output_0 : Varying_0;
    output_0.position_1 = clip_of_0(position_0(indices_0[index_0]));
    output_0.color_0 = vec3<f32>(0.0f);
    if(index_0 != _S4)
    {
        return output_0;
    }
    output_0.color_0 = shade_0((position_0(indices_0[_S4]) + position_0(indices_0[_S4 + u32(1)]) + position_0(indices_0[_S4 + u32(2)])) / vec2<f32>(3.0f), _S4);
    return output_0;
}

struct pixelOutput_0
{
    @location(0) output_1 : vec4<f32>,
};

struct pixelInput_0
{
    @interpolate(flat) @location(0) color_1 : vec3<f32>,
};

@fragment
fn fragment_main( _S5 : pixelInput_0, @builtin(position) position_2 : vec4<f32>) -> pixelOutput_0
{
    var _S6 : pixelOutput_0 = pixelOutput_0( vec4<f32>(_S5.color_1, 1.0f) );
    return _S6;
}

