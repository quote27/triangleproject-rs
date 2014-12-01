#![feature(phase)]
#![feature(globs)]

extern crate current;
extern crate shader_version;
extern crate vecmath;
extern crate event;
extern crate input;
extern crate cam;
extern crate gfx;
extern crate glfw;
extern crate glfw_window;
// extern crate sdl2;
// extern crate sdl2_window;
#[phase(plugin)]
extern crate gfx_macros;
extern crate time;

use current::{ Set };
use std::rand;
use std::cell::RefCell;
use glfw_window::GlfwWindow;
// use sdl2_window::Sdl2Window;
use gfx::{ Device, DeviceHelper, ToSlice, Mesh };
use event::{ Events, WindowSettings };
use event::window::{ CaptureCursor };

//----------------------------------------
// line associated data

#[vertex_format]
struct LineVertex {
    a_pos: [f32, ..3],
    a_color: [f32, ..3],
}

impl LineVertex {
    fn new(pos: [f32, ..3], color: [f32, ..3]) -> LineVertex {
        LineVertex {
            a_pos: pos,
            a_color: color,
        }
    }
    fn rand_pos(color: [f32, ..3]) -> LineVertex {
        let x = rand::random::<f32>();
        let y = rand::random::<f32>();
        let z = rand::random::<f32>();

        LineVertex::new([x, y, z], color)
    }
}

#[shader_param(LineBatch)]
struct LineParams {
    u_model_view_proj: [[f32, ..4], ..4],
    u_alpha: f32,
}

static LINE_VERTEX_SRC: gfx::ShaderSource<'static> = shaders! {
GLSL_120: b"
    #version 120
    attribute vec3 a_pos;
    attribute vec3 a_color;
    varying vec4 v_color;
    uniform mat4 u_model_view_proj;
    uniform float u_alpha;
    void main() {
        v_color = vec4(a_color, u_alpha);
        gl_Position = u_model_view_proj * vec4(a_pos, 1.0);
    }
"
GLSL_150: b"
    #version 150 core
    in vec3 a_pos;
    in vec3 a_color;
    out vec4 v_color;
    uniform mat4 u_model_view_proj;
    uniform float u_alpha;
    void main() {
        v_color = vec4(a_color, u_alpha);
        gl_Position = u_model_view_proj * vec4(a_pos, 1.0);
    }
"
};

static LINE_FRAGMENT_SRC: gfx::ShaderSource<'static> = shaders! {
GLSL_120: b"
    #version 120
    varying vec4 v_color;
    void main() {
        gl_FragColor = v_color;
    }
"
GLSL_150: b"
    #version 150 core
    in vec4 v_color;
    out vec4 o_color;
    void main() {
        o_color = v_color;
    }
"
};

#[inline]
fn rand_color() -> [f32, ..3] {
    [rand::random::<f32>(), rand::random::<f32>(), rand::random::<f32>()]
}

