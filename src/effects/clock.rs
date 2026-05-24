use std::{borrow::Cow, collections::HashMap};

use iced::widget::canvas;
use iced::{
    Alignment, Color, Font, Pixels, Point, Rectangle, Size, alignment, mouse, wgpu, window,
};

const WALLPAPER_BYTES: &[u8] = include_bytes!("../../bg.jpg");
const WALLPAPER_SIZE: [f32; 2] = [1600.0, 900.0];
const CLOCK_FONT_FAMILY: &str = "Sarasa Gothic TC";

#[derive(Debug, Clone)]
pub(crate) struct MacosGlassClock {
    window: window::Id,
    date: String,
    time: String,
}

impl MacosGlassClock {
    pub(crate) fn new(window: window::Id, date: String, time: String) -> Self {
        Self { window, date, time }
    }
}

impl<Message> iced::widget::shader::Program<Message> for MacosGlassClock {
    type State = ();
    type Primitive = MacosGlassClockPrimitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: Rectangle,
    ) -> Self::Primitive {
        MacosGlassClockPrimitive {
            window: self.window,
            date: self.date.clone(),
            time: self.time.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MacosGlassClockPrimitive {
    window: window::Id,
    date: String,
    time: String,
}

impl iced::widget::shader::Primitive for MacosGlassClockPrimitive {
    type Pipeline = MacosGlassClockPipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &iced::widget::shader::Viewport,
    ) {
        pipeline.prepare(self, device, queue, bounds, viewport);
    }

    fn draw(&self, pipeline: &Self::Pipeline, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        pipeline.draw(self, render_pass)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ClockUniform {
    resolution: [f32; 2],
    wallpaper_size: [f32; 2],
}

#[derive(Debug, Clone, PartialEq)]
struct MaskCacheKey {
    physical_size: [u32; 2],
    scale_factor: f32,
    date: String,
    time: String,
}

struct ClockBindings {
    buffer: wgpu::Buffer,
    mask_texture: wgpu::Texture,
    mask_view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    cache_key: Option<MaskCacheKey>,
}

impl ClockBindings {
    fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        wallpaper_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("limes macos clock uniforms"),
            size: std::mem::size_of::<ClockUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mask_texture =
            create_mask_texture(device, [1, 1], Some("limes macos clock empty mask"));
        let mask_view = mask_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = create_bind_group(
            device,
            layout,
            &buffer,
            wallpaper_view,
            sampler,
            &mask_view,
            sampler,
        );

        Self {
            buffer,
            mask_texture,
            mask_view,
            bind_group,
            cache_key: None,
        }
    }

    fn update_mask(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        resources: ClockMaskResources<'_>,
        mask: ClockMask,
        cache_key: MaskCacheKey,
    ) {
        self.mask_texture = create_mask_texture(device, mask.size, Some("limes macos clock mask"));
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.mask_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &mask.alpha,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(mask.size[0]),
                rows_per_image: Some(mask.size[1]),
            },
            wgpu::Extent3d {
                width: mask.size[0],
                height: mask.size[1],
                depth_or_array_layers: 1,
            },
        );

        self.mask_view = self
            .mask_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.bind_group = create_bind_group(
            device,
            resources.layout,
            &self.buffer,
            resources.wallpaper_view,
            resources.sampler,
            &self.mask_view,
            resources.sampler,
        );
        self.cache_key = Some(cache_key);
    }
}

struct ClockMaskResources<'a> {
    layout: &'a wgpu::BindGroupLayout,
    wallpaper_view: &'a wgpu::TextureView,
    sampler: &'a wgpu::Sampler,
}

pub(crate) struct MacosGlassClockPipeline {
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    _wallpaper_texture: wgpu::Texture,
    wallpaper_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    bindings: HashMap<window::Id, ClockBindings>,
}

impl MacosGlassClockPipeline {
    fn prepare(
        &mut self,
        primitive: &MacosGlassClockPrimitive,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &iced::widget::shader::Viewport,
    ) {
        let scale_factor = viewport.scale_factor();
        let physical_size = [
            (bounds.width * scale_factor).round().max(1.0) as u32,
            (bounds.height * scale_factor).round().max(1.0) as u32,
        ];
        let uniforms = ClockUniform {
            resolution: [physical_size[0] as f32, physical_size[1] as f32],
            wallpaper_size: WALLPAPER_SIZE,
        };
        let bind_group_layout = self.bind_group_layout.clone();
        let wallpaper_view = self.wallpaper_view.clone();
        let sampler = self.sampler.clone();

        let bindings = self.bindings(device, primitive.window);
        queue.write_buffer(&bindings.buffer, 0, bytemuck::bytes_of(&uniforms));

        let cache_key = MaskCacheKey {
            physical_size,
            scale_factor,
            date: primitive.date.clone(),
            time: primitive.time.clone(),
        };

        if bindings.cache_key.as_ref() != Some(&cache_key) {
            let mask = build_clock_mask(
                &primitive.date,
                &primitive.time,
                Size::new(bounds.width, bounds.height),
                physical_size,
                scale_factor,
            );
            let resources = ClockMaskResources {
                layout: &bind_group_layout,
                wallpaper_view: &wallpaper_view,
                sampler: &sampler,
            };
            bindings.update_mask(device, queue, resources, mask, cache_key);
        }
    }

