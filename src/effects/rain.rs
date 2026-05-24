use std::{borrow::Cow, collections::HashMap};

use iced::{Rectangle, mouse, wgpu, window};

#[derive(Debug, Clone, Copy)]
pub(crate) struct RainDrops {
    window: window::Id,
    time: f32,
    intensity: f32,
}

impl RainDrops {
    pub(crate) fn new(window: window::Id, time: f32, intensity: f32) -> Self {
        Self {
            window,
            time,
            intensity,
        }
    }
}

impl<Message> iced::widget::shader::Program<Message> for RainDrops {
    type State = ();
    type Primitive = RainDropsPrimitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: Rectangle,
    ) -> Self::Primitive {
        RainDropsPrimitive {
            window: self.window,
            time: self.time,
            intensity: self.intensity,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RainDropsPrimitive {
    window: window::Id,
    time: f32,
    intensity: f32,
}

impl iced::widget::shader::Primitive for RainDropsPrimitive {
    type Pipeline = RainDropsPipeline;

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
struct RainDropsUniform {
    resolution: [f32; 2],
    time: f32,
    intensity: f32,
}

struct RainDropsBindings {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl RainDropsBindings {
    fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("limes rain-drop uniforms"),
            size: std::mem::size_of::<RainDropsUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("limes rain-drop bind group"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self { buffer, bind_group }
    }
}

pub(crate) struct RainDropsPipeline {
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    uniforms: HashMap<window::Id, RainDropsBindings>,
}

impl RainDropsPipeline {
    fn prepare(
        &mut self,
        primitive: &RainDropsPrimitive,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &iced::widget::shader::Viewport,
    ) {
        let scale_factor = viewport.scale_factor();
        let uniforms = RainDropsUniform {
            resolution: [
                (bounds.width * scale_factor).max(1.0),
                (bounds.height * scale_factor).max(1.0),
            ],
            time: primitive.time,
            intensity: primitive.intensity.clamp(0.0, 1.0),
        };

        let bindings = self.bindings(device, primitive.window);
        queue.write_buffer(&bindings.buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    fn draw(&self, primitive: &RainDropsPrimitive, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        let Some(bindings) = self.uniforms.get(&primitive.window) else {
            return false;
        };

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &bindings.bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        true
    }

    fn bindings(&mut self, device: &wgpu::Device, window: window::Id) -> &RainDropsBindings {
        if !self.uniforms.contains_key(&window) {
            let bindings = RainDropsBindings::new(device, &self.bind_group_layout);
            self.uniforms.insert(window, bindings);
        }

        self.uniforms
            .get(&window)
            .expect("rain-drop uniforms should be initialized")
    }
}

impl iced::widget::shader::Pipeline for RainDropsPipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("limes rain-drop bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        std::mem::size_of::<RainDropsUniform>() as u64,
                    ),
                },
                count: None,
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("limes rain-drop pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("limes rain-drop shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(concat!(
                include_str!("../rain_ripples.wgsl"),
                "\n",
                include_str!("../rain_drops.wgsl"),
            ))),
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("limes rain-drop pipeline"),
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
            uniforms: HashMap::new(),
        }
    }
}
