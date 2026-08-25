[format("rgba8")]
WTexture2D <float4> output;

public struct GaborParams {
	public float x;
	public float y;
	public float log_sx;
	public float log_sy;
	public float theta;
	public float freq;
	public float phase;
	public float r;
	public float g;
	public float b;
};

public struct Moment {
	public float m;
	public float v;
};

public struct GaborMoments {
	public Moment x;
	public Moment y;
	public Moment log_sx;
	public Moment log_sy;
	public Moment theta;
	public Moment freq;
	public Moment phase;
	public Moment r;
	public Moment g;
	public Moment b;
};

RWStructuredBuffer <GaborParams> params;
RWStructuredBuffer <GaborMoments> moments;
RWStructuredBuffer <float4> blended;
StructuredBuffer <float4> target;

static const uint SIZE = 128;
static const float3 BASE = float3(0.5);
static const float TWO_PI = 6.2831853;
static const float POSITION_LR = 0.3;
static const float SCALE_LR = 0.015;
static const float THETA_LR = 0.035;
static const float FREQ_LR = 0.0035;
static const float PHASE_LR = 0.035;
static const float COLOR_LR = 0.015;
static const float MIN_LOG = -1.0;
static const float MAX_LOG = 2.5;

struct Gabor {
	float2 pos;
	float2 scale;
	float sin_t;
	float cos_t;
	float freq;
	float phase;
	float3 color;
};

Gabor load_gabor(uint index)
{
	let p = params[index];
	Gabor s;
	s.pos = float2(p.x, p.y);
	s.scale = exp(float2(p.log_sx, p.log_sy));
	s.sin_t = sin(p.theta);
	s.cos_t = cos(p.theta);
	s.freq = p.freq;
	s.phase = p.phase;
	s.color = float3(p.r, p.g, p.b);
	return s;
}

float kernel(Gabor s, float2 pixel, out float2 n, out float2 uv, out float envelope, out float sarg)
{
	float2 d = pixel - s.pos;
	uv = float2(s.cos_t * d.x + s.sin_t * d.y, -s.sin_t * d.x + s.cos_t * d.y);
	n = uv / s.scale;
	envelope = exp(-0.5 * dot(n, n));
	float arg = TWO_PI * s.freq * uv.x + s.phase;
	sarg = sin(arg);
	return envelope * cos(arg);
}

float random(uint seed)
{
	uint x = seed * 747796405u + 2891336453u;
	x = ((x >> ((x >> 28) + 4u)) ^ x) * 277803737u;
	x = (x >> 22u) ^ x;
	return float(x) / 4294967295.0;
}

float glint(uint index, float phase)
{
	let offset = phase - random(index);
	let cycle = offset - floor(offset);
	let distance = min(cycle, 1.0 - cycle);
	return exp(-90000.0 * distance * distance);
}

[shader("compute")]
[numthreads(8, 8, 1)]
void gather(uint3 thread_id: SV_DispatchThreadID, uniform float2 shine)
{
	if (thread_id.x >= SIZE || thread_id.y >= SIZE)
		return;

	float2 pixel = float2(thread_id.xy) + 0.5;
	float3 composed = BASE;
	float3 lit = BASE;

	for (uint i = 0; i < params.getCount(); i++) {
		let p = params[i];
		float2 delta = pixel - float2(p.x, p.y);
		float reach = 3.0 * exp(max(p.log_sx, p.log_sy));
		if (!(dot(delta, delta) <= reach * reach))
			continue;

		let s = load_gabor(i);
		float2 n;
		float2 uv;
		float envelope;
		float sarg;
		float g = kernel(s, pixel, n, uv, envelope, sarg);
		composed += g * s.color;
		let fringe = abs(g) / max(envelope, 1e-12);
		let core = envelope * envelope * fringe * fringe * fringe;
		lit += g * s.color + shine.y * glint(i, shine.x) * core;
	}

	uint idx = thread_id.y * SIZE + thread_id.x;
	blended[idx] = float4(0.0, composed);
	output.Store(thread_id.xy, float4(clamp(lit, 0.0, 1.0), 1.0));
}

float bounded(float value, float low, float high)
{
	return isfinite(value) ? clamp(value, low, high) : 0.5 * (low + high);
}