    fn draw(
        &self,
        primitive: &MacosGlassClockPrimitive,
        render_pass: &mut wgpu::RenderPass<'_>,
    ) -> bool {
        let Some(bindings) = self.bindings.get(&primitive.window) else {
            return false;
        };

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &bindings.bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        true
    }

    fn bindings(&mut self, device: &wgpu::Device, window: window::Id) -> &mut ClockBindings {
        if !self.bindings.contains_key(&window) {
            let bindings = ClockBindings::new(
                device,
                &self.bind_group_layout,
                &self.wallpaper_view,
                &self.sampler,
            );
            self.bindings.insert(window, bindings);
        }

        self.bindings
            .get_mut(&window)
            .expect("clock bindings should be initialized")
    }
}

impl iced::widget::shader::Pipeline for MacosGlassClockPipeline {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let wallpaper = image::load_from_memory(WALLPAPER_BYTES)
            .expect("bundled wallpaper should decode")
            .to_rgba8();
        let wallpaper_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("limes macos clock wallpaper texture"),
            size: wgpu::Extent3d {
                width: wallpaper.width(),
                height: wallpaper.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &wallpaper_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wallpaper.as_raw(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(wallpaper.width() * 4),
                rows_per_image: Some(wallpaper.height()),
            },
            wgpu::Extent3d {
                width: wallpaper.width(),
                height: wallpaper.height(),
                depth_or_array_layers: 1,
            },
        );
        let wallpaper_view = wallpaper_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("limes macos clock sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("limes macos clock bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<ClockUniform>() as u64,
                        ),
                    },
                    count: None,
                },
                texture_binding(1, wgpu::TextureSampleType::Float { filterable: true }),
                sampler_binding(2),
                texture_binding(3, wgpu::TextureSampleType::Float { filterable: true }),
                sampler_binding(4),
            ],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("limes macos clock pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("limes macos clock shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../macos_clock.wgsl"))),
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("limes macos clock pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            render_pipeline,
            bind_group_layout,
            _wallpaper_texture: wallpaper_texture,
            wallpaper_view,
            sampler,
            bindings: HashMap::new(),
        }
    }
}

fn texture_binding(
    binding: u32,
    sample_type: wgpu::TextureSampleType,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_binding(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn create_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
    wallpaper_view: &wgpu::TextureView,
    wallpaper_sampler: &wgpu::Sampler,
    mask_view: &wgpu::TextureView,
    mask_sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("limes macos clock bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(wallpaper_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(wallpaper_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(mask_view),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Sampler(mask_sampler),
            },
        ],
    })
}

