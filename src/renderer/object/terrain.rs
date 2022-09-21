use crate::core::*;
use crate::renderer::*;
use std::rc::Rc;

pub enum Lod {
    High,
    Medium,
    Low,
}

const VERTICES_PER_SIDE: usize = 33;

pub struct Terrain<M: Material> {
    context: Context,
    center: (i32, i32),
    patches: Vec<Gm<TerrainPatch, M>>,
    index_buffer1: Rc<ElementBuffer>,
    index_buffer4: Rc<ElementBuffer>,
    index_buffer16: Rc<ElementBuffer>,
    material: M,
    lod: Box<dyn Fn(f32) -> Lod>,
    height_map: Box<dyn Fn(f32, f32) -> f32>,
    side_length: f32,
    vertex_distance: f32,
}
impl<M: Material + Clone> Terrain<M> {
    pub fn new(
        context: &Context,
        material: M,
        height_map: Box<dyn Fn(f32, f32) -> f32>,
        side_length: f32,
        vertex_distance: f32,
        center: Vec2,
    ) -> Self {
        let index_buffer1 = Self::indices(context, 1);
        let mut patches = Vec::new();
        let (x0, y0) = pos2patch(vertex_distance, center);
        let half_patches_per_side = half_patches_per_side(vertex_distance, side_length);
        for ix in x0 - half_patches_per_side..x0 + half_patches_per_side + 1 {
            for iy in y0 - half_patches_per_side..y0 + half_patches_per_side + 1 {
                let patch = TerrainPatch::new(
                    context,
                    &height_map,
                    (ix, iy),
                    index_buffer1.clone(),
                    vertex_distance,
                );
                patches.push(Gm::new(patch, material.clone()));
            }
        }
        Self {
            context: context.clone(),
            center: (x0, y0),
            patches,
            index_buffer1,
            index_buffer4: Self::indices(context, 4),
            index_buffer16: Self::indices(context, 16),
            lod: Box::new(|_| Lod::High),
            material: material.clone(),
            height_map,
            side_length,
            vertex_distance,
        }
    }

    pub fn set_lod(&mut self, lod: Box<dyn Fn(f32) -> Lod>) {
        self.lod = lod;
    }

    pub fn set_center(&mut self, center: Vec2) {
        let (x0, y0) = pos2patch(self.vertex_distance, center);
        let half_patches_per_side = half_patches_per_side(self.vertex_distance, self.side_length);

        while x0 > self.center.0 {
            self.center.0 += 1;
            for iy in
                self.center.1 - half_patches_per_side..self.center.1 + half_patches_per_side + 1
            {
                self.patches.push(Gm::new(
                    TerrainPatch::new(
                        &self.context,
                        &self.height_map,
                        (self.center.0 + half_patches_per_side, iy),
                        self.index_buffer1.clone(),
                        self.vertex_distance,
                    ),
                    self.material.clone(),
                ));
            }
        }

        while x0 < self.center.0 {
            self.center.0 -= 1;
            for iy in
                self.center.1 - half_patches_per_side..self.center.1 + half_patches_per_side + 1
            {
                self.patches.push(Gm::new(
                    TerrainPatch::new(
                        &self.context,
                        &self.height_map,
                        (self.center.0 - half_patches_per_side, iy),
                        self.index_buffer1.clone(),
                        self.vertex_distance,
                    ),
                    self.material.clone(),
                ));
            }
        }
        while y0 > self.center.1 {
            self.center.1 += 1;
            for ix in
                self.center.0 - half_patches_per_side..self.center.0 + half_patches_per_side + 1
            {
                self.patches.push(Gm::new(
                    TerrainPatch::new(
                        &self.context,
                        &self.height_map,
                        (ix, self.center.1 + half_patches_per_side),
                        self.index_buffer1.clone(),
                        self.vertex_distance,
                    ),
                    self.material.clone(),
                ));
            }
        }

        while y0 < self.center.1 {
            self.center.1 -= 1;
            for ix in
                self.center.0 - half_patches_per_side..self.center.0 + half_patches_per_side + 1
            {
                self.patches.push(Gm::new(
                    TerrainPatch::new(
                        &self.context,
                        &self.height_map,
                        (ix, self.center.1 - half_patches_per_side),
                        self.index_buffer1.clone(),
                        self.vertex_distance,
                    ),
                    self.material.clone(),
                ));
            }
        }

        self.patches.retain(|p| {
            let (ix, iy) = p.index();
            (x0 - ix).abs() <= half_patches_per_side && (y0 - iy).abs() <= half_patches_per_side
        });

        self.patches.iter_mut().for_each(|p| {
            let distance = p.center().distance(center);
            p.index_buffer = match (*self.lod)(distance) {
                Lod::Low => self.index_buffer16.clone(),
                Lod::Medium => self.index_buffer4.clone(),
                Lod::High => self.index_buffer1.clone(),
            };
        })
    }

