#![feature(plugin, custom_attribute)]
#![plugin(gfx_macros)]

extern crate gfx;
extern crate gfx_device_gl;
extern crate shader_version;

extern crate glfw_window;
extern crate window;
extern crate quack;
extern crate input;
extern crate event;
extern crate vecmath;
extern crate cam;
extern crate camera_controllers;
extern crate rand;

use std::cell::RefCell;
use quack::{ Set };
use glfw_window::GlfwWindow;
// use sdl2_window::Sdl2Window;
use gfx::{ Device, DeviceExt, ToSlice, Mesh, Resources };
use window::{ WindowSettings, CaptureCursor };
use timer::{ Timer, TimeMap };
use camera_controllers::{ FirstPerson, FirstPersonSettings };

mod timer;

//----------------------------------------
// line associated data

#[vertex_format]
#[derive(Copy)]
struct LineVertex {
    a_pos: [f32; 3],
    a_color: [f32; 3],
    a_normal: [f32; 3],
}

impl LineVertex {
    fn new(pos: [f32; 3], color: [f32; 3], normal: [f32; 3]) -> LineVertex {
        LineVertex {
            a_pos: pos,
            a_color: color,
            a_normal: normal,
        }
    }
    fn rand_pos(color: [f32; 3]) -> LineVertex {
        let x = rand::random::<f32>();
        let y = rand::random::<f32>();
        let z = rand::random::<f32>();

        LineVertex::new([x, y, z], color, [0.0, 0.0, 0.0])
    }
}

#[shader_param]
struct LineParams<R: gfx::Resources> {
    #[name = "u_model_view_proj"]
    u_model_view_proj: [[f32; 4]; 4],
    #[name = "u_normal_mat"]
    u_normal_mat: [[f32; 3]; 3],
    #[name = "u_alpha"]
    u_alpha: f32,
    _dummy: std::marker::PhantomData<R>,
}

