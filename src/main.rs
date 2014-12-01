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
use std::rand::{task_rng, Rng};
use std::cell::RefCell;
use glfw_window::GlfwWindow;
// use sdl2_window::Sdl2Window;
use gfx::{ Device, DeviceHelper, ToSlice, Mesh };
use event::{ Events, WindowSettings };
use event::window::{ CaptureCursor };

//----------------------------------------
// Cube associated data

#[vertex_format]
struct Vertex {
    #[as_float]
    a_pos: [i8, ..3],
    #[as_float]
    a_tex_coord: [u8, ..2],
}

impl Vertex {
    fn new(pos: [i8, ..3], tc: [u8, ..2]) -> Vertex {
        Vertex {
            a_pos: pos,
            a_tex_coord: tc,
        }
    }
}

#[shader_param(CubeBatch)]
struct Params {
    u_model_view_proj: [[f32, ..4], ..4],
    t_color: gfx::shade::TextureParam,
}

static VERTEX_SRC: gfx::ShaderSource<'static> = shaders! {
GLSL_120: b"
    #version 120
    attribute vec3 a_pos;
    attribute vec2 a_tex_coord;
    varying vec2 v_TexCoord;
    uniform mat4 u_model_view_proj;
    void main() {
        v_TexCoord = a_tex_coord;
        gl_Position = u_model_view_proj * vec4(a_pos, 1.0);
    }
"
GLSL_150: b"
    #version 150 core
    in vec3 a_pos;
    in vec2 a_tex_coord;
    out vec2 v_TexCoord;
    uniform mat4 u_model_view_proj;
    void main() {
        v_TexCoord = a_tex_coord;
        gl_Position = u_model_view_proj * vec4(a_pos, 1.0);
    }
"
};

static FRAGMENT_SRC: gfx::ShaderSource<'static> = shaders! {
GLSL_120: b"
    #version 120
    varying vec2 v_TexCoord;
    uniform sampler2D t_color;
    void main() {
        vec4 tex = texture2D(t_color, v_TexCoord);
        float blend = dot(v_TexCoord-vec2(0.5,0.5), v_TexCoord-vec2(0.5,0.5));
        gl_FragColor = mix(tex, vec4(0.0,0.0,0.0,0.0), blend*1.0);
    }
"
GLSL_150: b"
    #version 150 core
    in vec2 v_TexCoord;
    out vec4 o_Color;
    uniform sampler2D t_color;
    void main() {
        vec4 tex = texture(t_color, v_TexCoord);
        float blend = dot(v_TexCoord-vec2(0.5,0.5), v_TexCoord-vec2(0.5,0.5));
        o_Color = vec4(0.0, 0.5, 0.8, 1.0);  // mix(tex, vec4(0.0,0.0,0.0,0.0), blend*1.0);
    }
"
};

//----------------------------------------


//----------------------------------------
// line associated data

#[vertex_format]
struct LineVertex {
    a_pos: [f32, ..3],
    a_color: [f32, ..4],
}

impl LineVertex {
    fn new(pos: [f32, ..3], color: [f32, ..4]) -> LineVertex {
        LineVertex {
            a_pos: pos,
            a_color: color,
        }
    }
    fn rand_pos(color: [f32, ..4]) -> LineVertex {
        let x = rand::random::<f32>();
        let y = rand::random::<f32>();
        let z = rand::random::<f32>();

        LineVertex {
            a_pos: [x, y, z],
            a_color: color,
        }
    }
}

#[shader_param(LineBatch)]
struct LineParams {
    u_model_view_proj: [[f32, ..4], ..4],
}

// static LINE_VERTEX_SRC: gfx::ShaderSource<'static> = shaders! {
// GLSL_120: b"
//     #version 120
//     attribute vec3 a_pos;
//     uniform mat4 u_model_view_proj;
//     void main() {
//         gl_Position = u_model_view_proj * vec4(a_pos, 1.0);
//     }
// "
// GLSL_150: b"
//     #version 150 core
//     in vec3 a_pos;
//     uniform mat4 u_model_view_proj;
//     void main() {
//         gl_Position = u_model_view_proj * vec4(a_pos, 1.0);
//     }
// "
// };
// 
// static LINE_FRAGMENT_SRC: gfx::ShaderSource<'static> = shaders! {
// GLSL_120: b"
//     #version 120
//     uniform vec4 u_color;
//     void main() {
//         gl_FragColor = u_color;
//     }
// "
// GLSL_150: b"
//     #version 150 core
//     out vec4 o_color;
//     uniform vec4 u_color;
//     void main() {
//         o_color = u_color;
//     }
// "
// };

