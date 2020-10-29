use glium::{Surface};
use glium::*;
use glium::backend::Facade;
use glutin::{event_loop::EventLoop, event::{Event, WindowEvent, VirtualKeyCode, KeyboardInput, DeviceEvent}};
use glutin::event_loop::ControlFlow;
use std::sync::{Arc, Mutex};
use glutin::platform::desktop::EventLoopExtDesktop;
use SmolECS::{
    component::*,
    entity::*,
    system::*,
    rayon::*,
    world::*,
};
use std::collections::HashSet;
use rand::prelude::*;
use std::io::Cursor;

#[derive(Copy, Clone)]
pub struct Player;

#[derive(Copy, Clone)]
pub struct Enemy;

#[derive(Copy, Clone)]
pub struct Asteroid;

#[derive(Copy, Clone)]
pub struct Bullet;

#[derive(Copy, Clone)]
pub struct Velocity{
    x: f32,
    y: f32,
}

#[derive(Copy, Clone)]
pub struct Position{
    x: f32, 
    y: f32
}

#[derive(Copy, Clone)]
pub struct Rotation(f32);

#[derive(Copy, Clone)]
pub struct RotationVelocity(f32);

#[derive(Copy, Clone)]
pub struct Radius(f32);

#[derive(Copy, Clone)]
pub struct Health(isize);

pub struct WorldBounds{
    x: f32, 
    y: f32
}

pub struct Time{
    beginning: std::time::Instant,
    last: std::time::Instant,
    total: f64,
    delta: f64
}

pub struct ControlInputs{
    pressed: HashSet<VirtualKeyCode>,
    down: HashSet<VirtualKeyCode>
}

#[derive(Debug)]
enum KeyStatus{
    Up,
    Pressed,
    Down,
}

impl std::fmt::Display for KeyStatus{
        
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self{
            KeyStatus::Up => {write!(f, "{:?}", self)}
            KeyStatus::Pressed => {write!(f, "{:?}", self)}
            KeyStatus::Down => {write!(f, "{:?}", self)}
        }
    }
}

impl ControlInputs{
    fn key_status(&self, code: VirtualKeyCode) -> KeyStatus{
        if self.pressed.contains(&code){
            return KeyStatus::Pressed;
        }
        if self.down.contains(&code){
            return KeyStatus::Down;
        }
        KeyStatus::Up
    }
}

// SYSTEMS
pub struct UpdateTime;
impl<'d, 'w: 'd> System<'d, 'w, World> for UpdateTime{
    type SystemData = (
        Write<'d, Time>
    );

    fn run(&self, (mut time): Self::SystemData) {
        let current = std::time::Instant::now();
        time.delta = current.duration_since(time.last).as_secs_f64();
        time.total = current.duration_since(time.beginning).as_secs_f64();
        time.last = current;
    }
}

pub struct ApplyVelocities;
impl<'d, 'w: 'd> System<'d, 'w, World> for ApplyVelocities{
    type SystemData = (
        ReadComp<'d, Velocity>,
        Read<'d, Time>,
        WriteComp<'d, Position>
    );

    fn run(&self, (vels, time, mut positions): Self::SystemData) {
        for (vel, position) in (&vels, &mut positions).join(){
            position.x += vel.x * time.delta as f32;
            position.y += vel.y * time.delta as f32;

            if position.x < -10.5{
                position.x = 21.0 - position.x.abs();
            } else if position.x > 10.5{
                position.x = -21.0 + position.x.abs();
            }
            
            if position.y < -10.5{
                position.y = 21.0 - position.y.abs();
            } else if position.y > 10.5{
                position.y = -21.0 + position.y.abs();
            }
        }
    }
}

pub struct ApplyRotationVelocities;
impl<'d, 'w: 'd> System<'d, 'w, World> for ApplyRotationVelocities{
    type SystemData = (
        ReadComp<'d, RotationVelocity>,
        Read<'d, Time>,
        WriteComp<'d, Rotation>
    );

    fn run(&self, (vels, time, mut rots): Self::SystemData) {
        for (vel, rot) in (&vels, &mut rots).join(){
            rot.0 += vel.0 * time.delta as f32;
            rot.0 = rot.0.signum() * rot.0.abs() % 360.0;
        }
    }
}