fn create_mask_texture(
    device: &wgpu::Device,
    size: [u32; 2],
    label: Option<&'static str>,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label,
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

struct ClockMask {
    size: [u32; 2],
    alpha: Vec<u8>,
}

fn build_clock_mask(
    date: &str,
    time: &str,
    logical_size: Size,
    physical_size: [u32; 2],
    scale_factor: f32,
) -> ClockMask {
    let mut pixmap =
        tiny_skia::Pixmap::new(physical_size[0], physical_size[1]).expect("create clock mask");
    let mut paint = tiny_skia::Paint::default();
    paint.set_color_rgba8(255, 255, 255, 255);
    paint.anti_alias = true;
    let transform = tiny_skia::Transform::from_scale(scale_factor, scale_factor);
    let metrics = ClockMetrics::new(logical_size);

    fill_text_mask(
        &mut pixmap,
        date,
        metrics.date_center,
        metrics.date_size,
        clock_font(iced::font::Weight::Semibold),
        &paint,
        transform,
    );
    fill_text_mask(
        &mut pixmap,
        time,
        metrics.time_center,
        metrics.time_size,
        clock_font(iced::font::Weight::Bold),
        &paint,
        transform,
    );
    let alpha = soften_alpha(
        pixmap.pixels().iter().map(|pixel| pixel.alpha()).collect(),
        physical_size,
    );

    ClockMask {
        size: physical_size,
        alpha,
    }
}

fn clock_font(weight: iced::font::Weight) -> Font {
    Font {
        weight,
        ..Font::with_name(CLOCK_FONT_FAMILY)
    }
}

fn soften_alpha(alpha: Vec<u8>, size: [u32; 2]) -> Vec<u8> {
    let width = size[0] as usize;
    let height = size[1] as usize;
    let mut expanded = alpha.clone();

    for y in 0..height {
        for x in 0..width {
            let mut max_alpha = alpha[y * width + x] as f32;

            for (offset_x, offset_y, weight) in [
                (-1isize, 0isize, 0.78),
                (1, 0, 0.78),
                (0, -1, 0.78),
                (0, 1, 0.78),
                (-1, -1, 0.50),
                (1, -1, 0.50),
                (-1, 1, 0.50),
                (1, 1, 0.50),
            ] {
                let sample_x = x as isize + offset_x;
                let sample_y = y as isize + offset_y;

                if sample_x >= 0
                    && sample_y >= 0
                    && sample_x < width as isize
                    && sample_y < height as isize
                {
                    let sample = alpha[sample_y as usize * width + sample_x as usize] as f32;
                    max_alpha = max_alpha.max(sample * weight);
                }
            }

            expanded[y * width + x] = max_alpha.round().clamp(0.0, 255.0) as u8;
        }
    }

    let mut blurred = expanded.clone();
    for y in 0..height {
        for x in 0..width {
            let center = expanded[y * width + x] as f32 * 4.0;
            let mut total = center;
            let mut weight = 4.0;

            for (offset_x, offset_y, sample_weight) in [
                (-1isize, 0isize, 2.0),
                (1, 0, 2.0),
                (0, -1, 2.0),
                (0, 1, 2.0),
                (-1, -1, 1.0),
                (1, -1, 1.0),
                (-1, 1, 1.0),
                (1, 1, 1.0),
            ] {
                let sample_x = x as isize + offset_x;
                let sample_y = y as isize + offset_y;

                if sample_x >= 0
                    && sample_y >= 0
                    && sample_x < width as isize
                    && sample_y < height as isize
                {
                    total += expanded[sample_y as usize * width + sample_x as usize] as f32
                        * sample_weight;
                    weight += sample_weight;
                }
            }

            blurred[y * width + x] = (total / weight).round().clamp(0.0, 255.0) as u8;
        }
    }

    blurred
}

fn fill_text_mask(
    pixmap: &mut tiny_skia::Pixmap,
    content: &str,
    position: Point,
    size: f32,
    font: Font,
    paint: &tiny_skia::Paint<'_>,
    transform: tiny_skia::Transform,
) {
    let text = canvas::Text {
        content: content.to_owned(),
        position,
        max_width: f32::INFINITY,
        color: Color::WHITE,
        size: Pixels(size),
        font,
        align_x: Alignment::Center.into(),
        align_y: alignment::Vertical::Center,
        ..Default::default()
    };

    text.draw_with(|path, _color| {
        if let Some(path) = convert_path(&path) {
            pixmap.fill_path(&path, paint, tiny_skia::FillRule::Winding, transform, None);
        }
    });
}

fn convert_path(path: &canvas::Path) -> Option<tiny_skia::Path> {
    let mut builder = tiny_skia::PathBuilder::new();
    let mut last_point = None;

    for event in path.raw() {
        match event {
            canvas::path::lyon_path::Event::Begin { at } => {
                builder.move_to(at.x, at.y);
                last_point = Some(at);
            }
            canvas::path::lyon_path::Event::Line { from, to } => {
                if last_point != Some(from) {
                    builder.move_to(from.x, from.y);
                }
                builder.line_to(to.x, to.y);
                last_point = Some(to);
            }
            canvas::path::lyon_path::Event::Quadratic { from, ctrl, to } => {
                if last_point != Some(from) {
                    builder.move_to(from.x, from.y);
                }
                builder.quad_to(ctrl.x, ctrl.y, to.x, to.y);
                last_point = Some(to);
            }
            canvas::path::lyon_path::Event::Cubic {
                from,
                ctrl1,
                ctrl2,
                to,
            } => {
                if last_point != Some(from) {
                    builder.move_to(from.x, from.y);
                }
                builder.cubic_to(ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y);
                last_point = Some(to);
            }
            canvas::path::lyon_path::Event::End { close, .. } => {
                if close {
                    builder.close();
                }
                last_point = None;
            }
        }
    }

    builder.finish()
}

struct ClockMetrics {
    date_center: Point,
    date_size: f32,
    time_center: Point,
    time_size: f32,
}

impl ClockMetrics {
    fn new(size: Size) -> Self {
        let base_time_size = (size.width * 0.07375).clamp(52.5, 107.5);
        let base_date_size = (base_time_size * 0.27).clamp(27.0, 52.0);
        let time_size = base_time_size;
        let date_size = base_date_size;
        let top = (size.height * 0.047).clamp(30.0, 56.0);
        let gap = (base_time_size * 0.05).clamp(4.0, 10.0);
        let center_x = size.width / 2.0;
        let date_center = Point::new(center_x, top + date_size / 2.0);
        let time_center = Point::new(center_x, top + date_size + gap + time_size / 2.0);

        Self {
            date_center,
            date_size,
            time_center,
            time_size,
        }
    }
}
