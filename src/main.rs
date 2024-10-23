use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
    dpi::PhysicalPosition,
};
use std::sync::Arc;
use wgpu::util::DeviceExt;

// Define vertex data structure
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

// New shape-related structures
#[derive(Debug)]
enum Shape {
    Circle { center: [f32; 2], radius: f32, segments: u32 },
    // We can add more shapes here later: Rectangle, Triangle, etc.
}

impl Shape {
    fn generate_vertices(&self) -> Vec<Vertex> {
        match self {
            Shape::Circle { center, radius, segments } => {
                let mut vertices = Vec::with_capacity((*segments as usize + 2) * 3);
                
                // Generate circle vertices
                for i in 0..*segments {
                    // Add center vertex
                    vertices.push(Vertex { position: *center });
                    
                    // Add first point of the triangle
                    let angle1 = (i as f32 * 2.0 * std::f32::consts::PI) / *segments as f32;
                    let x1 = center[0] + radius * angle1.cos();
                    let y1 = center[1] + radius * angle1.sin();
                    vertices.push(Vertex { position: [x1, y1] });
                    
                    // Add second point of the triangle
                    let angle2 = ((i + 1) as f32 * 2.0 * std::f32::consts::PI) / *segments as f32;
                    let x2 = center[0] + radius * angle2.cos();
                    let y2 = center[1] + radius * angle2.sin();
                    vertices.push(Vertex { position: [x2, y2] });
                }
                vertices
            }
        }
    }
}

// Add this after the Vertex struct
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    aspect_ratio: f32,
}

// Add this new struct after the Uniforms struct
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct MouseState {
    position: [f32; 2],
}

// Update the App struct to include the new uniform buffer and bind group
struct App {
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    render_pipeline: Option<wgpu::RenderPipeline>,
    vertex_buffer: Option<wgpu::Buffer>,
    config: Option<wgpu::SurfaceConfiguration>,
    shapes: Vec<Shape>,
    num_vertices: u32,
    uniform_buffer: Option<wgpu::Buffer>,
    uniform_bind_group: Option<wgpu::BindGroup>,
    mouse_state: MouseState,
}

impl App {
    fn default() -> Self {
        Self {
            window: None,
            surface: None,
            device: None,
            queue: None,
            render_pipeline: None,
            vertex_buffer: None,
            config: None,
            shapes: Vec::new(),
            num_vertices: 0,
            uniform_buffer: None,
            uniform_bind_group: None,
            mouse_state: MouseState { position: [0.0, 0.0] },
        }
    }

    fn update_uniform_buffer(&self, width: u32, height: u32) {
        if let (Some(queue), Some(uniform_buffer)) = (&self.queue, &self.uniform_buffer) {
            let aspect_ratio = height as f32 / width as f32;
            let uniforms = Uniforms { aspect_ratio };
            queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        }
    }


    async fn initialize_wgpu(&mut self, window: Arc<Window>) {
        // Create instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        
        // Create surface
        let surface = instance.create_surface(window.clone()).unwrap();

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // Create device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                },
                None,
            )
            .await
            .expect("Failed to create device");

        // Update the circle creation with a smaller radius
        self.shapes = vec![Shape::Circle {
            center: [0.0, 0.0],
            radius: 0.05, // Smaller radius (was 0.5)
            segments: 32,
        }];

        // Generate vertices for all shapes
        let mut vertices = Vec::new();
        for shape in &self.shapes {
            vertices.extend(shape.generate_vertices());
        }
        self.num_vertices = vertices.len() as u32;

        // Create vertex buffer with the generated vertices
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Create shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        // Create uniform buffer
        let uniforms = Uniforms {
            aspect_ratio: window.inner_size().height as f32 / window.inner_size().width as f32,
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Create bind group
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Update pipeline layout to include the bind group layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Get preferred surface format
        let swapchain_format = surface.get_capabilities(&adapter).formats[0];

        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(swapchain_format.into())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Configure surface
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &config);

        // Store everything
        self.window = Some(window);
        self.surface = Some(surface);
        self.device = Some(device);
        self.queue = Some(queue);
        self.render_pipeline = Some(render_pipeline);
        self.vertex_buffer = Some(vertex_buffer);
        self.config = Some(config);
        self.uniform_buffer = Some(uniform_buffer);
        self.uniform_bind_group = Some(uniform_bind_group);
    }

    fn render_frame(&self) {
        if let (Some(surface), Some(device), Some(queue), Some(render_pipeline), Some(vertex_buffer), Some(uniform_bind_group)) =
            (&self.surface, &self.device, &self.queue, &self.render_pipeline, &self.vertex_buffer, &self.uniform_bind_group)
        {
            // Get texture for current frame
            let frame = surface
                .get_current_texture()
                .expect("Failed to acquire next swap chain texture");
            
            // Create texture view
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            // Create command encoder
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

            // Begin render pass
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                render_pass.set_pipeline(render_pipeline);
                render_pass.set_bind_group(0, uniform_bind_group, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.draw(0..self.num_vertices, 0..1);
            }

            // Submit command buffer and present frame
            queue.submit(Some(encoder.finish()));
            frame.present();
        }
    }


}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());
        pollster::block_on(self.initialize_wgpu(window));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                if let (Some(surface), Some(device), Some(config)) =
                    (&self.surface, &self.device, &mut self.config)
                {
                    config.width = new_size.width.max(1);
                    config.height = new_size.height.max(1);
                    surface.configure(device, config);
                    // Update aspect ratio when window is resized
                    self.update_uniform_buffer(new_size.width, new_size.height);
                    self.window.as_ref().unwrap().request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                self.render_frame();
            }
            WindowEvent::KeyboardInput { device_id: _, event, is_synthetic: _ } => {
                if event.state == winit::event::ElementState::Pressed {
                    println!("Key pressed: {:?}", event.physical_key);
                }
            }
            WindowEvent::CursorMoved { device_id: _, position } => {
                self.mouse_state.position = self.window_position_to_ndc(&position);
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    let _ = event_loop.run_app(&mut app);
}