pub struct ApplyControls;
impl<'d, 'w: 'd> System<'d, 'w, World> for ApplyControls{
    type SystemData = (
        WriteComp<'d, RotationVelocity>,
        WriteComp<'d, Velocity>,
        WriteComp<'d, Position>,
        WriteComp<'d, Radius>,
        WriteComp<'d, Bullet>,
        WriteComp<'d, Health>,
        ReadComp<'d, Rotation>,
        ReadComp<'d, Player>,
        ReadComp<'d, Enemy>,
        Read<'d, ControlInputs>,
        Read<'d, Time>,
        Write<'d, EntityStorage>,
    );

    fn run(&self, (mut a_vels, mut vels, mut positions, mut radii, mut bullets, mut healths, rots, players, enemies, inputs, time, mut ents): Self::SystemData) {
        let mut new_bullet_position = None;
        for (vel, a_vel, rot, player, position) in (&mut vels, &mut a_vels, &rots, &players, &positions).join(){
            let mut turn_val = 0.0;
            match inputs.key_status(VirtualKeyCode::A){
                KeyStatus::Up => {},
                _ => {turn_val += 180.0;},
            }
            match inputs.key_status(VirtualKeyCode::D){
                KeyStatus::Up => {},
                _ => {turn_val -= 180.0;},
            }
            if turn_val == 0.0 && a_vel.0.abs() != 0.0{
                turn_val = -a_vel.0.signum() * 180.0;
            }
            a_vel.0 += turn_val * time.delta as f32;

            let mut forward_val = 0.0;
            match inputs.key_status(VirtualKeyCode::W){
                KeyStatus::Up => {},
                _ => {forward_val += 1.0},
            }
            let direction = (
                (-rot.0 * std::f32::consts::PI/180.0).sin() ,
                (rot.0 * std::f32::consts::PI/180.0).cos()
            );
            vel.x += forward_val * direction.0 * time.delta as f32;
            vel.y += forward_val * direction.1 * time.delta as f32;

            new_bullet_position = match inputs.key_status(VirtualKeyCode::S){
                KeyStatus::Pressed => {
                    Some((Position{
                        x: position.x + direction.0 * 1.0,
                        y: position.y + direction.1 * 1.0,
                    },
                    Velocity{
                        x: vel.x + direction.0 * 10.0,
                        y: vel.y + direction.1 * 10.0,
                    }
                    ))
                },
                _ => None,
            }
        }
        if let Some((pos, vel)) = new_bullet_position{
            ents.create_entity()
                .add(&mut positions, pos)
                .add(&mut vels, vel)
                .add(&mut bullets, Bullet{})
                .add(&mut radii, Radius(0.25))
                .add(&mut healths, Health(1));
        }

        
        let mut new_bullet_position = None;
        for (vel, a_vel, rot, enemy, position) in (&mut vels, &mut a_vels, &rots, &enemies, &positions).join(){
            let mut turn_val = 0.0;
            match inputs.key_status(VirtualKeyCode::Left){
                KeyStatus::Up => {},
                _ => {turn_val += 180.0;},
            }
            match inputs.key_status(VirtualKeyCode::Right){
                KeyStatus::Up => {},
                _ => {turn_val -= 180.0;},
            }
            if turn_val == 0.0 && a_vel.0.abs() != 0.0{
                turn_val = -a_vel.0.signum() * 180.0;
            }
            a_vel.0 += turn_val * time.delta as f32;

            let mut forward_val = 0.0;
            match inputs.key_status(VirtualKeyCode::Up){
                KeyStatus::Up => {},
                _ => {forward_val = 1.0},
            }
            let direction = (
                (-rot.0 * std::f32::consts::PI/180.0).sin() ,
                (rot.0 * std::f32::consts::PI/180.0).cos()
            );
            vel.x += forward_val * direction.0 * time.delta as f32;
            vel.y += forward_val * direction.1 * time.delta as f32;

            new_bullet_position = match inputs.key_status(VirtualKeyCode::Down){
                KeyStatus::Pressed => {
                    Some((Position{
                        x: position.x + direction.0 * 1.0,
                        y: position.y + direction.1 * 1.0,
                    },
                    Velocity{
                        x: vel.x + direction.0 * 10.0,
                        y: vel.y + direction.1 * 10.0,
                    }
                    ))
                },
                _ => None,
            }
        }
        if let Some((pos, vel)) = new_bullet_position{
            ents.create_entity()
                .add(&mut positions, pos)
                .add(&mut vels, vel)
                .add(&mut bullets, Bullet{})
                .add(&mut radii, Radius(0.25))
                .add(&mut healths, Health(1));
        }
    }
}