GaborParams sanitize(GaborParams p)
{
	let edge = float(SIZE);
	p.x = bounded(p.x, -edge, 2.0 * edge);
	p.y = bounded(p.y, -edge, 2.0 * edge);
	p.log_sx = bounded(p.log_sx, MIN_LOG, MAX_LOG);
	p.log_sy = bounded(p.log_sy, MIN_LOG, MAX_LOG);
	p.theta = bounded(p.theta, -TWO_PI, TWO_PI);
	p.freq = bounded(p.freq, -0.5, 0.5);
	p.phase = bounded(p.phase, -TWO_PI, TWO_PI);
	p.r = bounded(p.r, -2.0, 2.0);
	p.g = bounded(p.g, -2.0, 2.0);
	p.b = bounded(p.b, -2.0, 2.0);
	return p;
}

float adam_update(inout Moment moment, float g, float lr, float correction)
{
	moment.m = 0.9 * moment.m + 0.1 * g;
	moment.v = 0.999 * moment.v + 0.001 * g * g;
	return lr * correction * moment.m / (sqrt(moment.v) + 1e-8);
}

[shader("compute")]
[numthreads(64, 1, 1)]
void backward(uint3 thread_id: SV_DispatchThreadID, uniform uint step_index)
{
	uint index = thread_id.x;
	if (index >= params.getCount())
		return;

	let s = load_gabor(index);
	float2 grad_pos = float2(0.0);
	float2 grad_scale = float2(0.0);
	float grad_theta = 0.0;
	float grad_freq = 0.0;
	float grad_phase = 0.0;
	float3 grad_color = float3(0.0);

	float extent = 3.0 * max(s.scale.x, s.scale.y);
	uint x0 = uint(clamp(s.pos.x - extent, 0.0, float(SIZE)));
	uint x1 = uint(clamp(s.pos.x + extent + 1.0, 0.0, float(SIZE)));
	uint y0 = uint(clamp(s.pos.y - extent, 0.0, float(SIZE)));
	uint y1 = uint(clamp(s.pos.y + extent + 1.0, 0.0, float(SIZE)));

	for (uint py = y0; py < y1; py++) {
		for (uint px = x0; px < x1; px++) {
			float2 pixel = float2(float(px), float(py)) + 0.5;
			float2 n;
			float2 uv;
			float envelope;
			float sarg;
			float g = kernel(s, pixel, n, uv, envelope, sarg);
			uint idx = py * SIZE + px;
			float3 residual = blended[idx].yzw - target[idx].xyz;

			grad_color += 2.0 * residual * g;

			float coupling = 2.0 * dot(residual, s.color);
			float denv_dx = envelope * (n.x * s.cos_t / s.scale.x - n.y * s.sin_t / s.scale.y);
			float denv_dy = envelope * (n.x * s.sin_t / s.scale.x + n.y * s.cos_t / s.scale.y);
			float denv_dtheta = -envelope * (n.x * uv.y / s.scale.x - n.y * uv.x / s.scale.y);
			float cosine = g / max(envelope, 1e-12);
			float wave = TWO_PI * s.freq;

			grad_pos.x += coupling * (cosine * denv_dx + envelope * sarg * wave * s.cos_t);
			grad_pos.y += coupling * (cosine * denv_dy + envelope * sarg * wave * s.sin_t);
			grad_scale.x += coupling * cosine * envelope * n.x * n.x;
			grad_scale.y += coupling * cosine * envelope * n.y * n.y;
			grad_theta += coupling * (cosine * denv_dtheta - envelope * sarg * wave * uv.y);
			grad_freq += coupling * envelope * (-sarg) * TWO_PI * uv.x;
			grad_phase += coupling * envelope * (-sarg);
		}
	}

	float t = float(step_index);
	float correction = sqrt(1.0 - pow(0.999, t)) / (1.0 - pow(0.9, t));

	GaborParams p = params[index];
	GaborMoments state = moments[index];

	p.x -= adam_update(state.x, grad_pos.x, POSITION_LR, correction);
	p.y -= adam_update(state.y, grad_pos.y, POSITION_LR, correction);
	p.log_sx -= adam_update(state.log_sx, grad_scale.x, SCALE_LR, correction);
	p.log_sy -= adam_update(state.log_sy, grad_scale.y, SCALE_LR, correction);
	p.theta -= adam_update(state.theta, grad_theta, THETA_LR, correction);
	p.freq -= adam_update(state.freq, grad_freq, FREQ_LR, correction);
	p.phase -= adam_update(state.phase, grad_phase, PHASE_LR, correction);
	p.r -= adam_update(state.r, grad_color.x, COLOR_LR, correction);
	p.g -= adam_update(state.g, grad_color.y, COLOR_LR, correction);
	p.b -= adam_update(state.b, grad_color.z, COLOR_LR, correction);

	params[index] = sanitize(p);
	moments[index] = state;
}
