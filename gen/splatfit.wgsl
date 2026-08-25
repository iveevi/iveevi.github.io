struct GaborParams_std430_0
{
    @align(4) x_0 : f32,
    @align(4) y_0 : f32,
    @align(4) log_sx_0 : f32,
    @align(4) log_sy_0 : f32,
    @align(4) theta_0 : f32,
    @align(4) freq_0 : f32,
    @align(4) phase_0 : f32,
    @align(4) r_0 : f32,
    @align(4) g_0 : f32,
    @align(4) b_0 : f32,
};

@binding(1) @group(0) var<storage, read_write> params_0 : array<GaborParams_std430_0>;

@binding(3) @group(0) var<storage, read_write> blended_0 : array<vec4<f32>>;

@binding(0) @group(0) var output_0 : texture_storage_2d<rgba8unorm, write>;

struct EntryPointParams_std140_0
{
    @align(16) shine_0 : vec2<f32>,
};

@binding(5) @group(0) var<uniform> entryPointParams_0 : EntryPointParams_std140_0;
@binding(4) @group(0) var<storage, read> target_0 : array<vec4<f32>>;

struct Moment_std430_0
{
    @align(4) m_0 : f32,
    @align(4) v_0 : f32,
};

struct GaborMoments_std430_0
{
    @align(4) x_1 : Moment_std430_0,
    @align(4) y_1 : Moment_std430_0,
    @align(4) log_sx_1 : Moment_std430_0,
    @align(4) log_sy_1 : Moment_std430_0,
    @align(4) theta_1 : Moment_std430_0,
    @align(4) freq_1 : Moment_std430_0,
    @align(4) phase_1 : Moment_std430_0,
    @align(4) r_1 : Moment_std430_0,
    @align(4) g_1 : Moment_std430_0,
    @align(4) b_1 : Moment_std430_0,
};

@binding(2) @group(0) var<storage, read_write> moments_0 : array<GaborMoments_std430_0>;

struct EntryPointParams_std140_1
{
    @align(16) step_index_0 : u32,
};

@binding(6) @group(0) var<uniform> entryPointParams_1 : EntryPointParams_std140_1;
fn isnan_0( x_2 : f32) -> bool
{
    var _S1 : u32 = (bitcast<u32>((x_2)));
    var _S2 : u32 = (_S1 & (u32(8388607)));
    var _S3 : bool;
    if(((((_S1 >> (u32(23)))) & (u32(255)))) == u32(255))
    {
        _S3 = _S2 != u32(0);
    }
    else
    {
        _S3 = false;
    }
    return _S3;
}

fn isinf_0( x_3 : f32) -> bool
{
    var _S4 : u32 = (bitcast<u32>((x_3)));
    var _S5 : u32 = (_S4 & (u32(8388607)));
    var _S6 : bool;
    if(((((_S4 >> (u32(23)))) & (u32(255)))) == u32(255))
    {
        _S6 = _S5 == u32(0);
    }
    else
    {
        _S6 = false;
    }
    return _S6;
}

fn isfinite_0( x_4 : f32) -> bool
{
    var _S7 : bool;
    if(isinf_0(x_4))
    {
        _S7 = true;
    }
    else
    {
        _S7 = isnan_0(x_4);
    }
    return !_S7;
}

struct Gabor_0
{
     pos_0 : vec2<f32>,
     scale_0 : vec2<f32>,
     sin_t_0 : f32,
     cos_t_0 : f32,
     freq_2 : f32,
     phase_2 : f32,
     color_0 : vec3<f32>,
};

fn load_gabor_0( index_0 : u32) -> Gabor_0
{
    var s_0 : Gabor_0;
    s_0.pos_0 = vec2<f32>(params_0[index_0].x_0, params_0[index_0].y_0);
    s_0.scale_0 = exp(vec2<f32>(params_0[index_0].log_sx_0, params_0[index_0].log_sy_0));
    s_0.sin_t_0 = sin(params_0[index_0].theta_0);
    s_0.cos_t_0 = cos(params_0[index_0].theta_0);
    s_0.freq_2 = params_0[index_0].freq_0;
    s_0.phase_2 = params_0[index_0].phase_0;
    s_0.color_0 = vec3<f32>(params_0[index_0].r_0, params_0[index_0].g_0, params_0[index_0].b_0);
    return s_0;
}