fn collision_check(rad_one: &Radius, pos_one: &Position, rad_two: &Radius, pos_two: &Position) -> bool{
    (pos_two.x - pos_one.x).powi(2) + (pos_two.y - pos_one.y).powi(2) <= (rad_one.0 + rad_two.0).powi(2)
}

use std::ops::Deref;
pub struct DamagerCollisionCheck;
impl<'d, 'w: 'd> System<'d, 'w, World> for DamagerCollisionCheck{
    type SystemData = (
        ReadComp<'d, Radius>,
        ReadComp<'d, Position>,
        WriteComp<'d, Health>,
        Read<'d, EntityStorage>,
    );

    fn run(&self, (radii, positions, mut healths, ents): Self::SystemData) {
        //Check Bullet Collisions
        for (pos_one, rad_one, health, ent_one) in (&positions, &radii, &mut healths, ents.deref()).join(){
            for(pos_two, rad_two, ent_two) in (&positions, &radii, ents.deref()).join(){
                if collision_check(rad_one, pos_one, rad_two, pos_two) && ent_one != ent_two{
                    health.0 -= 1;
                    break;
                }
            }
        }
    }
}

pub struct DestroyZeroHealth;
impl<'d, 'w: 'd> System<'d, 'w, World> for DestroyZeroHealth{
    type SystemData = (
        WriteComp<'d, Player>,
        WriteComp<'d, Enemy>,
        WriteComp<'d, Velocity>,
        WriteComp<'d, Position>,
        WriteComp<'d, Radius>,
        WriteComp<'d, Health>,
        WriteComp<'d, Rotation>,
        WriteComp<'d, RotationVelocity>,
        WriteComp<'d, Asteroid>,
        WriteComp<'d, Bullet>,
        Write<'d, EntityStorage>,
    );

    fn run(&self, (mut players, mut enemies, mut velocities, mut positions, mut radii, mut healths, mut rotations, mut rotationvels, mut asteroids, mut bullets, mut ents): Self::SystemData) {
        
        let mut bullets_to_delete = Vec::new();
        for (bullet, health, entity) in (&bullets, &healths, ents.deref()).join(){
            if health.0 <= 0{
                bullets_to_delete.push(entity.clone());
            }
        }
        for bullet in bullets_to_delete.drain(..){
            bullet
                .remove(&mut healths)
                .remove(&mut positions)
                .remove(&mut velocities)
                .remove(&mut radii)
                .remove(&mut bullets);
            ents.delete_entity(&bullet);
        }
        
        let mut asteroids_to_delete = Vec::new();
        for (asteroid, health, entity) in (&asteroids, &healths, ents.deref()).join(){
            if health.0 <= 0{
                asteroids_to_delete.push(entity.clone());
            }
        }
        for asteroid in asteroids_to_delete.drain(..){
            asteroid
                .remove(&mut healths)
                .remove(&mut positions)
                .remove(&mut velocities)
                .remove(&mut radii)
                .remove(&mut rotations)
                .remove(&mut rotationvels)
                .remove(&mut asteroids);
                ents.delete_entity(&asteroid);
        }
        
        let mut players_to_delete = Vec::new();
        for (player, health, entity) in (&players, &healths, ents.deref()).join(){
            if health.0 <= 0{
                players_to_delete.push(entity.clone());
            }
        }
        for player in players_to_delete.drain(..){
            player
                .remove(&mut players)
                .remove(&mut healths)
                .remove(&mut positions)
                .remove(&mut velocities)
                .remove(&mut radii)
                .remove(&mut rotations)
                .remove(&mut rotationvels);
                ents.delete_entity(&player);
        }
        
        let mut enemies_to_delete = Vec::new();
        for (enemy, health, entity) in (&enemies, &healths, ents.deref()).join(){
            if health.0 <= 0{
                enemies_to_delete.push(entity.clone());
            }
        }
        for enemy in enemies_to_delete.drain(..){
            enemy
                .remove(&mut players)
                .remove(&mut healths)
                .remove(&mut positions)
                .remove(&mut velocities)
                .remove(&mut radii)
                .remove(&mut rotations)
                .remove(&mut rotationvels);
                ents.delete_entity(&enemy);
        }
    }
}

