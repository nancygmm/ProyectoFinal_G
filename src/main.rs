use nalgebra_glm::{Vec3, Mat4, look_at, perspective};
use minifb::{Key, Window, WindowOptions};
use std::time::Duration;
use std::f32::consts::PI;

mod framebuffer;
mod triangle;
mod vertex;
mod obj;
mod color;
mod fragment;
mod shaders;
mod camera;

use framebuffer::Framebuffer;
use vertex::Vertex;
use obj::Obj;
use camera::Camera;
use triangle::triangle;
use shaders::{vertex_shader, fragment_shader};
use fastnoise_lite::{FastNoiseLite, NoiseType, FractalType};
use image::{open, DynamicImage};

pub struct Uniforms {
    model_matrix: Mat4,
    view_matrix: Mat4,
    projection_matrix: Mat4,
    viewport_matrix: Mat4,
    time: u32,
    noise: FastNoiseLite
}

fn create_noise() -> FastNoiseLite {
    create_cloud_noise()
}

fn create_cloud_noise() -> FastNoiseLite {
    let mut noise = FastNoiseLite::with_seed(1337);
    noise.set_noise_type(Some(NoiseType::OpenSimplex2));
    noise
}

fn create_model_matrix(translation: Vec3, scale: f32, rotation: Vec3) -> Mat4 {
    let (sin_x, cos_x) = rotation.x.sin_cos();
    let (sin_y, cos_y) = rotation.y.sin_cos();
    let (sin_z, cos_z) = rotation.z.sin_cos();

    let rotation_matrix_x = Mat4::new(
        1.0,  0.0,    0.0,   0.0,
        0.0,  cos_x, -sin_x, 0.0,
        0.0,  sin_x,  cos_x, 0.0,
        0.0,  0.0,    0.0,   1.0,
    );

    let rotation_matrix_y = Mat4::new(
        cos_y,  0.0,  sin_y, 0.0,
        0.0,    1.0,  0.0,   0.0,
        -sin_y, 0.0,  cos_y, 0.0,
        0.0,    0.0,  0.0,   1.0,
    );

    let rotation_matrix_z = Mat4::new(
        cos_z, -sin_z, 0.0, 0.0,
        sin_z,  cos_z, 0.0, 0.0,
        0.0,    0.0,  1.0, 0.0,
        0.0,    0.0,  0.0, 1.0,
    );

    let rotation_matrix = rotation_matrix_z * rotation_matrix_y * rotation_matrix_x;

    let transform_matrix = Mat4::new(
        scale, 0.0,   0.0,   translation.x,
        0.0,   scale, 0.0,   translation.y,
        0.0,   0.0,   scale, translation.z,
        0.0,   0.0,   0.0,   1.0,
    );

    transform_matrix * rotation_matrix
}


fn create_view_matrix(eye: Vec3, center: Vec3, up: Vec3) -> Mat4 {
    look_at(&eye, &center, &up)
}

fn create_perspective_matrix(window_width: f32, window_height: f32) -> Mat4 {
    let fov = 45.0 * PI / 180.0;
    let aspect_ratio = window_width / window_height;
    let near = 0.1;
    let far = 1000.0;

    perspective(fov, aspect_ratio, near, far)
}

fn create_viewport_matrix(width: f32, height: f32) -> Mat4 {
    Mat4::new(
        width / 2.0, 0.0, 0.0, width / 2.0,
        0.0, -height / 2.0, 0.0, height / 2.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0
    )
}

fn render(framebuffer: &mut Framebuffer, uniforms: &Uniforms, vertex_array: &[Vertex], current_shader: u8) {
    let mut transformed_vertices = Vec::with_capacity(vertex_array.len());
    for vertex in vertex_array {
        let transformed = vertex_shader(vertex, uniforms);
        transformed_vertices.push(transformed);
    }

    let mut triangles = Vec::new();
    for i in (0..transformed_vertices.len()).step_by(3) {
        if i + 2 < transformed_vertices.len() {
            triangles.push([
                transformed_vertices[i].clone(),
                transformed_vertices[i + 1].clone(),
                transformed_vertices[i + 2].clone(),
            ]);
        }
    }

    let mut fragments = Vec::new();
    for tri in &triangles {
        fragments.extend(triangle(&tri[0], &tri[1], &tri[2]));
    }

    for fragment in fragments {
        let x = fragment.position.x as usize;
        let y = fragment.position.y as usize;

        if x < framebuffer.width && y < framebuffer.height {
            let shaded_color = fragment_shader(&fragment, uniforms, current_shader);
            let color = shaded_color.to_hex();
            framebuffer.set_current_color(color);
            framebuffer.point(x, y, fragment.depth);
        }
    }
}