static LINE_VERTEX_SRC: gfx::ShaderSource<'static> = shaders! {
GLSL_120: b"
    #version 120
    attribute vec3 a_pos;
    attribute vec4 a_color;
    varying vec4 v_color;
    uniform mat4 u_model_view_proj;
    void main() {
        v_color = a_color;
        gl_Position = u_model_view_proj * vec4(a_pos, 1.0);
    }
"
GLSL_150: b"
    #version 150 core
    in vec3 a_pos;
    in vec4 a_color;
    out vec4 v_color;
    uniform mat4 u_model_view_proj;
    void main() {
        v_color = a_color;
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


//----------------------------------------

#[inline]
fn rand_color() -> [f32, ..4] {
    [rand::random::<f32>(), rand::random::<f32>(), rand::random::<f32>(), 1.0]
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

    //let mut device = gfx::GlDevice::new(|s| window.gl_get_proc_address(s) );
    let mut device = gfx::GlDevice::new(|s|
        window.window.get_proc_address(s)
    );
    let mut frame = gfx::Frame::new(win_width as u16, win_height as u16);
    let state = gfx::DrawState::new().depth(gfx::state::LessEqual, true);

    let vertex_data = vec![
        //top (0, 0, 1)
        Vertex::new([-1, -1,  1], [0, 0]),
        Vertex::new([ 1, -1,  1], [1, 0]),
        Vertex::new([ 1,  1,  1], [1, 1]),
        Vertex::new([-1,  1,  1], [0, 1]),
        //bottom (0, 0, -1)
        Vertex::new([ 1,  1, -1], [0, 0]),
        Vertex::new([-1,  1, -1], [1, 0]),
        Vertex::new([-1, -1, -1], [1, 1]),
        Vertex::new([ 1, -1, -1], [0, 1]),
        //right (1, 0, 0)
        Vertex::new([ 1, -1, -1], [0, 0]),
        Vertex::new([ 1,  1, -1], [1, 0]),
        Vertex::new([ 1,  1,  1], [1, 1]),
        Vertex::new([ 1, -1,  1], [0, 1]),
        //left (-1, 0, 0)
        Vertex::new([-1,  1,  1], [0, 0]),
        Vertex::new([-1, -1,  1], [1, 0]),
        Vertex::new([-1, -1, -1], [1, 1]),
        Vertex::new([-1,  1, -1], [0, 1]),
        //front (0, 1, 0)
        Vertex::new([-1,  1, -1], [0, 0]),
        Vertex::new([ 1,  1, -1], [1, 0]),
        Vertex::new([ 1,  1,  1], [1, 1]),
        Vertex::new([-1,  1,  1], [0, 1]),
        //back (0, -1, 0)
        Vertex::new([ 1, -1,  1], [0, 0]),
        Vertex::new([-1, -1,  1], [1, 0]),
        Vertex::new([-1, -1, -1], [1, 1]),
        Vertex::new([ 1, -1, -1], [0, 1]),

        // extras
        Vertex::new([0, 0, 0], [0, 0]), // 24
        Vertex::new([1, 0, 0], [1, 0]), // 25
        Vertex::new([1, 1, 0], [1, 1]), // 26
        Vertex::new([0, 1, 0], [0, 1]), // 27
        Vertex::new([0, 0, 1], [0, 1]), // 28
    ];

    let mesh = device.create_mesh(vertex_data.as_slice());

//    let index_data: &[u8] = &[
//         0,  1,  2,  2,  3,  0, // top
//         4,  6,  5,  6,  4,  7, // bottom
//         8,  9, 10, 10, 11,  8, // right
//        12, 14, 13, 14, 12, 16, // left
//        16, 18, 17, 18, 16, 19, // front
//        20, 21, 22, 22, 23, 20, // back
//    ];

    // cube-outline index data
    let index_data: &[u8] = &[
        //0, 1, 1, 2, 2, 3, 3, 0,
        //8, 9, 9, 10, 10, 11, 11, 8, // right
        //16, 17, 17, 18, 18, 19, 19, 16, // front
        24, 25, 24, 27, 24, 28 // angle
    ];

    let slice = device
        .create_buffer_static::<u8>(index_data)
        .to_slice(gfx::Line);
    
    let tinfo = gfx::tex::TextureInfo {
        width: 1,
        height: 1,
        depth: 1,
        levels: 1,
        kind: gfx::tex::Texture2D,
        format: gfx::tex::RGBA8,
    };
    let img_info = tinfo.to_image_info();
    let texture = device.create_texture(tinfo).unwrap();
    device.update_texture(
            &texture, 
            &img_info,
            //vec![0x20u8, 0xA0u8, 0xC0u8, 0x00u8].as_slice()
            vec![0xffu8, 0xffu8, 0x00u8, 0x00u8].as_slice() // rgba
        ).unwrap();

    let sampler = device.create_sampler(
        gfx::tex::SamplerInfo::new(
            gfx::tex::Bilinear, 
            gfx::tex::Clamp
        )
    );
    
    let program = device.link_program(
            VERTEX_SRC.clone(), 
            FRAGMENT_SRC.clone()
        ).unwrap();

    let mut graphics = gfx::Graphics::new(device);
    let batch: CubeBatch = graphics.make_batch(&program, &mesh, slice, &state).unwrap();

    let mut data = Params {
        u_model_view_proj: vecmath::mat4_id(),
        t_color: (texture, Some(sampler)),
    };





    // -------------------------
    // start linedrawing prep
    let num_points = 1024;
    let mut lines_vd = Vec::from_fn(num_points, |i| {
        LineVertex::rand_pos(rand_color())
    });
    // let lines_vd = vec![
    //     LineVertex::new([-1,  1,  1], [1.0, 1.0, 1.0, 1.0]),
    //     LineVertex::new([ 1,  1,  1], [1.0, 1.0, 1.0, 1.0]),
    //     LineVertex::new([ 1,  3,  1], [1.0, 1.0, 1.0, 1.0]),
    //     LineVertex::new([-1,  3,  1], [1.0, 1.0, 1.0, 1.0]),
    // ]; //  0, 1, 1, 2, 2, 3, 3, 0, // square

    let lines_vd_buff = graphics.device.create_buffer::<LineVertex>(lines_vd.len(), gfx::UsageDynamic);
    graphics.device.update_buffer(lines_vd_buff, lines_vd.as_slice(), 0);
    let lines_mesh = Mesh::from_format(lines_vd_buff, lines_vd.len() as u32);


    //let lines_mesh = graphics.device.create_mesh(lines_vd.as_slice());

    let mut lines_idxd = Vec::from_fn(lines_vd.len(), |i| i as u8);
    let lines_idx_buff = graphics.device.create_buffer::<u8>(lines_idxd.len(), gfx::UsageDynamic);
    graphics.device.update_buffer(lines_idx_buff, lines_idxd.as_slice(), 0);

    let lines_slice = lines_idx_buff.to_slice(gfx::Line);

    let lines_prog = graphics.device.link_program(LINE_VERTEX_SRC.clone(), LINE_FRAGMENT_SRC.clone()).unwrap();

    let lines_batch: LineBatch = graphics.make_batch(&lines_prog, &lines_mesh, lines_slice, &state).unwrap();

    let mut lines_data = LineParams {
        u_model_view_proj: vecmath::mat4_id(),
    };





    //-------------------------------
    //-------------------------------
    //-------------------------------
    //-------------------------------

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

    let mut rng = task_rng();
    let mut frame_count = 0u;

    let window = RefCell::new(window);
    for e in Events::new(&window) {
        use event::{RenderEvent, ResizeEvent};
        use vecmath::col_mat4_mul;
        use std::num::FloatMath;

        first_person.event(&e);
        e.render(|args| {
            graphics.clear(
                gfx::ClearData {
                    //color: [0.3, 0.3, 0.3, 1.0],
                    color: [0.0, 0.0, 0.0, 1.0],
                    depth: 1.0,
                    stencil: 0,
                },
                gfx::COLOR | gfx::DEPTH,
                &frame
            );

            let mut translate = vecmath::mat4_id();
            let scale = 0.2;
            translate[0][0] = scale;
            translate[1][1] = scale;
            translate[2][2] = scale;

            let scale = scale * 2.0 + 0.1;

//            for x in range(0u, 3) {
//                for z in range(0u, 3) {
//                    translate[3][0] = scale * x as f32;
//                    translate[3][2] = scale * z as f32;
//
//                    translate[3][1] = (frame_count as f32 / 60.0).sin() * x as f32 / 5.0 * z as f32 / 5.0;
//
//                    data.u_model_view_proj = cam::model_view_projection(
//                            col_mat4_mul(model, translate),
//                            first_person.camera(args.ext_dt).orthogonal(),
//                            projection
//                        );
//                    graphics.draw(&batch, &data, &frame);
//                }
//            }

            lines_data.u_model_view_proj = cam::model_view_projection(
                    col_mat4_mul(model, translate),
                    first_person.camera(args.ext_dt).orthogonal(),
                    projection
                );

            graphics.draw(&lines_batch, &lines_data, &frame);

            graphics.end_frame();
            frame_count += 1;

            if frame_count % 10 == 0 {
                rng.shuffle(lines_idxd.as_mut_slice());
                graphics.device.update_buffer(lines_idx_buff, lines_idxd.as_slice(), 0);
            }

            if frame_count % 60 == 0 {
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

