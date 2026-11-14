//! WebGL2 painter. The document model never touches this module.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use glow::HasContext;

use crate::scene::{DrawFrame, UploadedMesh};

const MESH_VS: &str = r#"#version 300 es
precision highp float;
layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec3 a_normal;
uniform mat4 u_vp;
uniform mat4 u_model;
out vec3 v_n;
out float v_y;
void main() {
    vec4 world = u_model * vec4(a_pos, 1.0);
    gl_Position = u_vp * world;
    v_n = mat3(u_model) * a_normal;
    v_y = world.y;
}
"#;

const MESH_FS: &str = r#"#version 300 es
precision mediump float;
in vec3 v_n;
in float v_y;
uniform vec3 u_color;
uniform float u_selected;
uniform float u_alpha;
uniform float u_clip_y;
out vec4 frag;
void main() {
    if (v_y > u_clip_y) discard;
    vec3 n = normalize(v_n);
    float key = clamp(dot(n, normalize(vec3(0.35, 0.92, 0.22))), 0.0, 1.0);
    float fill = clamp(dot(n, normalize(vec3(-0.45, 0.15, -0.55))), 0.0, 1.0);
    vec3 color = u_color * (0.34 + 0.58 * key + 0.16 * fill);
    color = mix(color, vec3(0.55, 0.86, 1.0), u_selected * 0.42);
    frag = vec4(color, u_alpha);
}
"#;

const LINE_VS: &str = r#"#version 300 es
precision highp float;
layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec3 a_color;
uniform mat4 u_vp;
out vec3 v_color;
void main() {
    gl_Position = u_vp * vec4(a_pos, 1.0);
    v_color = a_color;
}
"#;

const LINE_FS: &str = r#"#version 300 es
precision mediump float;
in vec3 v_color;
out vec4 frag;
void main() { frag = vec4(v_color, 1.0); }
"#;

struct GpuMesh {
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    nbo: glow::Buffer,
    ebo: glow::Buffer,
    count: i32,
    revision: u64,
}

struct GpuState {
    mesh_program: glow::Program,
    line_program: glow::Program,
    grid_vao: glow::VertexArray,
    grid_vbo: glow::Buffer,
    grid_count: i32,
    meshes: HashMap<u64, GpuMesh>,
    grid_ready: bool,
}

pub struct ViewportPainter {
    state: Arc<Mutex<Option<GpuState>>>,
    error: Arc<Mutex<Option<String>>>,
}

impl ViewportPainter {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(None)),
            error: Arc::new(Mutex::new(None)),
        }
    }

    pub fn error(&self) -> Option<String> {
        self.error.lock().ok().and_then(|e| e.clone())
    }

    pub fn callback(&self, frame: DrawFrame) -> Arc<egui_glow::CallbackFn> {
        let state = Arc::clone(&self.state);
        let error = Arc::clone(&self.error);
        Arc::new(egui_glow::CallbackFn::new(move |info, painter| {
            let gl = painter.gl();
            if let Err(err) = paint(gl, &state, &frame, &info) {
                if let Ok(mut slot) = error.lock() {
                    *slot = Some(err);
                }
            }
        }))
    }
}

fn paint(
    gl: &glow::Context,
    state: &Mutex<Option<GpuState>>,
    frame: &DrawFrame,
    info: &egui::PaintCallbackInfo,
) -> Result<(), String> {
    let mut guard = state.lock().map_err(|_| "painter lock".to_string())?;
    if guard.is_none() {
        *guard = Some(GpuState::new(gl)?);
    }
    let gpu = guard.as_mut().unwrap();
    if !gpu.grid_ready {
        gpu.upload_grid(gl, &frame.grid.vertices);
        gpu.grid_ready = true;
    }
    let live: HashMap<u64, ()> = frame.objects.iter().map(|o| (o.id, ())).collect();
    let stale: Vec<u64> = gpu
        .meshes
        .keys()
        .copied()
        .filter(|id| !live.contains_key(id))
        .collect();
    for id in stale {
        if let Some(mesh) = gpu.meshes.remove(&id) {
            mesh.delete(gl);
        }
    }
    for object in &frame.objects {
        let stale = gpu
            .meshes
            .get(&object.id)
            .map(|mesh| mesh.revision != object.revision)
            .unwrap_or(true);
        if stale {
            if let Some(old) = gpu.meshes.remove(&object.id) {
                old.delete(gl);
            }
            gpu.upload_mesh(gl, object.id, object.revision, &object.mesh);
        }
    }

    let vp = info.viewport_in_pixels();
    if vp.width_px <= 0 || vp.height_px <= 0 {
        return Ok(());
    }

    unsafe {
        gl.enable(glow::SCISSOR_TEST);
        gl.viewport(vp.left_px, vp.from_bottom_px, vp.width_px, vp.height_px);
        gl.scissor(vp.left_px, vp.from_bottom_px, vp.width_px, vp.height_px);
        gl.enable(glow::DEPTH_TEST);
        gl.depth_func(glow::LEQUAL);
        gl.clear_depth_f32(1.0);
        gl.clear_color(
            frame.clear[0],
            frame.clear[1],
            frame.clear[2],
            frame.clear[3],
        );
        gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        gl.enable(glow::CULL_FACE);
        gl.cull_face(glow::BACK);
        gl.front_face(glow::CCW);

        gl.use_program(Some(gpu.line_program));
        let loc = gl.get_uniform_location(gpu.line_program, "u_vp");
        gl.uniform_matrix_4_f32_slice(loc.as_ref(), false, &frame.view_proj);
        gl.bind_vertex_array(Some(gpu.grid_vao));
        gl.draw_arrays(glow::LINES, 0, gpu.grid_count);

        gl.use_program(Some(gpu.mesh_program));
        let vp_loc = gl.get_uniform_location(gpu.mesh_program, "u_vp");
        let model_loc = gl.get_uniform_location(gpu.mesh_program, "u_model");
        let color_loc = gl.get_uniform_location(gpu.mesh_program, "u_color");
        let sel_loc = gl.get_uniform_location(gpu.mesh_program, "u_selected");
        let alpha_loc = gl.get_uniform_location(gpu.mesh_program, "u_alpha");
        let clip_loc = gl.get_uniform_location(gpu.mesh_program, "u_clip_y");
        gl.uniform_matrix_4_f32_slice(vp_loc.as_ref(), false, &frame.view_proj);
        gl.uniform_1_f32(clip_loc.as_ref(), frame.clip_y);
        for transparent in [false, true] {
            if transparent {
                gl.enable(glow::BLEND);
                gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
                gl.depth_mask(false);
            }
            for object in &frame.objects {
                let is_glass = object.alpha < 0.99;
                if is_glass != transparent {
                    continue;
                }
                let Some(mesh) = gpu.meshes.get(&object.id) else {
                    continue;
                };
                gl.uniform_matrix_4_f32_slice(model_loc.as_ref(), false, &object.model);
                gl.uniform_3_f32(
                    color_loc.as_ref(),
                    object.color[0],
                    object.color[1],
                    object.color[2],
                );
                gl.uniform_1_f32(sel_loc.as_ref(), if object.selected { 1.0 } else { 0.0 });
                gl.uniform_1_f32(alpha_loc.as_ref(), object.alpha);
                gl.bind_vertex_array(Some(mesh.vao));
                gl.draw_elements(glow::TRIANGLES, mesh.count, glow::UNSIGNED_INT, 0);
            }
            if transparent {
                gl.depth_mask(true);
                gl.disable(glow::BLEND);
            }
        }

        gl.bind_vertex_array(None);
        gl.use_program(None);
        gl.disable(glow::CULL_FACE);
        gl.disable(glow::DEPTH_TEST);
        gl.disable(glow::SCISSOR_TEST);
    }
    Ok(())
}