fn kernel_0( s_1 : Gabor_0,  pixel_0 : vec2<f32>,  n_0 : ptr<function, vec2<f32>>,  uv_0 : ptr<function, vec2<f32>>,  envelope_0 : ptr<function, f32>,  sarg_0 : ptr<function, f32>) -> f32
{
    var d_0 : vec2<f32> = pixel_0 - s_1.pos_0;
    var _S8 : f32 = d_0.x;
    var _S9 : f32 = d_0.y;
    var _S10 : vec2<f32> = vec2<f32>(s_1.cos_t_0 * _S8 + s_1.sin_t_0 * _S9, - s_1.sin_t_0 * _S8 + s_1.cos_t_0 * _S9);
    (*uv_0) = _S10;
    var _S11 : vec2<f32> = _S10 / s_1.scale_0;
    (*n_0) = _S11;
    (*envelope_0) = exp(-0.5f * dot(_S11, _S11));
    var arg_0 : f32 = 6.28318548202514648f * s_1.freq_2 * (*uv_0).x + s_1.phase_2;
    (*sarg_0) = sin(arg_0);
    return (*envelope_0) * cos(arg_0);
}

fn random_0( seed_0 : u32) -> f32
{
    var x_5 : u32 = seed_0 * u32(747796405) + u32(2891336453);
    var x_6 : u32 = ((((x_5 >> ((((x_5 >> (u32(28)))) + u32(4))))) ^ (x_5))) * u32(277803737);
    return f32((((x_6 >> (u32(22)))) ^ (x_6))) / 4.294967296e+09f;
}

fn glint_0( index_1 : u32,  phase_3 : f32) -> f32
{
    var _S12 : f32 = phase_3 - random_0(index_1);
    var _S13 : f32 = _S12 - floor(_S12);
    var _S14 : f32 = min(_S13, 1.0f - _S13);
    return exp(-9.0e+04f * _S14 * _S14);
}

fn RWStructuredBuffer_getCount_0() -> i32
{
    var _S15 : vec2<u32> = vec2<u32>(arrayLength(&params_0), 40);
    return i32(_S15.x);
}

@compute
@workgroup_size(8, 8, 1)
fn gather(@builtin(global_invocation_id) thread_id_0 : vec3<u32>)
{
    var _S16 : u32 = thread_id_0.x;
    var _S17 : bool;
    if(_S16 >= u32(128))
    {
        _S17 = true;
    }
    else
    {
        _S17 = (thread_id_0.y) >= u32(128);
    }
    if(_S17)
    {
        return;
    }
    var _S18 : vec2<u32> = thread_id_0.xy;
    var _S19 : vec2<f32> = vec2<f32>(_S18) + vec2<f32>(0.5f);
    var i_0 : u32 = u32(0);
    var composed_0 : vec3<f32> = vec3<f32>(0.5f);
    var lit_0 : vec3<f32> = vec3<f32>(0.5f);
    for(;;)
    {
        var _S20 : i32 = RWStructuredBuffer_getCount_0();
        if(i_0 < u32(_S20))
        {
        }
        else
        {
            break;
        }
        var delta_0 : vec2<f32> = _S19 - vec2<f32>(params_0[i_0].x_0, params_0[i_0].y_0);
        var reach_0 : f32 = 3.0f * exp(max(params_0[i_0].log_sx_0, params_0[i_0].log_sy_0));
        if(!((dot(delta_0, delta_0)) <= (reach_0 * reach_0)))
        {
            i_0 = i_0 + u32(1);
            continue;
        }
        var _S21 : Gabor_0 = load_gabor_0(i_0);
        var n_1 : vec2<f32>;
        var uv_1 : vec2<f32>;
        var envelope_1 : f32;
        var sarg_1 : f32;
        var g_2 : f32 = kernel_0(_S21, _S19, &(n_1), &(uv_1), &(envelope_1), &(sarg_1));
        var _S22 : vec3<f32> = vec3<f32>(g_2) * _S21.color_0;
        var _S23 : f32 = abs(g_2) / max(envelope_1, 9.999999960041972e-13f);
        var lit_1 : vec3<f32> = lit_0 + (_S22 + vec3<f32>((entryPointParams_0.shine_0.y * glint_0(i_0, entryPointParams_0.shine_0.x) * (envelope_1 * envelope_1 * _S23 * _S23 * _S23))));
        composed_0 = composed_0 + _S22;
        lit_0 = lit_1;
        i_0 = i_0 + u32(1);
    }
    blended_0[thread_id_0.y * u32(128) + _S16] = vec4<f32>(0.0f, composed_0);
    textureStore((output_0), (_S18), (vec4<f32>(clamp(lit_0, vec3<f32>(0.0f), vec3<f32>(1.0f)), 1.0f)));
    return;
}

