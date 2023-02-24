use {
bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    prelude::*,
    render::view::NoFrustumCulling,
},

instancing::{
    CustomMaterialPlugin, 
    InstanceData, 
    InstanceMaterialData
},
bevy_tasks::TaskPool,
bevy::time::FixedTimestep,
rand::{thread_rng, Rng, distributions::Distribution},
};

pub mod instancing;

type CellLocations = [bool; CELL_LOCATIONS_SIZE];

const GAME_SIZE: f32 = 50.;
const CELL_LOCATIONS_SIZE: usize = (GAME_SIZE * GAME_SIZE * GAME_SIZE) as usize;
const CELL_SIZE: f32 = 1.;

struct GameRule {
    survival: [bool; 27],
    spawn: [bool; 27],
}

impl GameRule {
    pub fn default() -> Self {
        let survival = Self::convert_to_dense_array(&[5, 6, 7, 8]);
        let spawn = Self::convert_to_dense_array(&[6,7,9]);
        GameRule {
            survival,
            spawn,
        }
    }

    pub fn convert_to_dense_array(vc: &[u8]) -> [bool; 27] {
        let mut ar = [false; 27];
        for i in vc {
            ar[*i as usize] = true;
        }
        ar
    }
}

fn main() {
    let cell_locations: CellLocations = [false; CELL_LOCATIONS_SIZE];
    let game_rule: GameRule = GameRule::default();
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(CustomMaterialPlugin)
        .insert_resource(TaskPool::new())
        .add_startup_system(initialize_game)
        .add_system(update_cell_locations)
        .add_system(create_new_cells)
        .insert_resource(cell_locations)
        .insert_resource(game_rule)
        .run();
}

fn convert_loc_to_index(x: f32, y: f32, z: f32) -> usize {
    let x = ((x / CELL_SIZE).floor() * CELL_SIZE) + (GAME_SIZE / 2.);
    let y = ((y / CELL_SIZE).floor() * CELL_SIZE) + (GAME_SIZE / 2.);
    let z = ((z / CELL_SIZE).floor() * CELL_SIZE) + (GAME_SIZE / 2.);
    (x + GAME_SIZE * y + GAME_SIZE * GAME_SIZE * z) as usize
}

fn convert_index_to_loc(index: usize) -> (f32, f32, f32) {
    let i = index as f32;
    let x = i % GAME_SIZE - (GAME_SIZE / 2.);
    let y = (i / GAME_SIZE).floor() % GAME_SIZE - (GAME_SIZE / 2.);
    let z = (i / (GAME_SIZE * GAME_SIZE)).floor() - (GAME_SIZE / 2.);
    (x, y, z)
}

fn count_neighbors(index: i32, cell_locations: &ResMut<CellLocations>) -> i32 {
    let loc = convert_index_to_loc(index as usize);
    let mut count = 0;
    for x in &[        
        (-1., -1., -1.), (0., -1., -1.), (1., -1., -1.), (-1., 0., -1.),        
        (0., 0., -1.), (1., 0., -1.), (-1., 1., -1.), (0., 1., -1.),
        (1., 1., -1.), (-1., -1., 0.), (0., -1., 0.), (1., -1., 0.),        
        (-1., 0., 0.), (1., 0., 0.), (-1., 1., 0.), (0., 1., 0.),        
        (1., 1., 0.), (-1., -1., 1.), (0., -1., 1.), (1., -1., 1.),        
        (-1., 0., 1.), (0., 0., 1.), (1., 0., 1.), (-1., 1., 1.), (0., 1., 1.), (1., 1., 1.)    ] {
        let new_loc = (
            loc.0 + x.0,
            loc.1 + x.1,
            loc.2 + x.2,
        );
        if new_loc.0.abs() >= (GAME_SIZE / 2.) || new_loc.1.abs() >= (GAME_SIZE / 2.) || new_loc.2.abs() >= (GAME_SIZE / 2.) {
            continue;
        }
        let new_index = convert_loc_to_index(new_loc.0, new_loc.1, new_loc.2);
        count += cell_locations[new_index] as i32;
    }
    count
}


