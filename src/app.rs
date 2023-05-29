use eframe::{
    egui, egui_wgpu,
    wgpu::{self, include_wgsl},
};
use encase::{ArrayLength, ShaderSize, ShaderType, UniformBuffer};

use crate::{StorageBuffer, Texture, CHUNK_SIZE};

#[derive(ShaderType)]
pub struct GpuCamera {
    position: cgmath::Vector4<f32>,
    forward: cgmath::Vector4<f32>,
    right: cgmath::Vector4<f32>,
    up: cgmath::Vector4<f32>,
    fov: f32,
    max_distance: f32,
}

#[derive(ShaderType)]
pub struct Material {
    color: cgmath::Vector3<f32>,
}

#[derive(ShaderType)]
pub struct Materials<'a> {
    count: ArrayLength,
    #[size(runtime)]
    data: &'a [Material],
}

#[derive(ShaderType)]
pub struct Voxel {
    material: u32,
}

#[derive(ShaderType)]
pub struct Chunk {
    data: [Voxel; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as _],
}

pub struct App {
    last_time: std::time::Instant,
    main_texture: Texture<'static>,
    main_egui_texture_id: egui::TextureId,
    main_texture_bind_group_layout: wgpu::BindGroupLayout,
    main_texture_bind_group: wgpu::BindGroup,
    camera: GpuCamera,
    camera_uniform_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    materials_storage_buffer: StorageBuffer<'static>,
    voxels_storage_buffer: StorageBuffer<'static>,
    tesseracts_bind_group_layout: wgpu::BindGroupLayout,
    tesseracts_bind_group: wgpu::BindGroup,
    ray_tracing_pipeline: wgpu::ComputePipeline,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let egui_wgpu::RenderState {
            device, renderer, ..
        } = cc.wgpu_render_state.as_ref().unwrap();

