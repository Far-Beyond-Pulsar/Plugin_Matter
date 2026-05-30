//! WGPU canvas renderer — renders grid background + painting layers into a WgpuSurface.
//!
//! Follows the same lazy-init pattern as the Helio level-editor renderer:
//!   1. First call to `render_frame` creates all WGPU resources.
//!   2. Subsequent calls update uniforms and issue draw commands.

use std::collections::HashMap;

use wgpu::util::DeviceExt as _;

use crate::state::Document;

use super::types::CanvasRenderInput;

// ── Uniform layout (must match shader structs, 16-byte aligned) ──────────────

/// GridUniforms — 32 bytes, matches `grid.wgsl`
#[repr(C)]
#[derive(Copy, Clone)]
struct GridUniforms {
    pan_offset:    [f32; 2],
    zoom:          f32,
    _pad0:         f32,
    viewport_size: [f32; 2],
    canvas_size:   [f32; 2],
}

/// TileUniforms — 32 bytes, matches `tile.wgsl`
#[repr(C)]
#[derive(Copy, Clone)]
struct TileUniforms {
    pan_offset:    [f32; 2],
    zoom:          f32,
    _pad:          f32,
    viewport_size: [f32; 2],
    _pad2:         [f32; 2],
}

/// Vertex layout for tile quads: `canvas_pos` + `uv` (4 × f32 = 16 bytes).
#[repr(C)]
#[derive(Copy, Clone)]
struct TileVertex {
    canvas_pos: [f32; 2],
    uv:         [f32; 2],
}

fn as_bytes<T: Copy>(v: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v as *const T as *const u8, std::mem::size_of::<T>()) }
}

fn slice_as_bytes<T: Copy>(v: &[T]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * std::mem::size_of::<T>())
    }
}

// ── Per-layer GPU resources ───────────────────────────────────────────────────

struct LayerGpu {
    texture:    wgpu::Texture,
    bind_group: wgpu::BindGroup,
    width:      u32,
    height:     u32,
}

// ── Initialised GPU state ─────────────────────────────────────────────────────

struct GpuState {
    // ── Grid pass ──────────────────────────────────────────────────────────
    grid_pipeline:    wgpu::RenderPipeline,
    grid_uniform_buf: wgpu::Buffer,
    grid_bind_group:  wgpu::BindGroup,

    // ── Tile pass ──────────────────────────────────────────────────────────
    tile_pipeline:          wgpu::RenderPipeline,
    tile_uniform_buf:       wgpu::Buffer,
    tile_bind_group_layout: wgpu::BindGroupLayout,
    tile_sampler:           wgpu::Sampler,

    // ── Per-layer composite textures (keyed by layer id) ───────────────────
    layer_gpu: HashMap<String, LayerGpu>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// WGPU renderer for the Matter canvas. Lazily initialises GPU resources on
/// the first call to [`render_frame`].
pub struct CanvasRenderer {
    state: Option<GpuState>,
}

impl CanvasRenderer {
    pub fn new() -> Self {
        Self { state: None }
    }

    /// Render one frame into `view`.
    /// `live_tiles` contains tiles modified during an active stroke that have
    /// not yet been committed to PIF — they override the PIF data for display.
    pub fn render_frame(
        &mut self,
        device: &wgpu::Device,
        queue:  &wgpu::Queue,
        view:   &wgpu::TextureView,
        width:  u32,
        height: u32,
        format: wgpu::TextureFormat,
        input:  &CanvasRenderInput,
        document: &Document,
        live_tiles: Option<&HashMap<(String, u32, u32), Vec<u8>>>,
    ) {
        if self.state.is_none() {
            self.state = Some(Self::create_gpu_state(device, format));
        }
        let state = self.state.as_mut().unwrap();

        let vp_w = width  as f32;
        let vp_h = height as f32;

        // ── Update grid uniforms ───────────────────────────────────────────
        let grid_u = GridUniforms {
            pan_offset:    input.pan_offset,
            zoom:          input.zoom,
            _pad0:         0.0,
            viewport_size: [vp_w, vp_h],
            canvas_size:   input.canvas_size,
        };
        queue.write_buffer(&state.grid_uniform_buf, 0, as_bytes(&grid_u));

        // ── Update tile uniforms ───────────────────────────────────────────
        let tile_u = TileUniforms {
            pan_offset:    input.pan_offset,
            zoom:          input.zoom,
            _pad:          0.0,
            viewport_size: [vp_w, vp_h],
            _pad2:         [0.0; 2],
        };
        queue.write_buffer(&state.tile_uniform_buf, 0, as_bytes(&tile_u));

        // ── Sync layer textures (PIF data + live stroke overlay) ──────────
        Self::sync_layers(state, device, queue, document, live_tiles);

        // ── Build render pass ──────────────────────────────────────────────
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("canvas_encoder"),
        });