fn update_cell_locations(
    mut cell_locations: ResMut<CellLocations>,
    task_pool: Res<TaskPool>,
    game_rule: Res<GameRule>,
) {
    let mut alive: Vec<usize> = Vec::new();
    let mut alive2: Vec<usize> = Vec::new();
    let mut dead: Vec<usize> = Vec::new();

    task_pool.scope(|s| {
        let task1 = s.spawn(async {
            for i in 0..CELL_LOCATIONS_SIZE {
                if cell_locations[i] {
                    let neighbors = count_neighbors(i as i32, &cell_locations);
                    if game_rule.survival[neighbors as usize] {
                        alive.push(i);
                    } else {
                        dead.push(i);
                    }
                }
            }
        });

        let task2 = s.spawn(async {
            for i in 0..CELL_LOCATIONS_SIZE {
                if !cell_locations[i] {
                    let neighbors = count_neighbors(i as i32, &cell_locations);
                    if game_rule.spawn[neighbors as usize] {
                        alive2.push(i);
                    }
                }
            }
        });
    });

    alive.extend(&alive2);

    for i in &alive {
        cell_locations[*i] = true;
    }
    for i in &dead {
        cell_locations[*i] = false;
    }
}



fn create_new_cells(
    cell_locations: Res<CellLocations>,
    game_rule: Res<GameRule>,
    mut q_instances: Query<&mut InstanceMaterialData>,
) {
    let mut rng = thread_rng();
    let mut instances = q_instances.get_single_mut().expect("Query returned None");
    let x: Vec<InstanceData> = cell_locations
        .iter()
        .enumerate()
        .filter(|(_, x)| **x)
        .map(|(index, _)| {
            let loc = convert_index_to_loc(index);
            let distance = loc.0.abs().max(loc.1.abs()).max(loc.2.abs()) / (GAME_SIZE / 2.);
            let r =
                (1. - distance) * rng.gen::<f32>() + distance * rng.gen::<f32>();
            let g =
                (1. - distance) * rng.gen::<f32>() + distance * rng.gen::<f32>();
            let b =
                (1. - distance) * rng.gen::<f32>() + distance * rng.gen::<f32>();
            InstanceData {
                position: Vec3::new(loc.0, loc.1, loc.2),
                scale: 1.,
                color: [r,g,b,1.0],
            }
        })
        .collect();
    *instances = InstanceMaterialData(x);
}


fn create_random_spawn_points(
    points: i32,
    center: (i32, i32, i32),
    distance: i32,
) -> Vec<(f32, f32, f32)> {
    let x_start = match center.0 - (distance / 2) as i32 {
        x if x < -(GAME_SIZE / 2.) as i32 => -(GAME_SIZE / 2.) as i32,
        x if x > (GAME_SIZE / 2.) as i32 => (GAME_SIZE / 2.) as i32,
        x => x,
    };
    let y_start = match center.1 - (distance / 2) as i32 {
        y if y < -(GAME_SIZE / 2.) as i32 => -(GAME_SIZE / 2.) as i32,
        y if y > (GAME_SIZE / 2.) as i32 => (GAME_SIZE / 2.) as i32,
        y => y,
    };
    let z_start = match center.2 - (distance / 2) as i32 {
        z if z < -(GAME_SIZE / 2.) as i32 => -(GAME_SIZE / 2.) as i32,
        z if z > (GAME_SIZE / 2.) as i32 => (GAME_SIZE / 2.) as i32,
        z => z,
    };
    let mut rng = rand::thread_rng();
    let x_distro = rand::distributions::Uniform::from(x_start..(x_start + distance));
    let y_distro = rand::distributions::Uniform::from(y_start..(y_start + distance));
    let z_distro = rand::distributions::Uniform::from(z_start..(z_start + distance));
    
    x_distro
    .sample_iter(&mut rng.clone())
    .zip(y_distro.sample_iter(&mut rng.clone()))
    .zip(z_distro.sample_iter(&mut rng))
    .map(|((x, y), z)| (x as f32, y as f32, z as f32))
    .take(points as usize)
    .collect()
    
}

fn initialize_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cell_locations: ResMut<CellLocations>,
    task_pool: Res<TaskPool>,
) {

    commands
    .spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(-50., 0., 80.).looking_at(Vec3::ZERO, Vec3::Y),
        camera_3d: Camera3d {
            clear_color: ClearColorConfig::Custom(Color::rgb(0., 0., 0.)),
            ..default()
        },
        ..default()
    });

        task_pool.scope(|s| {
            let originTask = s.spawn(async {
                commands.spawn()
                    .insert_bundle((
                        meshes.add(Mesh::from(shape::Cube { size: CELL_SIZE })),
                        Transform::from_xyz(0.0, 0.0, 0.0),
                        GlobalTransform::default(),
                        InstanceMaterialData(Vec::new()),
                        Visibility::default(),
                        ComputedVisibility::default(),
                        NoFrustumCulling,
                    ));
            });

            let randomTask = s.spawn(async {
                for t in create_random_spawn_points(900, (0, 0, 0), 20) {
                    let index = convert_loc_to_index(t.0, t.1, t.2);
                    cell_locations[index] = true;
                }
            });
        });
}