        let main_texture = Texture::new(
            device,
            wgpu::TextureDescriptor {
                label: Some("Main Texture"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
                view_formats: &[],
            },
        );
        let main_egui_texture_id = renderer.write().register_native_texture(
            device,
            &main_texture.create_view(&Default::default()),
            wgpu::FilterMode::Nearest,
        );

        let main_texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Main Texture Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                }],
            });
        let main_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Main Texture Bind Group"),
            layout: &main_texture_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(
                    &main_texture.create_view(&Default::default()),
                ),
            }],
        });

        let camera_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Uniform Buffer"),
            size: <GpuCamera as ShaderSize>::SHADER_SIZE.get(),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(<GpuCamera as ShaderSize>::SHADER_SIZE),
                    },
                    count: None,
                }],
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &camera_uniform_buffer,
                    offset: 0,
                    size: Some(<GpuCamera as ShaderSize>::SHADER_SIZE),
                }),
            }],
        });

        let materials_storage_buffer = StorageBuffer::new(
            device,
            wgpu::BufferDescriptor {
                label: Some("Materials Storage Buffer"),
                size: <Materials<'_> as ShaderType>::min_size().get(),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            },
        );
        let voxels_storage_buffer = StorageBuffer::new(
            device,
            wgpu::BufferDescriptor {
                label: Some("Voxels Storage Buffer"),
                size: <Chunk as ShaderType>::min_size().get(),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            },
        );
        let tesseracts_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Tesseracts Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: Some(<Materials<'_> as ShaderType>::min_size()),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: Some(<Chunk as ShaderType>::min_size()),
                        },
                        count: None,
                    },
                ],
            });
        let tesseracts_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Tesseracts Bind Group"),
            layout: &tesseracts_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &materials_storage_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &voxels_storage_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        let ray_tracing_shader = device.create_shader_module(include_wgsl!("./ray_tracing.wgsl"));
        let ray_tracing_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Ray Tracing Pipeline Layout"),
                bind_group_layouts: &[
                    &main_texture_bind_group_layout,
                    &camera_bind_group_layout,
                    &tesseracts_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let ray_tracing_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Ray Tracing Pipeline"),
                layout: Some(&ray_tracing_pipeline_layout),
                module: &ray_tracing_shader,
                entry_point: "main",
            });

        Self {
            last_time: std::time::Instant::now(),
            main_texture,
            main_egui_texture_id,
            main_texture_bind_group_layout,
            main_texture_bind_group,
            camera: GpuCamera {
                position: cgmath::vec4(0.0, 0.0, -3.0, 0.0),
                forward: cgmath::vec4(0.0, 0.0, 1.0, 0.0),
                right: cgmath::vec4(1.0, 0.0, 0.0, 0.0),
                up: cgmath::vec4(0.0, 1.0, 0.0, 0.0),
                fov: 90.0,
                max_distance: 100.0,
            },
            camera_uniform_buffer,
            camera_bind_group,
            materials_storage_buffer,
            voxels_storage_buffer,
            tesseracts_bind_group_layout,
            tesseracts_bind_group,
            ray_tracing_pipeline,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        let time = std::time::Instant::now();
        let dt = time.duration_since(self.last_time);
        self.last_time = time;

        let ts = dt.as_secs_f32();

        let egui_wgpu::RenderState {
            device,
            queue,
            renderer,
            ..
        } = frame.wgpu_render_state().unwrap();

        if !ctx.wants_keyboard_input() {
            ctx.input(|i| {
                if i.key_down(egui::Key::W) {
                    self.camera.position += self.camera.forward * (5.0 * ts);
                }
                if i.key_down(egui::Key::S) {
                    self.camera.position -= self.camera.forward * (5.0 * ts);
                }
                if i.key_down(egui::Key::A) {
                    self.camera.position -= self.camera.right * (5.0 * ts);
                }
                if i.key_down(egui::Key::D) {
                    self.camera.position += self.camera.right * (5.0 * ts);
                }
                if i.key_down(egui::Key::Q) {
                    self.camera.position -= self.camera.up * (5.0 * ts);
                }
                if i.key_down(egui::Key::E) {
                    self.camera.position += self.camera.up * (5.0 * ts);
                }
            });
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(255, 0, 255))) // color it pink, this should never be seen under normal circumstances
            .show(ctx, |ui| {
                let size = ui.available_size();

                // Update bind group and egui texture id if it has changed size
                if self.main_texture.resize(
                    device,
                    cgmath::vec2(size.x.max(1.0) as _, size.y.max(1.0) as _),
                ) {
                    self.main_texture_bind_group =
                        device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("Main Texture Bind Group"),
                            layout: &self.main_texture_bind_group_layout,
                            entries: &[wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(
                                    &self.main_texture.create_view(&Default::default()),
                                ),
                            }],
                        });
                    renderer.write().update_egui_texture_from_wgpu_texture(
                        device,
                        &self.main_texture.create_view(&Default::default()),
                        wgpu::FilterMode::Nearest,
                        self.main_egui_texture_id,
                    );
                }

                // Upload camera
                {
                    let mut uniform_buffer =
                        UniformBuffer::new([0; <GpuCamera as ShaderSize>::SHADER_SIZE.get() as _]);
                    uniform_buffer.write(&self.camera).unwrap();
                    let buffer = uniform_buffer.into_inner();
                    queue.write_buffer(&self.camera_uniform_buffer, 0, &buffer);
                }

                // Upload materials and voxels
                {
                    let materials = Materials {
                        count: ArrayLength,
                        data: &[
                            Material {
                                color: cgmath::Vector3 {
                                    x: 1.0,
                                    y: 0.0,
                                    z: 0.0,
                                },
                            },
                            Material {
                                color: cgmath::Vector3 {
                                    x: 0.0,
                                    y: 1.0,
                                    z: 0.0,
                                },
                            },
                        ],
                    };

                    let mut chunk = Chunk {
                        data: std::array::from_fn(|_| Voxel { material: u32::MAX }),
                    };
                    chunk.data[0].material = 0;
                    chunk.data[2].material = 1;
                    chunk.data[4].material = 1;

                    let mut bind_group_invalidated = false;

                    // Upload materials
                    {
                        let mut materials_storage_buffer =
                            encase::StorageBuffer::new(Vec::with_capacity(
                                std::mem::size_of_val(&materials.count)
                                    + std::mem::size_of_val(materials.data),
                            ));
                        materials_storage_buffer.write(&materials).unwrap();
                        let materials_buffer = materials_storage_buffer.into_inner();

                        bind_group_invalidated |= self.materials_storage_buffer.set_data_lossy(
                            device,
                            queue,
                            &materials_buffer,
                        );
                    }

                    // Upload chunks
                    {
                        let mut chunk_storage_buffer = encase::StorageBuffer::new(
                            Vec::with_capacity(std::mem::size_of_val(&chunk.data)),
                        );
                        chunk_storage_buffer.write(&chunk).unwrap();
                        let chunk_buffer = chunk_storage_buffer.into_inner();
                        bind_group_invalidated |=
                            self.voxels_storage_buffer
                                .set_data_lossy(device, queue, &chunk_buffer);
                    }

                    if bind_group_invalidated {
                        self.tesseracts_bind_group =
                            device.create_bind_group(&wgpu::BindGroupDescriptor {
                                label: Some("Tesseracts Bind Group"),
                                layout: &self.tesseracts_bind_group_layout,
                                entries: &[
                                    wgpu::BindGroupEntry {
                                        binding: 0,
                                        resource: wgpu::BindingResource::Buffer(
                                            wgpu::BufferBinding {
                                                buffer: &self.materials_storage_buffer,
                                                offset: 0,
                                                size: None,
                                            },
                                        ),
                                    },
                                    wgpu::BindGroupEntry {
                                        binding: 1,
                                        resource: wgpu::BindingResource::Buffer(
                                            wgpu::BufferBinding {
                                                buffer: &self.voxels_storage_buffer,
                                                offset: 0,
                                                size: None,
                                            },
                                        ),
                                    },
                                ],
                            });
                    }
                }

                // Submit ray tracing commands
                {
                    let mut command_encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Ray Tracing Encoder"),
                        });
                    // Compute Pass
                    {
                        let mut compute_pass =
                            command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                                label: Some("Ray Tracing Compute Pass"),
                            });
                        compute_pass.set_pipeline(&self.ray_tracing_pipeline);
                        compute_pass.set_bind_group(0, &self.main_texture_bind_group, &[]);
                        compute_pass.set_bind_group(1, &self.camera_bind_group, &[]);
                        compute_pass.set_bind_group(2, &self.tesseracts_bind_group, &[]);
                        let workgroups = {
                            const WORKGROUPS_SIZE: cgmath::Vector2<u32> = cgmath::vec2(16, 16);
                            let size = self.main_texture.size();
                            cgmath::vec2(
                                (size.width + WORKGROUPS_SIZE.x - 1) / WORKGROUPS_SIZE.x,
                                (size.height + WORKGROUPS_SIZE.y - 1) / WORKGROUPS_SIZE.y,
                            )
                        };
                        compute_pass.dispatch_workgroups(workgroups.x, workgroups.y, 1);
                    }
                    queue.submit([command_encoder.finish()]);
                }

                ui.image(self.main_egui_texture_id, size);
            });

        ctx.request_repaint();
    }
}