fn main() {
    let window_width = 800;
    let window_height = 600;
    let framebuffer_width = 800;
    let framebuffer_height = 600;
    let frame_delay = Duration::from_millis(16);

    let mut framebuffer = Framebuffer::new(framebuffer_width, framebuffer_height);
    let mut window = Window::new(
        "Sistema Solar",
        window_width,
        window_height,
        WindowOptions::default(),
    )
    .unwrap();

    window.set_position(500, 500);
    window.update();

    // Cargar la imagen del espacio
    let space_texture = load_texture("assets/textures/Sky.png");

    let mut camera = Camera::new(
        Vec3::new(0.0, 0.0, 20.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );

    let sphere = Obj::load("assets/models/sphere.obj").expect("Failed to load sphere.obj");
    let vertex_arrays = sphere.get_vertex_array();

    let mut time = 0;

    let planet_data = vec![
        (Vec3::new(0.0, 0.0, 0.0), 2.0, 6, 0.0, 0.0),
        (Vec3::new(3.0, 0.0, 0.0), 0.5, 1, 0.05, 0.02),
        (Vec3::new(6.0, 0.0, 0.0), 0.7, 2, 0.03, 0.015),
        (Vec3::new(9.0, 0.0, 0.0), 0.9, 3, 0.02, 0.01),
        (Vec3::new(12.0, 0.0, 0.0), 1.2, 4, 0.01, 0.007),
        (Vec3::new(15.0, 0.0, 0.0), 1.5, 5, 0.04, 0.005),
        (Vec3::new(18.0, 0.0, 0.0), 1.7, 7, 0.02, 0.003),
        (Vec3::new(21.0, 0.0, 0.0), 1.8, 8, 0.03, 0.002),
    ];

    while window.is_open() {
        if window.is_key_down(Key::Escape) {
            break;
        }

        time += 1;

        handle_input(&window, &mut camera, &mut 0);

        framebuffer.clear();

        render_background(&mut framebuffer, &space_texture);

        let view_matrix = create_view_matrix(camera.eye, camera.center, camera.up);
        let projection_matrix = create_perspective_matrix(window_width as f32, window_height as f32);
        let viewport_matrix = create_viewport_matrix(framebuffer_width as f32, framebuffer_height as f32);

        for (translation, scale, shader, rotation_speed, orbital_speed) in &planet_data {
            let self_rotation = Vec3::new(0.0, time as f32 * rotation_speed, 0.0);

            let angle = time as f32 * orbital_speed;
            let orbital_translation = Vec3::new(
                translation.x * angle.cos() - translation.z * angle.sin(),
                translation.y,
                translation.x * angle.sin() + translation.z * angle.cos(),
            );

            let model_matrix = create_model_matrix(orbital_translation, *scale, self_rotation);
            let uniforms = Uniforms {
                model_matrix,
                view_matrix,
                projection_matrix,
                viewport_matrix,
                time,
                noise: create_noise(),
            };

            render(&mut framebuffer, &uniforms, &vertex_arrays, *shader);
        }

        window
            .update_with_buffer(&framebuffer.buffer, framebuffer_width, framebuffer_height)
            .unwrap();

        std::thread::sleep(frame_delay);
    }
}

fn load_texture(path: &str) -> DynamicImage {
    open(path).expect("Failed to load texture")
}

fn render_background(framebuffer: &mut Framebuffer, texture: &DynamicImage) {
    let texture = texture.to_rgb8();
    let (texture_width, texture_height) = texture.dimensions();

    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            let tx = (x as u32 * texture_width / framebuffer.width as u32) as u32;
            let ty = (y as u32 * texture_height / framebuffer.height as u32) as u32;

            let pixel = texture.get_pixel(tx, ty);
            let color = (pixel[0] as u32) << 16 | (pixel[1] as u32) << 8 | (pixel[2] as u32);

            framebuffer.set_current_color(color);
            framebuffer.point(x, y, 1.0);
        }
    }
}



fn handle_input(window: &Window, camera: &mut Camera, current_shader: &mut u8) {
    let movement_speed = 1.0;
    let rotation_speed = PI / 50.0;
    let zoom_speed = 0.1;

    if window.is_key_down(Key::Key0) {
        *current_shader = 0;
    }
    if window.is_key_down(Key::Key1) {
        *current_shader = 1;
    }
    if window.is_key_down(Key::Key2) {
        *current_shader = 2;
    }
    if window.is_key_down(Key::Key3) {
        *current_shader = 3;
    }
    if window.is_key_down(Key::Key4) {
        *current_shader = 4;
    }
    if window.is_key_down(Key::Key5) {
        *current_shader = 5;
    }
    if window.is_key_down(Key::Key6) {
        *current_shader = 6;
    }
    if window.is_key_down(Key::Key7) {
        *current_shader = 7;
    }
    if window.is_key_down(Key::Key8) {
        *current_shader = 8;
    }
    if window.is_key_down(Key::Key9) {
        *current_shader = 9;
    }

   
    //  camera orbit controls
    if window.is_key_down(Key::Left) {
      camera.orbit(rotation_speed, 0.0);
    }
    if window.is_key_down(Key::Right) {
      camera.orbit(-rotation_speed, 0.0);
    }
    if window.is_key_down(Key::W) {
      camera.orbit(0.0, -rotation_speed);
    }
    if window.is_key_down(Key::S) {
      camera.orbit(0.0, rotation_speed);
    }

    // Camera movement controls
    let mut movement = Vec3::new(0.0, 0.0, 0.0);
    if window.is_key_down(Key::A) {
      movement.x -= movement_speed;
    }
    if window.is_key_down(Key::D) {
      movement.x += movement_speed;
    }
    if window.is_key_down(Key::Q) {
      movement.y += movement_speed;
    }
    if window.is_key_down(Key::E) {
      movement.y -= movement_speed;
    }
    if movement.magnitude() > 0.0 {
      camera.move_center(movement);
    }

    // Camera zoom controls
    if window.is_key_down(Key::Up) {
      camera.zoom(zoom_speed);
    }
    if window.is_key_down(Key::Down) {
      camera.zoom(-zoom_speed);
    }
}