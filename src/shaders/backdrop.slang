static const float SPIN = 0.4;

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

	// constant-magnitude chroma per facet, but its hue drifts -- each facet at
	// its own rate, nudged by a slow travelling field, so the mesh disperses
	let magnitude = random(facet);

	// every facet spins its hue at the same speed, but each reverses on its own
	// schedule: asin(sin(.)) is a triangle wave, so its slope is a clean +/-1
	// and the flips land every `interval` seconds
	let interval = 2.5 + 6.0 * random(facet + 131u);
	let omega = 3.14159265 / interval;
	let start = 6.2831853 * random(facet + 173u);
	let hue = random(facet + 31u) + (SPIN / omega) * asin(sin(omega * params.time + start));
	let chroma = cos(6.2831853 * (hue + float3(0.0, 1.0 / 3.0, 2.0 / 3.0)));

	return saturate(color + 0.6 * params.tint * magnitude * chroma);
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