        // Grid pass — clear + full-screen quad
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("canvas_grid_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice:    None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load:  wgpu::LoadOp::Clear(wgpu::Color { r: 0.118, g: 0.118, b: 0.137, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes:         None,
                occlusion_query_set:      None,
                multiview_mask:           None,
            });
            pass.set_pipeline(&state.grid_pipeline);
            pass.set_bind_group(0, &state.grid_bind_group, &[]);
            pass.draw(0..6, 0..1);
        }

        // Tile pass — one draw per visible layer (alpha blend on top of grid)
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("canvas_tile_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice:    None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load:  wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes:         None,
                occlusion_query_set:      None,
                multiview_mask:           None,
            });
            pass.set_pipeline(&state.tile_pipeline);

            let layers = document.layers();
            for layer in &layers {
                use pulsar_image_format::model::Layer;
                let (id, visible) = match layer {
                    Layer::Raster { id, visible, .. } => (id.clone(), *visible),
                    Layer::Vector { id, visible, .. } => (id.clone(), *visible),
                };
                if !visible { continue; }

                if let Some(layer_gpu) = state.layer_gpu.get(&id) {
                    let cw = layer_gpu.width  as f32;
                    let ch = layer_gpu.height as f32;
                    let quad: [TileVertex; 6] = [
                        TileVertex { canvas_pos: [0.0, 0.0], uv: [0.0, 0.0] },
                        TileVertex { canvas_pos: [cw,  0.0], uv: [1.0, 0.0] },
                        TileVertex { canvas_pos: [0.0, ch ], uv: [0.0, 1.0] },
                        TileVertex { canvas_pos: [cw,  0.0], uv: [1.0, 0.0] },
                        TileVertex { canvas_pos: [cw,  ch ], uv: [1.0, 1.0] },
                        TileVertex { canvas_pos: [0.0, ch ], uv: [0.0, 1.0] },
                    ];
                    let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label:    Some("tile_quad_vbuf"),
                        contents: slice_as_bytes(&quad),
                        usage:    wgpu::BufferUsages::VERTEX,
                    });

                    pass.set_bind_group(0, &layer_gpu.bind_group, &[]);
                    pass.set_vertex_buffer(0, vbuf.slice(..));
                    pass.draw(0..6, 0..1);
                }
            }
        }

        queue.submit(std::iter::once(encoder.finish()));
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn sync_layers(
        state:      &mut GpuState,
        device:     &wgpu::Device,
        queue:      &wgpu::Queue,
        document:   &Document,
        live_tiles: Option<&HashMap<(String, u32, u32), Vec<u8>>>,
    ) {
        let (canvas_w, canvas_h) = document.dimensions();
        let layers = document.layers();

        for layer in &layers {
            use pulsar_image_format::model::Layer;
            let id = match layer {
                Layer::Raster { id, .. } | Layer::Vector { id, .. } => id.clone(),
            };

            let needs_create = state.layer_gpu
                .get(&id)
                .map(|g| g.width != canvas_w || g.height != canvas_h)
                .unwrap_or(true);

            if needs_create {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label:           Some("layer_composite"),
                    size:            wgpu::Extent3d { width: canvas_w, height: canvas_h, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count:    1,
                    dimension:       wgpu::TextureDimension::D2,
                    format:          wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage:           wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats:    &[],
                });
                let tex_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label:   Some("layer_bg"),
                    layout:  &state.tile_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: state.tile_uniform_buf.as_entire_binding() },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tex_view) },
                        wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&state.tile_sampler) },
                    ],
                });
                state.layer_gpu.insert(id.clone(), LayerGpu {
                    texture,
                    bind_group,
                    width:  canvas_w,
                    height: canvas_h,
                });
            }

            // Composite tile data into a CPU buffer then upload.
            let Some(layer_gpu) = state.layer_gpu.get(&id) else { continue };
            let tile_size: u32 = 256;
            let tiles_x = canvas_w.div_ceil(tile_size);
            let tiles_y = canvas_h.div_ceil(tile_size);

            let mut composite = vec![0u8; (canvas_w * canvas_h * 4) as usize];
            for ty in 0..tiles_y {
                for tx in 0..tiles_x {
                    // Prefer live stroke data over committed PIF data so brushstrokes
                    // appear immediately during painting.
                    let tile_data: Vec<u8> = if let Some(live) = live_tiles {
                        if let Some(d) = live.get(&(id.clone(), tx, ty)) {
                            d.clone()
                        } else {
                            let Ok(d) = document.load_tile(&id, tx, ty) else { continue };
                            d
                        }
                    } else {
                        let Ok(d) = document.load_tile(&id, tx, ty) else { continue };
                        d
                    };
                    if tile_data.is_empty() { continue }

                    let tile_w = tile_size.min(canvas_w.saturating_sub(tx * tile_size));
                    let tile_h = tile_size.min(canvas_h.saturating_sub(ty * tile_size));

                    for row in 0..tile_h {
                        let src_start = (row * tile_size * 4) as usize;
                        let dst_row   = ty * tile_size + row;
                        let dst_start = (dst_row * canvas_w * 4 + tx * tile_size * 4) as usize;
                        let len       = (tile_w * 4) as usize;

                        if src_start + len > tile_data.len() { break; }
                        if dst_start + len > composite.len()  { break; }

                        composite[dst_start..dst_start + len]
                            .copy_from_slice(&tile_data[src_start..src_start + len]);
                    }
                }
            }

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture:   &layer_gpu.texture,
                    mip_level: 0,
                    origin:    wgpu::Origin3d::ZERO,
                    aspect:    wgpu::TextureAspect::All,
                },
                &composite,
                wgpu::TexelCopyBufferLayout {
                    offset:         0,
                    bytes_per_row:  Some(canvas_w * 4),
                    rows_per_image: Some(canvas_h),
                },
                wgpu::Extent3d { width: canvas_w, height: canvas_h, depth_or_array_layers: 1 },
            );
        }

        // Drop GPU resources for removed layers.
        let live_ids: Vec<String> = layers.iter().map(|l| {
            use pulsar_image_format::model::Layer;
            match l {
                Layer::Raster { id, .. } | Layer::Vector { id, .. } => id.clone(),
            }
        }).collect();
        state.layer_gpu.retain(|k, _| live_ids.contains(k));
    }

    fn create_gpu_state(device: &wgpu::Device, format: wgpu::TextureFormat) -> GpuState {
        // ── Grid pipeline ──────────────────────────────────────────────────
        let grid_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label:  Some("grid_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/grid.wgsl").into()),
        });

        let grid_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("grid_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding:    0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty:         wgpu::BindingType::Buffer {
                    ty:                 wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size:   None,
                },
                count: None,
            }],
        });

        let grid_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label:              Some("grid_uniform_buf"),
            size:               std::mem::size_of::<GridUniforms>() as u64,
            usage:              wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let grid_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("grid_bg"),
            layout:  &grid_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding:  0,
                resource: grid_uniform_buf.as_entire_binding(),
            }],
        });

        let grid_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label:              Some("grid_layout"),
            bind_group_layouts: &[Some(&grid_bgl)],
            immediate_size:     0,
        });

        let grid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label:  Some("grid_pipeline"),
            layout: Some(&grid_layout),
            vertex: wgpu::VertexState {
                module:              &grid_shader,
                entry_point:         Some("vs_main"),
                buffers:             &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology:  wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample:   wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module:              &grid_shader,
                entry_point:         Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend:      Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache:          None,
        });

        // ── Tile pipeline ──────────────────────────────────────────────────
        let tile_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label:  Some("tile_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/tile.wgsl").into()),
        });

        let tile_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tile_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty:                 wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size:   None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type:    wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled:   false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let tile_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label:              Some("tile_uniform_buf"),
            size:               std::mem::size_of::<TileUniforms>() as u64,
            usage:              wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let tile_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label:          Some("tile_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter:     wgpu::FilterMode::Nearest,
            min_filter:     wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let tile_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label:              Some("tile_layout"),
            bind_group_layouts: &[Some(&tile_bgl)],
            immediate_size:     0,
        });

        let tile_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label:  Some("tile_pipeline"),
            layout: Some(&tile_layout),
            vertex: wgpu::VertexState {
                module:              &tile_shader,
                entry_point:         Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<TileVertex>() as u64,
                    step_mode:    wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x2 },
                        wgpu::VertexAttribute { offset: 8, shader_location: 1, format: wgpu::VertexFormat::Float32x2 },
                    ],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology:  wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample:   wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module:              &tile_shader,
                entry_point:         Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend:      Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache:          None,
        });

        GpuState {
            grid_pipeline,
            grid_uniform_buf,
            grid_bind_group,
            tile_pipeline,
            tile_uniform_buf,
            tile_bind_group_layout: tile_bgl,
            tile_sampler,
            layer_gpu: HashMap::new(),
        }
    }
}