struct Moment_0
{
     m_0 : f32,
     v_0 : f32,
};

fn adam_update_0( moment_0 : ptr<function, Moment_0>,  g_3 : f32,  lr_0 : f32,  correction_0 : f32) -> f32
{
    var _S24 : f32 = 0.89999997615814209f * (*moment_0).m_0 + 0.10000000149011612f * g_3;
    (*moment_0).m_0 = _S24;
    var _S25 : f32 = 0.99900001287460327f * (*moment_0).v_0 + 0.00100000004749745f * g_3 * g_3;
    (*moment_0).v_0 = _S25;
    return lr_0 * correction_0 * _S24 / (sqrt(_S25) + 9.99999993922529029e-09f);
}

fn bounded_0( value_0 : f32,  low_0 : f32,  high_0 : f32) -> f32
{
    var _S26 : f32;
    if(isfinite_0(value_0))
    {
        _S26 = clamp(value_0, low_0, high_0);
    }
    else
    {
        _S26 = 0.5f * (low_0 + high_0);
    }
    return _S26;
}

struct GaborParams_0
{
     x_0 : f32,
     y_0 : f32,
     log_sx_0 : f32,
     log_sy_0 : f32,
     theta_0 : f32,
     freq_0 : f32,
     phase_0 : f32,
     r_0 : f32,
     g_0 : f32,
     b_0 : f32,
};

fn sanitize_0( p_0 : GaborParams_0) -> GaborParams_0
{
    var _S27 : GaborParams_0 = p_0;
    _S27.x_0 = bounded_0(_S27.x_0, -128.0f, 256.0f);
    _S27.y_0 = bounded_0(_S27.y_0, -128.0f, 256.0f);
    _S27.log_sx_0 = bounded_0(_S27.log_sx_0, -1.0f, 2.5f);
    _S27.log_sy_0 = bounded_0(_S27.log_sy_0, -1.0f, 2.5f);
    _S27.theta_0 = bounded_0(_S27.theta_0, -6.28318548202514648f, 6.28318548202514648f);
    _S27.freq_0 = bounded_0(_S27.freq_0, -0.5f, 0.5f);
    _S27.phase_0 = bounded_0(_S27.phase_0, -6.28318548202514648f, 6.28318548202514648f);
    _S27.r_0 = bounded_0(_S27.r_0, -2.0f, 2.0f);
    _S27.g_0 = bounded_0(_S27.g_0, -2.0f, 2.0f);
    _S27.b_0 = bounded_0(_S27.b_0, -2.0f, 2.0f);
    return _S27;
}

struct GaborMoments_0
{
     x_1 : Moment_0,
     y_1 : Moment_0,
     log_sx_1 : Moment_0,
     log_sy_1 : Moment_0,
     theta_1 : Moment_0,
     freq_1 : Moment_0,
     phase_1 : Moment_0,
     r_1 : Moment_0,
     g_1 : Moment_0,
     b_1 : Moment_0,
};