fn main() {
    let (win_width, win_height) = (640, 480);
    let mut window = GlfwWindow::new(
        shader_version::opengl::OpenGL_3_2,
        WindowSettings {
            title: "cube".to_string(),
            size: [win_width, win_height],
            fullscreen: false,
            exit_on_esc: true,
            samples: 4,
        }
    );

    window.set_mut(CaptureCursor(true));

    let device = gfx::GlDevice::new(|s| window.window.get_proc_address(s));
    let mut graphics = gfx::Graphics::new(device);
    let mut frame = gfx::Frame::new(win_width as u16, win_height as u16);
    let state = gfx::DrawState::new()
        .depth(gfx::state::LessEqual, true)
        .blend(gfx::BlendAlpha); // BlendAlpha or BlendAdditive


    // start linedrawing prep
    let num_triangles = 4;
    let num_points = num_triangles * 3;

    // vertex data
    let mut lines_vd = Vec::from_fn(num_points, |_| LineVertex::rand_pos(rand_color()));
    let lines_vd_buff = graphics.device.create_buffer::<LineVertex>(lines_vd.len(), gfx::UsageDynamic);
    graphics.device.update_buffer(lines_vd_buff, lines_vd.as_slice(), 0);
    let lines_mesh = Mesh::from_format(lines_vd_buff, lines_vd.len() as u32);

    // index data
    let mut lines_idxd = Vec::with_capacity(num_triangles * 6);
    let mut lines_tri_idxd = Vec::with_capacity(num_triangles * 3);
    for i in range(0, num_triangles) {
        let off = i as u8 * 3;
        //let ioff = i * 6;
        lines_idxd.push(off + 0);
        lines_idxd.push(off + 1);
        lines_idxd.push(off + 1);
        lines_idxd.push(off + 2);
        lines_idxd.push(off + 2);
        lines_idxd.push(off + 0);

        lines_tri_idxd.push(off + 0);
        lines_tri_idxd.push(off + 1);
        lines_tri_idxd.push(off + 2);
    }
    let lines_idx_buff = graphics.device.create_buffer::<u8>(lines_idxd.len(), gfx::UsageDynamic);
    graphics.device.update_buffer(lines_idx_buff, lines_idxd.as_slice(), 0);
    let lines_slice = lines_idx_buff.to_slice(gfx::Line);

    let lines_tri_idx_buff = graphics.device.create_buffer::<u8>(lines_tri_idxd.len(), gfx::UsageDynamic);
    graphics.device.update_buffer(lines_tri_idx_buff, lines_tri_idxd.as_slice(), 0);
    let lines_tri_slice = lines_tri_idx_buff.to_slice(gfx::TriangleList);


    let lines_prog = graphics.device.link_program(LINE_VERTEX_SRC.clone(), LINE_FRAGMENT_SRC.clone()).unwrap();

    let lines_batch: LineBatch = graphics.make_batch(&lines_prog, &lines_mesh, lines_slice, &state).unwrap();
    let lines_tri_batch: LineBatch = graphics.make_batch(&lines_prog, &lines_mesh, lines_tri_slice, &state).unwrap();

    let mut lines_data = LineParams {
        u_model_view_proj: vecmath::mat4_id(),
        u_alpha: 1.0,
    };


    let model = vecmath::mat4_id();
    let mut projection = cam::CameraPerspective {
            fov: 45.0f32,
            near_clip: 0.1,
            far_clip: 1000.0,
            aspect_ratio: (win_width as f32) / (win_height as f32)
        }.projection();

    let mut first_person = cam::FirstPerson::new(
        [0.5f32, 0.5, 4.0],
        cam::FirstPersonSettings::keyboard_wasd(),
    );
    {
        use input::{keyboard, Keyboard};
        first_person.settings.move_faster_button = Keyboard(keyboard::LShift);
        first_person.settings.fly_down_button = Keyboard(keyboard::LCtrl);
        first_person.settings.speed_vertical *= 2.0;
    }

    // let mut rng = rand::task_rng();
    let mut frame_count = 0u;

    let window = RefCell::new(window);
    for e in Events::new(&window) {
        use event::{RenderEvent, ResizeEvent};
        use vecmath::col_mat4_mul;

        first_person.event(&e);
        e.render(|args| {
            graphics.clear(
                gfx::ClearData {
                    color: [0.0, 0.0, 0.0, 1.0],
                    depth: 1.0,
                    stencil: 0,
                },
                gfx::COLOR | gfx::DEPTH,
                &frame
            );

            let translate = vecmath::mat4_id();

            lines_data.u_model_view_proj = cam::model_view_projection(
                    col_mat4_mul(model, translate),
                    first_person.camera(args.ext_dt).orthogonal(),
                    projection
                );
            lines_data.u_alpha = 1.0;

            graphics.draw(&lines_batch, &lines_data, &frame);

            lines_data.u_alpha = 0.3;
            graphics.draw(&lines_tri_batch, &lines_data, &frame);

            graphics.end_frame();
            frame_count += 1;

            // if frame_count % 10 == 0 {
            //     rng.shuffle(lines_idxd.as_mut_slice());
            //     graphics.device.update_buffer(lines_idx_buff, lines_idxd.as_slice(), 0);
            // }

            if frame_count % 20 == 0 {
                for lv in lines_vd.iter_mut() {
                    *lv = LineVertex::rand_pos(rand_color());
                }
                graphics.device.update_buffer(lines_vd_buff, lines_vd.as_slice(), 0);
            }
        });

        e.resize(|w, h| {
            frame = gfx::Frame::new(w as u16, h as u16);

            projection = cam::CameraPerspective {
                fov: 45.0f32,
                near_clip: 0.1,
                far_clip: 1000.0,
                aspect_ratio: (w as f32) / (h as f32)
            }.projection();
        });
    }
}