const LINE_VERTEX_SRC: [&'static [u8]; 2] = [ b"
    #version 120
    attribute vec3 a_pos;
    attribute vec3 a_color;
    attribute vec3 a_normal;
    varying vec4 v_color;
    uniform mat4 u_model_view_proj;
    uniform mat3 u_normal_mat;
    uniform float u_alpha;
    void main() {
        v_color = vec4(a_color, u_alpha);
        //gl_Position = u_model_view_proj * vec4(a_pos, 1.0);
        //vec3 off = vec3(0.0); //normalize(u_normal_mat * a_normal);
        gl_Position = u_model_view_proj * vec4(a_pos + a_normal * 0.02 * a_color.b, 1.0);
    }
", b"
    #version 150 core
    in vec3 a_pos;
    in vec3 a_color;
    in vec3 a_normal;
    out vec4 v_color;
    uniform mat4 u_model_view_proj;
    uniform mat3 u_normal_mat;
    uniform float u_alpha;
    void main() {
        v_color = vec4(a_color, u_alpha);
        //vec3 off = vec3(0.0); //normalize(u_normal_mat * a_normal);
        gl_Position = u_model_view_proj * vec4(a_pos + a_normal * 0.02 * a_color.b, 1.0);
    }
"];

const LINE_FRAGMENT_SRC: [&'static [u8]; 2] = [ b"
    #version 120
    varying vec4 v_color;
    void main() {
        gl_FragColor = v_color;
    }
", b"
    #version 150 core
    in vec4 v_color;
    out vec4 o_color;
    void main() {
        o_color = v_color;
    }
"];

#[inline]
fn rand_color() -> [f32; 3] {
    [rand::random::<f32>(), rand::random::<f32>(), rand::random::<f32>()]
    //[0.0, 0.2, 0.8]
}

fn rand_triangles(lines_vd: &mut Vec<LineVertex>, num_triangles: usize, subdivide_lines: usize) {
    for _ in 0..num_triangles {
        let a = LineVertex::rand_pos(rand_color());
        let b = LineVertex::rand_pos(rand_color());
        let c = LineVertex::rand_pos(rand_color());

        // calculate triangle normal
        let ba = vecmath::vec3_sub(b.a_pos, a.a_pos);
        let ca = vecmath::vec3_sub(c.a_pos, a.a_pos);
        let tnorm = vecmath::vec3_normalized(vecmath::vec3_cross(ba, ca));

        for i in 0u8..3 {
            let (t1, t2) = match i {
                    0 => (a, b),
                    1 => (b, c),
                    _ => (c, a),
                };

            let v = vecmath::vec3_sub(t2.a_pos, t1.a_pos);
            let vdiv = vecmath::vec3_scale(v, 1.0 / subdivide_lines as f32);

            let mut p1 = t1.a_pos;
            lines_vd.push(LineVertex::new(p1, rand_color(), [0.0, 0.0, 0.0]));
            p1 = vecmath::vec3_add(p1, vdiv);

            for j in 1..subdivide_lines {
                if j % 2 == 0 {
                    lines_vd.push(LineVertex::new(p1, rand_color(), tnorm));
                } else {
                    lines_vd.push(LineVertex::new(p1, rand_color(), vecmath::vec3_sub([0.0, 0.0, 0.0], tnorm)));
                }
                p1 = vecmath::vec3_add(p1, vdiv);
            } // i32entionally skip last position
        }
    }
}

fn main() {
    let (win_width, win_height) = (640, 480);
    let mut window = GlfwWindow::new(
        shader_version::opengl::OpenGL::_3_2,
        WindowSettings {
            title: "cube".to_string(),
            size: [win_width, win_height],
            fullscreen: false,
            exit_on_esc: true,
            samples: 4,
        }
    );

    window.set_mut(CaptureCursor(true));

    type R = gfx_device_gl::GlResources;
    let mut device = gfx_device_gl::GlDevice::new(|s| window.window.get_proc_address(s));
    let mut frame = gfx::Frame::new(win_width as u16, win_height as u16);
    let state = gfx::DrawState::new()
        .depth(gfx::state::Comparison::LessEqual, true)
        .blend(gfx::BlendPreset::Alpha); // BlendAlpha or BlendAdditive


    // start linedrawing prep
    let num_triangles = 4usize;
    let subdivide_lines = 32usize;

    let mut _flip_wireframe = false;
    let mut _scroll_colors = false;
    let mut _generate_new_triangles = true;


    // vertex data
    //let mut lines_vd = Vec::from_fn(num_triangles * 3, |_| LineVertex::rand_pos(rand_color()));
    let mut lines_vd = Vec::new(); //Vec::from_fn(num_triangles * 3, |_| LineVertex::rand_pos(rand_color()));
    rand_triangles(&mut lines_vd, num_triangles, subdivide_lines);

    let lines_vd_buff = device.create_buffer::<LineVertex>(lines_vd.len(), gfx::BufferUsage::Dynamic);
    device.update_buffer(lines_vd_buff, lines_vd.as_slice(), 0);
    let lines_mesh = Mesh::from_format(lines_vd_buff, lines_vd.len() as u32);

    // index data
    let mut lines_idxd = Vec::with_capacity(num_triangles * 6);
    let mut lines_tri_idxd = Vec::with_capacity(num_triangles * 3);
    for i in 0..num_triangles {
        let off = (i * subdivide_lines) as u16 * 3;

        lines_idxd.push(off + 0);
        for j in 1..3 * subdivide_lines as u16 {
            lines_idxd.push(off + j);
            lines_idxd.push(off + j);
        }
        lines_idxd.push(off + 0);

        lines_tri_idxd.push(off + 0);
        lines_tri_idxd.push(off + subdivide_lines as u16);
        lines_tri_idxd.push(off + subdivide_lines as u16 * 2);
    }
    let lines_idx_buff = device.create_buffer::<u16>(lines_idxd.len(), gfx::BufferUsage::Dynamic);
    device.update_buffer(lines_idx_buff, lines_idxd.as_slice(), 0);
    let lines_slice = lines_idx_buff.to_slice(gfx::PrimitiveType::Line);

    let lines_tri_idx_buff = device.create_buffer::<u16>(lines_tri_idxd.len(), gfx::BufferUsage::Dynamic);
    device.update_buffer(lines_tri_idx_buff, lines_tri_idxd.as_slice(), 0);
    let lines_tri_slice = lines_tri_idx_buff.to_slice(gfx::PrimitiveType::TriangleList);


    let line_vertex = gfx::ShaderSource {
        glsl_120: Some(LINE_VERTEX_SRC[0]),
        glsl_150: Some(LINE_VERTEX_SRC[1]),
        .. gfx::ShaderSource::empty()
    };
    let line_fragment = gfx::ShaderSource {
        glsl_120: Some(LINE_FRAGMENT_SRC[0]),
        glsl_150: Some(LINE_FRAGMENT_SRC[1]),
        .. gfx::ShaderSource::empty()
    };

    let shader_model = device.get_capabilities().shader_model;

    let lines_prog = device.link_program(
        line_vertex.choose(shader_model).unwrap(),
        line_fragment.choose(shader_model).unwrap(),
    ).unwrap();

    let mut graphics = gfx::Graphics::new(device);

    let lines_batch: gfx::batch::RefBatch<LineParams<R>> = graphics.make_batch(&lines_prog, &lines_mesh, lines_slice, &state).unwrap();
    let lines_tri_batch: gfx::batch::RefBatch<LineParams<R>> = graphics.make_batch(&lines_prog, &lines_mesh, lines_tri_slice, &state).unwrap();

    let mut lines_data = LineParams {
        u_model_view_proj: vecmath::mat4_id(),
        u_alpha: 1.0,
        u_normal_mat: vecmath::mat3_id(),
        _dummy: std::marker::PhantomData,
    };



    // // plane
    // let plane_vd = vec!(
    //         LineVertex::new([ 0.0 ,  0.0,  1.0], [1.0, 1.0, 1.0]), // front / nose
    //         LineVertex::new([ 0.75,  0.0, -1.0], [1.0, 1.0, 1.0]), // left wing - 'port'
    //         LineVertex::new([-0.75,  0.0, -1.0], [1.0, 1.0, 1.0]), // right wing - 'starboard'
    //         LineVertex::new([ 0.0 ,  0.0, -1.0], [1.0, 1.0, 1.0]), // back midpoi32 between wings
    //         LineVertex::new([ 0.0 , -0.4, -1.0], [1.0, 1.0, 1.0]), // back bottom fin
    //     );

    // let plane_idx = vec!(
    //         0u8, 1, 3,
    //         0, 3, 2,
    //         0, 4, 3,
    //         0, 3, 4,
    //     );

    // let plane_vd_b = graphics.device.create_buffer::<LineVertex>(plane_vd.len(), gfx::UsageStatic);
    // graphics.device.update_buffer(plane_vd_b, plane_vd.as_slice(), 0);
    // let plane_mesh = Mesh::from_format(plane_vd_b, plane_vd.len() as u32);

    // let plane_idx_b = graphics.device.create_buffer::<u8>(plane_idx.len(), gfx::UsageStatic);
    // graphics.device.update_buffer(plane_idx_b, plane_idx.as_slice(), 0);
    // let plane_slice = plane_idx_b.to_slice(gfx::TriangleList);

    // let plane_batch: LineBatch = graphics.make_batch(&lines_prog, &plane_mesh, plane_slice, &state).unwrap();


    let model = vecmath::mat4_id();
    let mut projection = cam::CameraPerspective {
            fov: 45.0f32,
            near_clip: 0.1,
            far_clip: 1000.0,
            aspect_ratio: (win_width as f32) / (win_height as f32)
        }.projection();

    let mut first_person = FirstPerson::new(
        [0.5f32, 0.5, 4.0],
        FirstPersonSettings::keyboard_wasd(),
    );
    {
        use input::keyboard::Key;
        use input::Button::Keyboard;
        first_person.settings.move_faster_button = Keyboard(Key::LShift);
        first_person.settings.fly_down_button = Keyboard(Key::LCtrl);
        first_person.settings.speed_vertical *= 2.0;
    }

    // let mut rng = rand::task_rng();
    let mut frame_count = 0u32;
    let mut _pause = true;


    let window = RefCell::new(window);
    for e in event::events(&window) {
        use event::{RenderEvent, RenderArgs, ResizeEvent, PressEvent};

        first_person.event(&e);
        e.render(|args: &RenderArgs| {

            graphics.clear(
                gfx::ClearData {
                    color: [0.0, 0.0, 0.0, 1.0],
                    depth: 1.0,
                    stencil: 0,
                },
                gfx::COLOR | gfx::DEPTH,
                &frame
            );

            let mut translate = vecmath::mat4_id();

            lines_data.u_model_view_proj = cam::model_view_projection(
                    vecmath::col_mat4_mul(model, translate),
                    first_person.camera(args.ext_dt).orthogonal(),
                    projection
                );
            lines_data.u_alpha = 1.0;

            graphics.draw(&lines_batch, &lines_data, &frame);

            // draw triangle
            lines_data.u_alpha = 0.3;
            graphics.draw(&lines_tri_batch, &lines_data, &frame);

            // draw plane
            // translate[3][0] = 3.0;
            // lines_data.u_model_view_proj = cam::model_view_projection(
            //         vecmath::col_mat4_mul(model, translate),
            //         first_person.camera(args.ext_dt).orthogonal(),
            //         projection
            //     );
            // graphics.draw(&plane_batch, &lines_data, &frame);

            graphics.end_frame();
            frame_count += 1;

            if _pause { return; }

            // change line connections
            // if frame_count % 10 == 0 {
            //     rng.shuffle(lines_idxd.as_mut_slice());
            //     graphics.device.update_buffer(lines_idx_buff, lines_idxd.as_slice(), 0);
            // }

            let mut _update_buff = false;

            //if frame_count % 100 == 0 {
            //    subdivide_lines = match subdivide_lines {
            //        1 => 16,
            //        _ => 1,
            //    }
            //}

            // flip waveform each frame
            if false && frame_count % 4 == 0 {
                for lv in lines_vd.iter_mut() {
                    lv.a_normal = vecmath::vec3_sub([0.0, 0.0, 0.0], lv.a_normal);
                }

                _update_buff = true;
            }

            // scroll colors [optionally flip waveform to make it look like a static scroll]
            if false && frame_count % 2 == 0 {
                let color_0 = lines_vd[0].a_color;
                for i in 0..lines_vd.len()-1 {
                    lines_vd[i].a_normal = vecmath::vec3_sub([0.0, 0.0, 0.0], lines_vd[i].a_normal);
                    lines_vd[i].a_color = lines_vd[i+1].a_color;
                }
                let last_idx = lines_vd.len()-1;
                lines_vd[last_idx].a_normal = vecmath::vec3_sub([0.0, 0.0, 0.0], lines_vd[last_idx].a_normal);
                lines_vd[last_idx].a_color = color_0;

                _update_buff = true;
            }

            // generate new subdivide triangles
            if frame_count % 30 == 0 {
                lines_vd.clear();
                rand_triangles(&mut lines_vd, num_triangles, subdivide_lines);
                _update_buff = true;
            }

            if _update_buff {
                graphics.device.update_buffer(lines_vd_buff, lines_vd.as_slice(), 0);
            }
        });

        e.resize(|w: u32, h: u32| {
            frame = gfx::Frame::new(w as u16, h as u16);

            projection = cam::CameraPerspective {
                fov: 45.0f32,
                near_clip: 0.1,
                far_clip: 1000.0,
                aspect_ratio: (w as f32) / (h as f32)
            }.projection();
        });

        e.press(|button| {
            use input::keyboard::Key;
            use input::Button::Keyboard;
            match button {
                Keyboard(Key::P) => _pause = !_pause,
                _ => ()
            }
        });
    }
}