impl GpuState {
    fn new(gl: &glow::Context) -> Result<Self, String> {
        Ok(Self {
            mesh_program: program(gl, MESH_VS, MESH_FS)?,
            line_program: program(gl, LINE_VS, LINE_FS)?,
            grid_vao: unsafe { gl.create_vertex_array().map_err(|e| e.to_string())? },
            grid_vbo: unsafe { gl.create_buffer().map_err(|e| e.to_string())? },
            grid_count: 0,
            meshes: HashMap::new(),
            grid_ready: false,
        })
    }

    fn upload_grid(&mut self, gl: &glow::Context, vertices: &[f32]) {
        unsafe {
            gl.bind_vertex_array(Some(self.grid_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.grid_vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, f32_bytes(vertices), glow::STATIC_DRAW);
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 24, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, 24, 12);
            gl.bind_vertex_array(None);
        }
        self.grid_count = (vertices.len() / 6) as i32;
    }

    fn upload_mesh(&mut self, gl: &glow::Context, id: u64, revision: u64, mesh: &UploadedMesh) {
        unsafe {
            let vao = gl.create_vertex_array().expect("vao");
            let vbo = gl.create_buffer().expect("vbo");
            let nbo = gl.create_buffer().expect("nbo");
            let ebo = gl.create_buffer().expect("ebo");
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                f32_bytes(&mesh.positions),
                glow::STATIC_DRAW,
            );
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 12, 0);
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(nbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                f32_bytes(&mesh.normals),
                glow::STATIC_DRAW,
            );
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, 12, 0);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                u32_bytes(&mesh.indices),
                glow::STATIC_DRAW,
            );
            gl.bind_vertex_array(None);
            self.meshes.insert(
                id,
                GpuMesh {
                    vao,
                    vbo,
                    nbo,
                    ebo,
                    count: mesh.indices.len() as i32,
                    revision,
                },
            );
        }
    }
}

impl GpuMesh {
    fn delete(self, gl: &glow::Context) {
        unsafe {
            gl.delete_vertex_array(self.vao);
            gl.delete_buffer(self.vbo);
            gl.delete_buffer(self.nbo);
            gl.delete_buffer(self.ebo);
        }
    }
}

fn program(gl: &glow::Context, vs: &str, fs: &str) -> Result<glow::Program, String> {
    unsafe {
        let program = gl.create_program().map_err(|e| e.to_string())?;
        let vs = shader(gl, glow::VERTEX_SHADER, vs)?;
        let fs = shader(gl, glow::FRAGMENT_SHADER, fs)?;
        gl.attach_shader(program, vs);
        gl.attach_shader(program, fs);
        gl.link_program(program);
        gl.delete_shader(vs);
        gl.delete_shader(fs);
        if !gl.get_program_link_status(program) {
            return Err(gl.get_program_info_log(program));
        }
        Ok(program)
    }
}

fn shader(gl: &glow::Context, kind: u32, source: &str) -> Result<glow::Shader, String> {
    unsafe {
        let shader = gl.create_shader(kind).map_err(|e| e.to_string())?;
        gl.shader_source(shader, source);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            return Err(gl.get_shader_info_log(shader));
        }
        Ok(shader)
    }
}

fn f32_bytes(data: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(data.as_ptr().cast(), std::mem::size_of_val(data)) }
}

fn u32_bytes(data: &[u32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(data.as_ptr().cast(), std::mem::size_of_val(data)) }
}
