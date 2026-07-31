struct Params {
	float width;
	float height;
	float time;
	float blend;
	float tint;
	float exposure;
};

struct Node {
	float x;
	float y;
	float amplitude;
};

ConstantBuffer <Params> params;
StructuredBuffer <Node> nodes;
StructuredBuffer <uint> indices;
Texture2D <float4> previous;
Texture2D <float4> current;
SamplerState linear_sampler;

struct Varying {
	float4 position : SV_Position;
	nointerpolation float3 color : COLOR;
};

float random(uint seed)
{
	uint x = seed * 747796405u + 2891336453u;
	x = ((x >> ((x >> 28) + 4u)) ^ x) * 277803737u;
	x = (x >> 22u) ^ x;
	return float(x) / 4294967295.0;
}

float2 position(uint node)
{
	let n = nodes[node];
	return float2(n.x, n.y);
}

float3 shade(float2 centroid, uint facet)
{
	let uv = centroid / float2(params.width, params.height);
	let before = previous.SampleLevel(linear_sampler, uv, 0.0).rgb;
	let after = current.SampleLevel(linear_sampler, uv, 0.0).rgb;
	let color = params.exposure * lerp(before, after, params.blend);
	let tint = float3(random(facet), random(facet + 31u), random(facet + 61u)) - 0.5;
	return saturate(color + params.tint * tint);
}

float4 clip_of(float2 p)
{
	return float4(2.0 * p.x / params.width - 1.0, 1.0 - 2.0 * p.y / params.height, 0.0, 1.0);
}

[shader("vertex")]
Varying vertex_main(uint index: SV_VertexID)
{
	let facet = 3 * (index / 3);
	Varying output;
	output.position = clip_of(position(indices[index]));
	output.color = float3(0.0);
	if (index != facet)
		return output;

	let a = position(indices[facet]);
	let b = position(indices[facet + 1]);
	let c = position(indices[facet + 2]);
	output.color = shade((a + b + c) / 3.0, facet);
	return output;
}

[shader("fragment")]
float4 fragment_main(Varying varying) : SV_Target
{
	return float4(varying.color, 1.0);
}