//RENDER STUFF
#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
}
implement_vertex!(Vertex, position, uv);


fn main() {

    // Glium and Glutin setup
    let mut el = glutin::event_loop::EventLoop::new();

    let wb = glutin::window::WindowBuilder::new()
        .with_title("SmolShooter")
        .with_inner_size(glutin::dpi::LogicalSize::new(640.0, 640.0))
        .with_resizable(false);

    let windowed_context = glutin::ContextBuilder::new();
        //.build_windowed(wb, &el)
        //.unwrap();


    //let (context, window) = unsafe{windowed_context.split()};

    // Rendering setup
    let renderer = glium::Display::new(wb, windowed_context, &el).unwrap();

    let quad = vec![
        Vertex{
            position: [-0.5, -0.5],
            uv: [0.0, 0.0]
        },
        Vertex{
            position: [0.5, -0.5],
            uv: [1.0, 0.0]
        },
        Vertex{
            position: [0.5, 0.5],
            uv: [1.0, 1.0]
        },
        Vertex{
            position: [-0.5, 0.5],
            uv: [0.0, 1.0]
        },
    ];
    let triangles: Vec<u32> = vec![0, 1, 2, 2, 3, 0];

    let vertex_buffer = VertexBuffer::new(&renderer, &quad).unwrap();
    let indicies = IndexBuffer::new(&renderer, index::PrimitiveType::TrianglesList,&triangles).unwrap();

    let vert = include_bytes!("./shaders/vert.vert");
    let frag = include_bytes!("./shaders/frag.frag");
    let program = Program::from_source(
        &renderer, 
        std::str::from_utf8(vert).unwrap(), 
    std::str::from_utf8(frag).unwrap(), 
    None
    ).unwrap();

    // GET PLAYER TEXTURE
    let image = image::load(Cursor::new(&include_bytes!("./assets/player.png")[..]),
                        image::ImageFormat::Png).unwrap().to_rgba();
    let image_dimensions = image.dimensions();
    let image = glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
    let player_tex = glium::texture::Texture2d::new(&renderer, image).unwrap();
    
    // GET ENEMY TEXTURE
    let image = image::load(Cursor::new(&include_bytes!("./assets/enemy.png")[..]),
                        image::ImageFormat::Png).unwrap().to_rgba();
    let image_dimensions = image.dimensions();
    let image = glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
    let enemy_tex = glium::texture::Texture2d::new(&renderer, image).unwrap();

    // GET ASTEROID TEXTURE
    let image = image::load(Cursor::new(&include_bytes!("./assets/asteroid.png")[..]),
    image::ImageFormat::Png).unwrap().to_rgba();
    let image_dimensions = image.dimensions();
    let image = glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
    let asteroid_tex = glium::texture::Texture2d::new(&renderer, image).unwrap();

    
    // GET BULLET TEXTURE
    let image = image::load(Cursor::new(&include_bytes!("./assets/bullet.png")[..]),
    image::ImageFormat::Png).unwrap().to_rgba();
    let image_dimensions = image.dimensions();
    let image = glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
    let bullet_tex = glium::texture::Texture2d::new(&renderer, image).unwrap();
    
    let params = glium::DrawParameters{
        blend: glium::draw_parameters::Blend::alpha_blending(),
        .. Default::default()
    };


    // SmolECS Setup
    let mut world = World::new();
    world.register_comp::<Player>();
    world.register_comp::<Enemy>();
    world.register_comp::<Velocity>();
    world.register_comp::<Position>();
    world.register_comp::<Radius>();
    world.register_comp::<Health>();
    world.register_comp::<Rotation>();
    world.register_comp::<RotationVelocity>();
    world.register_comp::<Asteroid>();
    world.register_comp::<Bullet>();

    world.insert(WorldBounds{x: 10.0, y: 10.0});
    world.insert(Time{
        beginning: std::time::Instant::now(),
        last: std::time::Instant::now(),
        total: 0.0,
        delta: 0.0,
    });
    world.insert(ControlInputs{pressed: HashSet::new(), down: HashSet::new()});
    world.insert(EntityStorage::new());
    
    let mut ents = Write::<EntityStorage>::get_data(&world);
    let mut players = WriteComp::<Player>::get_data(&world);
    let mut enemies = WriteComp::<Enemy>::get_data(&world);
    let mut health = WriteComp::<Health>::get_data(&world);
    let mut positions = WriteComp::<Position>::get_data(&world);
    let mut vels = WriteComp::<Velocity>::get_data(&world);
    let mut radius = WriteComp::<Radius>::get_data(&world);
    let mut angles = WriteComp::<Rotation>::get_data(&world);
    let mut angle_vel = WriteComp::<RotationVelocity>::get_data(&world);
    let mut asteroids = WriteComp::<Asteroid>::get_data(&world);
    
    //Make the player
    ents.create_entity()
        .add(&mut players, Player{})
        .add(&mut health, Health(5))
        .add(&mut positions, Position{x: 0.0, y: -9.5})
        .add(&mut vels, Velocity{x: 0.0, y: 0.0})
        .add(&mut radius, Radius(0.5))
        .add(&mut angles, Rotation(0.0))
        .add(&mut angle_vel, RotationVelocity(0.0));

        
    //Make the enemies
    ents.create_entity()
        .add(&mut enemies, Enemy{})
        .add(&mut health, Health(5))
        .add(&mut positions, Position{x: 0.0, y: 9.5})
        .add(&mut vels, Velocity{x: 0.0, y: 0.0})
        .add(&mut radius, Radius(0.5))
        .add(&mut angles, Rotation(180.0))
        .add(&mut angle_vel, RotationVelocity(0.0));


    let mut rng = rand::thread_rng();
    for i in 0..30{
        ents.create_entity()
            .add(&mut health, Health(1))
            .add(&mut positions, Position{x: rng.gen_range(-10.0, 10.0), y: rng.gen_range(-7.0, 7.0)})
            .add(&mut vels, Velocity{x: rng.gen_range(-2.0, 2.0), y: rng.gen_range(-2.0, 2.0)})
            .add(&mut radius, Radius(0.5))
            .add(&mut angles, Rotation(rng.gen_range(0.0, 360.0)))
            .add(&mut angle_vel, RotationVelocity(rng.gen_range(-90.0, 90.0)))
            .add(&mut asteroids, Asteroid{});
    }
    
    
    let mut scheduler = SystemScheduler::new(Arc::new(ThreadPoolBuilder::new().num_threads(4).build().unwrap()));
    scheduler.add(UpdateTime{}, "update_time", vec![]);
    scheduler.add(ApplyControls{}, "apply_controls", vec!["update_time"]);
    scheduler.add(ApplyVelocities{}, "update_positions", vec!["update_time", "apply_controls"]);
    scheduler.add(ApplyRotationVelocities{}, "update_angles", vec!["update_time", "apply_controls"]);
    scheduler.add(DamagerCollisionCheck{}, "damage_check", vec!["update_positions"]);
    scheduler.add(DestroyZeroHealth{}, "destroy_zero", vec!["damage_check"]);

    let mut closed = false;

    drop(ents);
    drop(players);
    drop(enemies);
    drop(health);
    drop(positions);
    drop(vels);
    drop(radius);
    drop(angles);
    drop(angle_vel);
    drop(asteroids);

    // Main Loop
    loop{
        el.run_return(|event, _, control_flow| {
            *control_flow = ControlFlow::Poll;
        
            let mut keys = Write::<ControlInputs>::get_data(&world);
            let mut new_down = HashSet::new();
            for key in keys.pressed.drain(){
                new_down.insert(key);
            }

            for key in new_down.drain(){
                keys.down.insert(key);
            }

            match event {
                Event::WindowEvent {event: WindowEvent::CloseRequested, .. } => {
                    println!("The close button was pressed; stopping");
                    *control_flow = ControlFlow::Exit;
                    closed = true;
                    return;
                },
                Event::DeviceEvent {event: DeviceEvent::Key(KeyboardInput{virtual_keycode, state,  ..}, ..), ..} =>{
                    match virtual_keycode{
                        Some(key) => {
                            match state{
                                glutin::event::ElementState::Pressed => {
                                    keys.pressed.insert(key);
                                },
                                glutin::event::ElementState::Released => {
                                    keys.down.remove(&key);
                                },
                            }
                        },
                        None => {},
                    }
                },  
                _ => {}
            }
            drop(keys);

            scheduler.run(&world);
            
            let mut frame = renderer.draw();
            frame.clear_color(0.0, 0.0, 0.0, 0.0);
            
            let players = ReadComp::<Player>::get_data(&world);
            let enemies = ReadComp::<Enemy>::get_data(&world);
            let angles = ReadComp::<Rotation>::get_data(&world);
            let positions = ReadComp::<Position>::get_data(&world);
            let asteroids = ReadComp::<Asteroid>::get_data(&world);
            let bullets = ReadComp::<Bullet>::get_data(&world);


            for (asteroid, position, angle) in (&asteroids, &positions, &angles).join(){
                let uniform = uniform! {
                    p: [
                        [0.1, 0.0, 0.0, 0.0],
                        [0.0, 0.1, 0.0, 0.0],
                        [0.0, 0.0, -0.1, 0.0],
                        [0.0, 0.0, 0.0, 1.0_f32],
                    ],
                    pos: [position.x, position.y],
                    rots: [(angle.0/180.0 * std::f32::consts::PI).sin(), (angle.0/180.0 * std::f32::consts::PI).cos()],
                    tex: &asteroid_tex
                };
                frame.draw(
                    &vertex_buffer, 
                    &indicies,
                        &program, 
                    &uniform,
                    &params).unwrap();
            }

            for (player, position, angle) in (&players, &positions, &angles).join(){
                let uniform = uniform! {
                    p: [
                        [0.1, 0.0, 0.0, 0.0],
                        [0.0, 0.1, 0.0, 0.0],
                        [0.0, 0.0, -0.1, 0.0],
                        [0.0, 0.0, 0.0, 1.0_f32],
                    ],
                    pos: [position.x, position.y],
                    rots: [(angle.0/180.0 * std::f32::consts::PI).sin(), (angle.0/180.0 * std::f32::consts::PI).cos()],
                    tex: &player_tex
                };
                frame.draw(
                    &vertex_buffer, 
                    &indicies,
                        &program, 
                    &uniform,
                    &params).unwrap();
            }

            for (enemy, position, angle) in (&enemies, &positions, &angles).join(){
                let uniform = uniform! {
                    p: [
                        [0.1, 0.0, 0.0, 0.0],
                        [0.0, 0.1, 0.0, 0.0],
                        [0.0, 0.0, -0.1, 0.0],
                        [0.0, 0.0, 0.0, 1.0_f32],
                    ],
                    pos: [position.x, position.y],
                    rots: [(angle.0/180.0 * std::f32::consts::PI).sin(), (angle.0/180.0 * std::f32::consts::PI).cos()],
                    tex: &enemy_tex
                };
                frame.draw(
                    &vertex_buffer, 
                    &indicies,
                        &program, 
                    &uniform,
                    &params).unwrap();
            }

            for (bullet, position) in (&bullets, &positions).join(){
                let uniform = uniform! {
                    p: [
                        [0.1, 0.0, 0.0, 0.0],
                        [0.0, 0.1, 0.0, 0.0],
                        [0.0, 0.0, -0.1, 0.0],
                        [0.0, 0.0, 0.0, 1.0_f32],
                    ],
                    pos: [position.x, position.y],
                    rots: [1.0 as f32, 1.0  as f32],
                    tex: &bullet_tex
                };
                frame.draw(
                    &vertex_buffer, 
                    &indicies,
                        &program, 
                    &uniform,
                    &params).unwrap();
            }

            drop(players);
            drop(enemies);
            drop(angles);
            drop(bullets);
            drop(positions);
            drop(asteroids);

            frame.finish();
        });
        if closed{
            break;
        }
    }
}