@compute
@workgroup_size(64, 1, 1)
fn backward(@builtin(global_invocation_id) thread_id_1 : vec3<u32>)
{
    var index_2 : u32 = thread_id_1.x;
    var _S28 : i32 = RWStructuredBuffer_getCount_0();
    if(index_2 >= u32(_S28))
    {
        return;
    }
    var _S29 : Gabor_0 = load_gabor_0(index_2);
    var _S30 : vec2<f32> = vec2<f32>(0.0f);
    var grad_pos_0 : vec2<f32> = _S30;
    var grad_scale_0 : vec2<f32> = _S30;
    var _S31 : vec3<f32> = vec3<f32>(0.0f);
    var _S32 : f32 = _S29.scale_0.x;
    var _S33 : f32 = _S29.scale_0.y;
    var extent_0 : f32 = 3.0f * max(_S32, _S33);
    var _S34 : f32 = _S29.pos_0.x;
    var _S35 : u32 = u32(clamp(_S34 - extent_0, 0.0f, 128.0f));
    var _S36 : u32 = u32(clamp(_S34 + extent_0 + 1.0f, 0.0f, 128.0f));
    var _S37 : f32 = _S29.pos_0.y;
    var _S38 : u32 = u32(clamp(_S37 + extent_0 + 1.0f, 0.0f, 128.0f));
    var py_0 : u32 = u32(clamp(_S37 - extent_0, 0.0f, 128.0f));
    var grad_color_0 : vec3<f32> = _S31;
    var grad_theta_0 : f32 = 0.0f;
    var grad_freq_0 : f32 = 0.0f;
    var grad_phase_0 : f32 = 0.0f;
    for(;;)
    {
        if(py_0 < _S38)
        {
        }
        else
        {
            break;
        }
        var px_0 : u32 = _S35;
        for(;;)
        {
            if(px_0 < _S36)
            {
            }
            else
            {
                break;
            }
            var n_2 : vec2<f32>;
            var uv_2 : vec2<f32>;
            var envelope_2 : f32;
            var sarg_2 : f32;
            var g_4 : f32 = kernel_0(_S29, vec2<f32>(f32(px_0), f32(py_0)) + vec2<f32>(0.5f), &(n_2), &(uv_2), &(envelope_2), &(sarg_2));
            var idx_0 : u32 = py_0 * u32(128) + px_0;
            var residual_0 : vec3<f32> = blended_0[idx_0].yzw - target_0[idx_0].xyz;
            var grad_color_1 : vec3<f32> = grad_color_0 + vec3<f32>(2.0f) * residual_0 * vec3<f32>(g_4);
            var coupling_0 : f32 = 2.0f * dot(residual_0, _S29.color_0);
            var denv_dy_0 : f32 = envelope_2 * (n_2.x * _S29.sin_t_0 / _S32 + n_2.y * _S29.cos_t_0 / _S33);
            var denv_dtheta_0 : f32 = - envelope_2 * (n_2.x * uv_2.y / _S32 - n_2.y * uv_2.x / _S33);
            var cosine_0 : f32 = g_4 / max(envelope_2, 9.999999960041972e-13f);
            var wave_0 : f32 = 6.28318548202514648f * _S29.freq_2;
            grad_pos_0[i32(0)] = grad_pos_0[i32(0)] + coupling_0 * (cosine_0 * (envelope_2 * (n_2.x * _S29.cos_t_0 / _S32 - n_2.y * _S29.sin_t_0 / _S33)) + envelope_2 * sarg_2 * wave_0 * _S29.cos_t_0);
            grad_pos_0[i32(1)] = grad_pos_0[i32(1)] + coupling_0 * (cosine_0 * denv_dy_0 + envelope_2 * sarg_2 * wave_0 * _S29.sin_t_0);
            var _S39 : f32 = coupling_0 * cosine_0;
            grad_scale_0[i32(0)] = grad_scale_0[i32(0)] + _S39 * envelope_2 * n_2.x * n_2.x;
            grad_scale_0[i32(1)] = grad_scale_0[i32(1)] + _S39 * envelope_2 * n_2.y * n_2.y;
            var grad_theta_1 : f32 = grad_theta_0 + coupling_0 * (cosine_0 * denv_dtheta_0 - envelope_2 * sarg_2 * wave_0 * uv_2.y);
            var grad_freq_1 : f32 = grad_freq_0 + coupling_0 * envelope_2 * - sarg_2 * 6.28318548202514648f * uv_2.x;
            var grad_phase_1 : f32 = grad_phase_0 + coupling_0 * envelope_2 * - sarg_2;
            px_0 = px_0 + u32(1);
            grad_color_0 = grad_color_1;
            grad_theta_0 = grad_theta_1;
            grad_freq_0 = grad_freq_1;
            grad_phase_0 = grad_phase_1;
        }
        py_0 = py_0 + u32(1);
    }
    var t_0 : f32 = f32(entryPointParams_1.step_index_0);
    var correction_1 : f32 = sqrt(1.0f - pow(0.99900001287460327f, t_0)) / (1.0f - pow(0.89999997615814209f, t_0));
    var p_1 : GaborParams_0;
    p_1.x_0 = params_0[index_2].x_0;
    p_1.y_0 = params_0[index_2].y_0;
    p_1.log_sx_0 = params_0[index_2].log_sx_0;
    p_1.log_sy_0 = params_0[index_2].log_sy_0;
    p_1.theta_0 = params_0[index_2].theta_0;
    p_1.freq_0 = params_0[index_2].freq_0;
    p_1.phase_0 = params_0[index_2].phase_0;
    p_1.r_0 = params_0[index_2].r_0;
    p_1.g_0 = params_0[index_2].g_0;
    p_1.b_0 = params_0[index_2].b_0;
    var _S40 : Moment_0 = Moment_0( moments_0[index_2].x_1.m_0, moments_0[index_2].x_1.v_0 );
    var _S41 : Moment_0 = Moment_0( moments_0[index_2].y_1.m_0, moments_0[index_2].y_1.v_0 );
    var _S42 : Moment_0 = Moment_0( moments_0[index_2].log_sx_1.m_0, moments_0[index_2].log_sx_1.v_0 );
    var _S43 : Moment_0 = Moment_0( moments_0[index_2].log_sy_1.m_0, moments_0[index_2].log_sy_1.v_0 );
    var _S44 : Moment_0 = Moment_0( moments_0[index_2].theta_1.m_0, moments_0[index_2].theta_1.v_0 );
    var _S45 : Moment_0 = Moment_0( moments_0[index_2].freq_1.m_0, moments_0[index_2].freq_1.v_0 );
    var _S46 : Moment_0 = Moment_0( moments_0[index_2].phase_1.m_0, moments_0[index_2].phase_1.v_0 );
    var _S47 : Moment_0 = Moment_0( moments_0[index_2].r_1.m_0, moments_0[index_2].r_1.v_0 );
    var _S48 : Moment_0 = Moment_0( moments_0[index_2].g_1.m_0, moments_0[index_2].g_1.v_0 );
    var _S49 : Moment_0 = Moment_0( moments_0[index_2].b_1.m_0, moments_0[index_2].b_1.v_0 );
    var state_0 : GaborMoments_0;
    state_0.x_1 = _S40;
    state_0.y_1 = _S41;
    state_0.log_sx_1 = _S42;
    state_0.log_sy_1 = _S43;
    state_0.theta_1 = _S44;
    state_0.freq_1 = _S45;
    state_0.phase_1 = _S46;
    state_0.r_1 = _S47;
    state_0.g_1 = _S48;
    state_0.b_1 = _S49;
    var _S50 : f32 = grad_pos_0.x;
    var _S51 : Moment_0 = state_0.x_1;
    var _S52 : f32 = adam_update_0(&(_S51), _S50, 0.30000001192092896f, correction_1);
    state_0.x_1 = _S51;
    p_1.x_0 = p_1.x_0 - _S52;
    var _S53 : f32 = grad_pos_0.y;
    var _S54 : Moment_0 = state_0.y_1;
    var _S55 : f32 = adam_update_0(&(_S54), _S53, 0.30000001192092896f, correction_1);
    state_0.y_1 = _S54;
    p_1.y_0 = p_1.y_0 - _S55;
    var _S56 : f32 = grad_scale_0.x;
    var _S57 : Moment_0 = state_0.log_sx_1;
    var _S58 : f32 = adam_update_0(&(_S57), _S56, 0.01499999966472387f, correction_1);
    state_0.log_sx_1 = _S57;
    p_1.log_sx_0 = p_1.log_sx_0 - _S58;
    var _S59 : f32 = grad_scale_0.y;
    var _S60 : Moment_0 = state_0.log_sy_1;
    var _S61 : f32 = adam_update_0(&(_S60), _S59, 0.01499999966472387f, correction_1);
    state_0.log_sy_1 = _S60;
    p_1.log_sy_0 = p_1.log_sy_0 - _S61;
    var _S62 : Moment_0 = state_0.theta_1;
    var _S63 : f32 = adam_update_0(&(_S62), grad_theta_0, 0.03500000014901161f, correction_1);
    state_0.theta_1 = _S62;
    p_1.theta_0 = p_1.theta_0 - _S63;
    var _S64 : Moment_0 = state_0.freq_1;
    var _S65 : f32 = adam_update_0(&(_S64), grad_freq_0, 0.00350000010803342f, correction_1);
    state_0.freq_1 = _S64;
    p_1.freq_0 = p_1.freq_0 - _S65;
    var _S66 : Moment_0 = state_0.phase_1;
    var _S67 : f32 = adam_update_0(&(_S66), grad_phase_0, 0.03500000014901161f, correction_1);
    state_0.phase_1 = _S66;
    p_1.phase_0 = p_1.phase_0 - _S67;
    var _S68 : f32 = grad_color_0.x;
    var _S69 : Moment_0 = state_0.r_1;
    var _S70 : f32 = adam_update_0(&(_S69), _S68, 0.01499999966472387f, correction_1);
    state_0.r_1 = _S69;
    p_1.r_0 = p_1.r_0 - _S70;
    var _S71 : f32 = grad_color_0.y;
    var _S72 : Moment_0 = state_0.g_1;
    var _S73 : f32 = adam_update_0(&(_S72), _S71, 0.01499999966472387f, correction_1);
    state_0.g_1 = _S72;
    p_1.g_0 = p_1.g_0 - _S73;
    var _S74 : f32 = grad_color_0.z;
    var _S75 : Moment_0 = state_0.b_1;
    var _S76 : f32 = adam_update_0(&(_S75), _S74, 0.01499999966472387f, correction_1);
    state_0.b_1 = _S75;
    p_1.b_0 = p_1.b_0 - _S76;
    var _S77 : GaborParams_0 = sanitize_0(p_1);
    params_0[index_2].x_0 = _S77.x_0;
    params_0[index_2].y_0 = _S77.y_0;
    params_0[index_2].log_sx_0 = _S77.log_sx_0;
    params_0[index_2].log_sy_0 = _S77.log_sy_0;
    params_0[index_2].theta_0 = _S77.theta_0;
    params_0[index_2].freq_0 = _S77.freq_0;
    params_0[index_2].phase_0 = _S77.phase_0;
    params_0[index_2].r_0 = _S77.r_0;
    params_0[index_2].g_0 = _S77.g_0;
    params_0[index_2].b_0 = _S77.b_0;
    moments_0[index_2].x_1.m_0 = state_0.x_1.m_0;
    moments_0[index_2].x_1.v_0 = state_0.x_1.v_0;
    moments_0[index_2].y_1.m_0 = state_0.y_1.m_0;
    moments_0[index_2].y_1.v_0 = state_0.y_1.v_0;
    moments_0[index_2].log_sx_1.m_0 = state_0.log_sx_1.m_0;
    moments_0[index_2].log_sx_1.v_0 = state_0.log_sx_1.v_0;
    moments_0[index_2].log_sy_1.m_0 = state_0.log_sy_1.m_0;
    moments_0[index_2].log_sy_1.v_0 = state_0.log_sy_1.v_0;
    moments_0[index_2].theta_1.m_0 = state_0.theta_1.m_0;
    moments_0[index_2].theta_1.v_0 = state_0.theta_1.v_0;
    moments_0[index_2].freq_1.m_0 = state_0.freq_1.m_0;
    moments_0[index_2].freq_1.v_0 = state_0.freq_1.v_0;
    moments_0[index_2].phase_1.m_0 = state_0.phase_1.m_0;
    moments_0[index_2].phase_1.v_0 = state_0.phase_1.v_0;
    moments_0[index_2].r_1.m_0 = state_0.r_1.m_0;
    moments_0[index_2].r_1.v_0 = state_0.r_1.v_0;
    moments_0[index_2].g_1.m_0 = state_0.g_1.m_0;
    moments_0[index_2].g_1.v_0 = state_0.g_1.v_0;
    moments_0[index_2].b_1.m_0 = state_0.b_1.m_0;
    moments_0[index_2].b_1.v_0 = state_0.b_1.v_0;
    return;
}