    fn indices(context: &Context, resolution: u32) -> Rc<ElementBuffer> {
        let mut indices: Vec<u32> = Vec::new();
        let stride = VERTICES_PER_SIDE as u32;
        let max = (stride - 1) / resolution;
        for r in 0..max {
            for c in 0..max {
                indices.push(r * resolution + c * resolution * stride);
                indices.push(r * resolution + resolution + c * resolution * stride);
                indices.push(r * resolution + (c * resolution + resolution) * stride);
                indices.push(r * resolution + (c * resolution + resolution) * stride);
                indices.push(r * resolution + resolution + c * resolution * stride);
                indices.push(r * resolution + resolution + (c * resolution + resolution) * stride);
            }
        }
        Rc::new(ElementBuffer::new_with_data(context, &indices))
    }

    ///
    /// Returns an iterator over the reference to the objects in this terrain which can be used as input to a render function, for example [RenderTarget::render].
    ///
    pub fn obj_iter(&self) -> impl Iterator<Item = &dyn Object> + Clone {
        self.patches.iter().map(|m| m as &dyn Object)
    }

    ///
    /// Returns an iterator over the reference to the geometries in this terrain which can be used as input to for example [pick], [RenderTarget::render_with_material] or [DirectionalLight::generate_shadow_map].
    ///
    pub fn geo_iter(&self) -> impl Iterator<Item = &dyn Geometry> + Clone {
        self.patches.iter().map(|m| m as &dyn Geometry)
    }
}

fn patch_size(vertex_distance: f32) -> f32 {
    vertex_distance * (VERTICES_PER_SIDE - 1) as f32
}

fn half_patches_per_side(vertex_distance: f32, side_length: f32) -> i32 {
    let patch_size = patch_size(vertex_distance);
    let patches_per_side = (side_length / patch_size).ceil() as u32;
    (patches_per_side as i32 - 1) / 2
}

fn pos2patch(vertex_distance: f32, position: Vec2) -> (i32, i32) {
    let patch_size = vertex_distance * (VERTICES_PER_SIDE - 1) as f32;
    (
        (position.x / patch_size).floor() as i32,
        (position.y / patch_size).floor() as i32,
    )
}

struct TerrainPatch {
    context: Context,
    index: (i32, i32),
    positions_buffer: VertexBuffer,
    normals_buffer: VertexBuffer,
    center: Vec2,
    aabb: AxisAlignedBoundingBox,
    pub index_buffer: Rc<ElementBuffer>,
}

impl TerrainPatch {
    pub fn new(
        context: &Context,
        height_map: &impl Fn(f32, f32) -> f32,
        index: (i32, i32),
        index_buffer: Rc<ElementBuffer>,
        vertex_distance: f32,
    ) -> Self {
        let patch_size = patch_size(vertex_distance);
        let offset = vec2(index.0 as f32 * patch_size, index.1 as f32 * patch_size);
        let positions = Self::positions(height_map, offset, vertex_distance);
        let aabb = AxisAlignedBoundingBox::new_with_positions(&positions);
        let normals = Self::normals(height_map, offset, &positions, vertex_distance);

        let positions_buffer = VertexBuffer::new_with_data(context, &positions);
        let normals_buffer = VertexBuffer::new_with_data(context, &normals);
        Self {
            context: context.clone(),
            index,
            index_buffer,
            positions_buffer,
            normals_buffer,
            aabb,
            center: offset + vec2(0.5 * patch_size, 0.5 * patch_size),
        }
    }

    pub fn center(&self) -> Vec2 {
        self.center
    }

    pub fn index(&self) -> (i32, i32) {
        self.index
    }

    fn positions(
        height_map: &impl Fn(f32, f32) -> f32,
        offset: Vec2,
        vertex_distance: f32,
    ) -> Vec<Vec3> {
        let mut data = vec![vec3(0.0, 0.0, 0.0); VERTICES_PER_SIDE * VERTICES_PER_SIDE];
        for r in 0..VERTICES_PER_SIDE {
            for c in 0..VERTICES_PER_SIDE {
                let vertex_id = r * VERTICES_PER_SIDE + c;
                let x = offset.x + r as f32 * vertex_distance;
                let z = offset.y + c as f32 * vertex_distance;
                data[vertex_id] = vec3(x, height_map(x, z), z);
            }
        }
        data
    }

    fn normals(
        height_map: &impl Fn(f32, f32) -> f32,
        offset: Vec2,
        positions: &Vec<Vec3>,
        vertex_distance: f32,
    ) -> Vec<Vec3> {
        let mut data = vec![vec3(0.0, 0.0, 0.0); VERTICES_PER_SIDE * VERTICES_PER_SIDE];
        let h = vertex_distance;
        for r in 0..VERTICES_PER_SIDE {
            for c in 0..VERTICES_PER_SIDE {
                let vertex_id = r * VERTICES_PER_SIDE + c;
                let x = offset.x + r as f32 * vertex_distance;
                let z = offset.y + c as f32 * vertex_distance;
                let xp = if r == VERTICES_PER_SIDE - 1 {
                    height_map(x + h, z)
                } else {
                    positions[vertex_id + VERTICES_PER_SIDE][1]
                };
                let xm = if r == 0 {
                    height_map(x - h, z)
                } else {
                    positions[vertex_id - VERTICES_PER_SIDE][1]
                };
                let zp = if c == VERTICES_PER_SIDE - 1 {
                    height_map(x, z + h)
                } else {
                    positions[vertex_id + 1][1]
                };
                let zm = if c == 0 {
                    height_map(x, z - h)
                } else {
                    positions[vertex_id - 1][1]
                };
                let dx = xp - xm;
                let dz = zp - zm;
                data[vertex_id] = vec3(-dx, 2.0 * h, -dz).normalize();
            }
        }
        data
    }
}

impl Geometry for TerrainPatch {
    fn render_with_material(
        &self,
        material: &dyn Material,
        camera: &Camera,
        lights: &[&dyn Light],
    ) {
        let fragment_shader_source = material.fragment_shader_source(false, lights);
        self.context
            .program(
                &include_str!("shaders/terrain.vert"),
                &fragment_shader_source,
                |program| {
                    material.use_uniforms(program, camera, lights);
                    let transformation = Mat4::identity();
                    program.use_uniform("modelMatrix", &transformation);
                    program.use_uniform(
                        "viewProjectionMatrix",
                        &(camera.projection() * camera.view()),
                    );
                    program.use_uniform(
                        "normalMatrix",
                        &transformation.invert().unwrap().transpose(),
                    );
                    let render_states = RenderStates {
                        cull: Cull::Back,
                        ..Default::default()
                    };

                    program.use_vertex_attribute("position", &self.positions_buffer);
                    program.use_vertex_attribute("normal", &self.normals_buffer);
                    program.draw_elements(render_states, camera.viewport(), &self.index_buffer);
                },
            )
            .unwrap();
    }

    fn aabb(&self) -> AxisAlignedBoundingBox {
        self.aabb
    }
}
