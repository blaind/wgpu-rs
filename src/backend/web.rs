use std::{
    collections::HashSet,
    fmt,
    future::Future,
    ops::Range,
    pin::Pin,
    task::{self, Poll},
};
use wasm_bindgen::prelude::*;

// We need to make a wrapper for some of the handle types returned by the web backend to make them
// implement `Send` and `Sync` to match native.
//
// SAFETY: All webgpu handle types in wasm32 are internally a `JsValue`, and `JsValue` is neither
// Send nor Sync.  Currently, wasm32 has no threading support so implementing `Send` or `Sync` for a
// type is (for now) harmless.  Eventually wasm32 will support threading, and depending on how this
// is integrated (or not integrated) with values like those in webgpu, this may become unsound.

//forse run from integration system

#[derive(Clone, Debug)]
pub(crate) struct Sendable<T>(T);
unsafe impl<T> Send for Sendable<T> {}
unsafe impl<T> Sync for Sendable<T> {}

pub(crate) struct Context(web_sys::Gpu);
unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl fmt::Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context").field("type", &"Web").finish()
    }
}

#[derive(Debug)]
pub(crate) struct ComputePass(web_sys::GpuComputePassEncoder);
#[derive(Debug)]
pub(crate) struct RenderPass(web_sys::GpuRenderPassEncoder);
#[derive(Debug)]
pub(crate) struct RenderBundleEncoder(web_sys::GpuRenderBundleEncoder);

// We need to assert that any future we return is Send to match the native API.
//
// This is safe on wasm32 *for now*, but similarly to the unsafe Send impls for the handle type
// wrappers, the full story for threading on wasm32 is still unfolding.

pub(crate) struct MakeSendFuture<F, M> {
    future: F,
    map: M,
}

impl<F: Future, M: Fn(F::Output) -> T, T> Future for MakeSendFuture<F, M> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context) -> Poll<Self::Output> {
        // This is safe because we have no Drop implementation to violate the Pin requirements and
        // do not provide any means of moving the inner future.
        unsafe {
            let this = self.get_unchecked_mut();
            match Pin::new_unchecked(&mut this.future).poll(cx) {
                task::Poll::Ready(value) => task::Poll::Ready((this.map)(value)),
                task::Poll::Pending => task::Poll::Pending,
            }
        }
    }
}

impl<F, M> MakeSendFuture<F, M> {
    fn new(future: F, map: M) -> Self {
        Self { future, map }
    }
}

unsafe impl<F, M> Send for MakeSendFuture<F, M> {}

impl crate::ComputePassInner<Context> for ComputePass {
    fn set_pipeline(&mut self, pipeline: &Sendable<web_sys::GpuComputePipeline>) {
        self.0.set_pipeline(&pipeline.0);
    }
    fn set_bind_group(
        &mut self,
        index: u32,
        bind_group: &Sendable<web_sys::GpuBindGroup>,
        offsets: &[wgt::DynamicOffset],
    ) {
        self.0
            .set_bind_group_with_u32_array_and_f64_and_dynamic_offsets_data_length(
                index,
                &bind_group.0,
                offsets,
                0f64,
                offsets.len() as u32,
            );
    }

    fn set_push_constants(&mut self, _offset: u32, _data: &[u8]) {
        panic!("PUSH_CONSTANTS feature must be enabled to call multi_draw_indexed_indirect")
    }

    fn insert_debug_marker(&mut self, _label: &str) {
        // Not available in gecko yet
        // self.0.insert_debug_marker(label);
    }

    fn push_debug_group(&mut self, _group_label: &str) {
        // Not available in gecko yet
        // self.0.push_debug_group(group_label);
    }

    fn pop_debug_group(&mut self) {
        // Not available in gecko yet
        // self.0.pop_debug_group();
    }

    fn dispatch(&mut self, x: u32, y: u32, z: u32) {
        self.0.dispatch_with_y_and_z(x, y, z);
    }
    fn dispatch_indirect(
        &mut self,
        indirect_buffer: &Sendable<web_sys::GpuBuffer>,
        indirect_offset: wgt::BufferAddress,
    ) {
        self.0
            .dispatch_indirect_with_f64(&indirect_buffer.0, indirect_offset as f64);
    }

    fn write_timestamp(&mut self, _query_set: &(), _query_index: u32) {
        // Not available in gecko yet
    }

    fn begin_pipeline_statistics_query(&mut self, _query_set: &(), _query_index: u32) {
        // Not available in gecko yet
    }

    fn end_pipeline_statistics_query(&mut self) {
        // Not available in gecko yet
    }
}

impl crate::RenderInner<Context> for RenderPass {
    fn set_pipeline(&mut self, pipeline: &Sendable<web_sys::GpuRenderPipeline>) {
        self.0.set_pipeline(&pipeline.0);
    }
    fn set_bind_group(
        &mut self,
        index: u32,
        bind_group: &Sendable<web_sys::GpuBindGroup>,
        offsets: &[wgt::DynamicOffset],
    ) {
        self.0
            .set_bind_group_with_u32_array_and_f64_and_dynamic_offsets_data_length(
                index,
                &bind_group.0,
                offsets,
                0f64,
                offsets.len() as u32,
            );
    }
    fn set_index_buffer(
        &mut self,
        buffer: &Sendable<web_sys::GpuBuffer>,
        _index_format: wgt::IndexFormat,
        offset: wgt::BufferAddress,
        size: Option<wgt::BufferSize>,
    ) {
        let mapped_size = match size {
            Some(s) => s.get() as f64,
            None => 0f64,
        };
        self.0
            .set_index_buffer_with_f64_and_f64(&buffer.0, offset as f64, mapped_size);
    }
    fn set_vertex_buffer(
        &mut self,
        slot: u32,
        buffer: &Sendable<web_sys::GpuBuffer>,
        offset: wgt::BufferAddress,
        size: Option<wgt::BufferSize>,
    ) {
        let mapped_size = match size {
            Some(s) => s.get() as f64,
            None => 0f64,
        };
        self.0
            .set_vertex_buffer_with_f64_and_f64(slot, &buffer.0, offset as f64, mapped_size);
    }
    fn set_push_constants(&mut self, _stages: wgt::ShaderStage, _offset: u32, _data: &[u8]) {
        panic!("PUSH_CONSTANTS feature must be enabled to call multi_draw_indexed_indirect")
    }
    fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        self.0
            .draw_with_instance_count_and_first_vertex_and_first_instance(
                vertices.end - vertices.start,
                instances.end - instances.start,
                vertices.start,
                instances.start,
            );
    }
    fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.0
            .draw_indexed_with_instance_count_and_first_index_and_base_vertex_and_first_instance(
                indices.end - indices.start,
                instances.end - instances.start,
                indices.start,
                base_vertex,
                instances.start,
            );
    }
    fn draw_indirect(
        &mut self,
        indirect_buffer: &Sendable<web_sys::GpuBuffer>,
        indirect_offset: wgt::BufferAddress,
    ) {
        self.0
            .draw_indirect_with_f64(&indirect_buffer.0, indirect_offset as f64);
    }
    fn draw_indexed_indirect(
        &mut self,
        indirect_buffer: &Sendable<web_sys::GpuBuffer>,
        indirect_offset: wgt::BufferAddress,
    ) {
        self.0
            .draw_indexed_indirect_with_f64(&indirect_buffer.0, indirect_offset as f64);
    }
    fn multi_draw_indirect(
        &mut self,
        _indirect_buffer: &Sendable<web_sys::GpuBuffer>,
        _indirect_offset: wgt::BufferAddress,
        _count: u32,
    ) {
        panic!("MULTI_DRAW_INDIRECT feature must be enabled to call multi_draw_indirect")
    }
    fn multi_draw_indexed_indirect(
        &mut self,
        _indirect_buffer: &Sendable<web_sys::GpuBuffer>,
        _indirect_offset: wgt::BufferAddress,
        _count: u32,
    ) {
        panic!("MULTI_DRAW_INDIRECT feature must be enabled to call multi_draw_indexed_indirect")
    }
    fn multi_draw_indirect_count(
        &mut self,
        _indirect_buffer: &Sendable<web_sys::GpuBuffer>,
        _indirect_offset: wgt::BufferAddress,
        _count_buffer: &Sendable<web_sys::GpuBuffer>,
        _count_buffer_offset: wgt::BufferAddress,
        _max_count: u32,
    ) {
        panic!(
            "MULTI_DRAW_INDIRECT_COUNT feature must be enabled to call multi_draw_indirect_count"
        )
    }
    fn multi_draw_indexed_indirect_count(
        &mut self,
        _indirect_buffer: &Sendable<web_sys::GpuBuffer>,
        _indirect_offset: wgt::BufferAddress,
        _count_buffer: &Sendable<web_sys::GpuBuffer>,
        _count_buffer_offset: wgt::BufferAddress,
        _max_count: u32,
    ) {
        panic!("MULTI_DRAW_INDIRECT_COUNT feature must be enabled to call multi_draw_indexed_indirect_count")
    }
}

impl crate::RenderInner<Context> for RenderBundleEncoder {
    fn set_pipeline(&mut self, pipeline: &Sendable<web_sys::GpuRenderPipeline>) {
        self.0.set_pipeline(&pipeline.0);
    }
    fn set_bind_group(
        &mut self,
        index: u32,
        bind_group: &Sendable<web_sys::GpuBindGroup>,
        offsets: &[wgt::DynamicOffset],
    ) {
        self.0
            .set_bind_group_with_u32_array_and_f64_and_dynamic_offsets_data_length(
                index,
                &bind_group.0,
                offsets,
                0f64,
                offsets.len() as u32,
            );
    }
    fn set_index_buffer(
        &mut self,
        buffer: &Sendable<web_sys::GpuBuffer>,
        _index_format: wgt::IndexFormat,
        offset: wgt::BufferAddress,
        size: Option<wgt::BufferSize>,
    ) {
        let mapped_size = match size {
            Some(s) => s.get() as f64,
            None => 0f64,
        };
        self.0
            .set_index_buffer_with_f64_and_f64(&buffer.0, offset as f64, mapped_size);
    }
    fn set_vertex_buffer(
        &mut self,
        slot: u32,
        buffer: &Sendable<web_sys::GpuBuffer>,
        offset: wgt::BufferAddress,
        size: Option<wgt::BufferSize>,
    ) {
        let mapped_size = match size {
            Some(s) => s.get() as f64,
            None => 0f64,
        };
        self.0
            .set_vertex_buffer_with_f64_and_f64(slot, &buffer.0, offset as f64, mapped_size);
    }
    fn set_push_constants(&mut self, _stages: wgt::ShaderStage, _offset: u32, _data: &[u8]) {
        panic!("PUSH_CONSTANTS feature must be enabled to call multi_draw_indexed_indirect")
    }
    fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        self.0
            .draw_with_instance_count_and_first_vertex_and_first_instance(
                vertices.end - vertices.start,
                instances.end - instances.start,
                vertices.start,
                instances.start,
            );
    }
    fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.0
            .draw_indexed_with_instance_count_and_first_index_and_base_vertex_and_first_instance(
                indices.end - indices.start,
                instances.end - instances.start,
                indices.start,
                base_vertex,
                instances.start,
            );
    }
    fn draw_indirect(
        &mut self,
        indirect_buffer: &Sendable<web_sys::GpuBuffer>,
        indirect_offset: wgt::BufferAddress,
    ) {
        self.0
            .draw_indirect_with_f64(&indirect_buffer.0, indirect_offset as f64);
    }
    fn draw_indexed_indirect(
        &mut self,
        indirect_buffer: &Sendable<web_sys::GpuBuffer>,
        indirect_offset: wgt::BufferAddress,
    ) {
        self.0
            .draw_indexed_indirect_with_f64(&indirect_buffer.0, indirect_offset as f64);
    }
    fn multi_draw_indirect(
        &mut self,
        _indirect_buffer: &Sendable<web_sys::GpuBuffer>,
        _indirect_offset: wgt::BufferAddress,
        _count: u32,
    ) {
        panic!("MULTI_DRAW_INDIRECT feature must be enabled to call multi_draw_indirect")
    }
    fn multi_draw_indexed_indirect(
        &mut self,
        _indirect_buffer: &Sendable<web_sys::GpuBuffer>,
        _indirect_offset: wgt::BufferAddress,
        _count: u32,
    ) {
        panic!("MULTI_DRAW_INDIRECT feature must be enabled to call multi_draw_indexed_indirect")
    }
    fn multi_draw_indirect_count(
        &mut self,
        _indirect_buffer: &Sendable<web_sys::GpuBuffer>,
        _indirect_offset: wgt::BufferAddress,
        _count_buffer: &Sendable<web_sys::GpuBuffer>,
        _count_buffer_offset: wgt::BufferAddress,
        _max_count: u32,
    ) {
        panic!(
            "MULTI_DRAW_INDIRECT_COUNT feature must be enabled to call multi_draw_indirect_count"
        )
    }
    fn multi_draw_indexed_indirect_count(
        &mut self,
        _indirect_buffer: &Sendable<web_sys::GpuBuffer>,
        _indirect_offset: wgt::BufferAddress,
        _count_buffer: &Sendable<web_sys::GpuBuffer>,
        _count_buffer_offset: wgt::BufferAddress,
        _max_count: u32,
    ) {
        panic!("MULTI_DRAW_INDIRECT_COUNT feature must be enabled to call multi_draw_indexed_indirect_count")
    }
}

impl crate::RenderPassInner<Context> for RenderPass {
    fn set_blend_color(&mut self, color: wgt::Color) {
        self.0
            .set_blend_color_with_gpu_color_dict(&map_color(color));
    }
    fn set_scissor_rect(&mut self, x: u32, y: u32, width: u32, height: u32) {
        self.0.set_scissor_rect(x, y, width, height);
    }
    fn set_viewport(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        min_depth: f32,
        max_depth: f32,
    ) {
        self.0
            .set_viewport(x, y, width, height, min_depth, max_depth);
    }
    fn set_stencil_reference(&mut self, reference: u32) {
        self.0.set_stencil_reference(reference);
    }

    fn insert_debug_marker(&mut self, _label: &str) {
        // Not available in gecko yet
        // self.0.insert_debug_marker(label);
    }

    fn push_debug_group(&mut self, _group_label: &str) {
        // Not available in gecko yet
        // self.0.push_debug_group(group_label);
    }

    fn pop_debug_group(&mut self) {
        // Not available in gecko yet
        // self.0.pop_debug_group();
    }

    fn execute_bundles<'a, I: Iterator<Item = &'a Sendable<web_sys::GpuRenderBundle>>>(
        &mut self,
        render_bundles: I,
    ) {
        let mapped = render_bundles
            .map(|bundle| &bundle.0)
            .collect::<js_sys::Array>();
        self.0.execute_bundles(&mapped);
    }

    fn write_timestamp(&mut self, _query_set: &(), _query_index: u32) {
        // Not available in gecko yet
    }

    fn begin_pipeline_statistics_query(&mut self, _query_set: &(), _query_index: u32) {
        // Not available in gecko yet
    }

    fn end_pipeline_statistics_query(&mut self) {
        // Not available in gecko yet
    }
}

fn map_texture_format(texture_format: wgt::TextureFormat) -> web_sys::GpuTextureFormat {
    use web_sys::GpuTextureFormat as tf;
    use wgt::TextureFormat;
    match texture_format {
        TextureFormat::R8Unorm => tf::R8unorm,
        TextureFormat::R8Snorm => tf::R8snorm,
        TextureFormat::R8Uint => tf::R8uint,
        TextureFormat::R8Sint => tf::R8sint,
        TextureFormat::R16Uint => tf::R16uint,
        TextureFormat::R16Sint => tf::R16sint,
        TextureFormat::R16Float => tf::R16float,
        TextureFormat::Rg8Unorm => tf::Rg8unorm,
        TextureFormat::Rg8Snorm => tf::Rg8snorm,
        TextureFormat::Rg8Uint => tf::Rg8uint,
        TextureFormat::Rg8Sint => tf::Rg8sint,
        TextureFormat::R32Uint => tf::R32uint,
        TextureFormat::R32Sint => tf::R32sint,
        TextureFormat::R32Float => tf::R32float,
        TextureFormat::Rg16Uint => tf::Rg16uint,
        TextureFormat::Rg16Sint => tf::Rg16sint,
        TextureFormat::Rg16Float => tf::Rg16float,
        TextureFormat::Rgba8Unorm => tf::Rgba8unorm,
        TextureFormat::Rgba8UnormSrgb => tf::Rgba8unormSrgb,
        TextureFormat::Rgba8Snorm => tf::Rgba8snorm,
        TextureFormat::Rgba8Uint => tf::Rgba8uint,
        TextureFormat::Rgba8Sint => tf::Rgba8sint,
        TextureFormat::Bgra8Unorm => tf::Bgra8unorm,
        TextureFormat::Bgra8UnormSrgb => tf::Bgra8unormSrgb,
        TextureFormat::Rgb10a2Unorm => tf::Rgb10a2unorm,
        TextureFormat::Rg11b10Float => tf::Rg11b10ufloat,
        TextureFormat::Rg32Uint => tf::Rg32uint,
        TextureFormat::Rg32Sint => tf::Rg32sint,
        TextureFormat::Rg32Float => tf::Rg32float,
        TextureFormat::Rgba16Uint => tf::Rgba16uint,
        TextureFormat::Rgba16Sint => tf::Rgba16sint,
        TextureFormat::Rgba16Float => tf::Rgba16float,
        TextureFormat::Rgba32Uint => tf::Rgba32uint,
        TextureFormat::Rgba32Sint => tf::Rgba32sint,
        TextureFormat::Rgba32Float => tf::Rgba32float,
        TextureFormat::Depth32Float => tf::Depth32float,
        TextureFormat::Depth24Plus => tf::Depth24plus,
        TextureFormat::Depth24PlusStencil8 => tf::Depth24plusStencil8,
        _ => unimplemented!(),
    }
}

fn map_texture_component_type(
    sample_type: wgt::TextureSampleType,
) -> web_sys::GpuTextureComponentType {
    match sample_type {
        wgt::TextureSampleType::Float { .. } => web_sys::GpuTextureComponentType::Float,
        wgt::TextureSampleType::Sint => web_sys::GpuTextureComponentType::Sint,
        wgt::TextureSampleType::Uint => web_sys::GpuTextureComponentType::Uint,
        wgt::TextureSampleType::Depth => web_sys::GpuTextureComponentType::DepthComparison,
    }
}

fn map_cull_mode(cull_mode: Option<wgt::Face>) -> web_sys::GpuCullMode {
    use web_sys::GpuCullMode as cm;
    use wgt::Face;
    match cull_mode {
        None => cm::None,
        Some(Face::Front) => cm::Front,
        Some(Face::Back) => cm::Back,
    }
}

fn map_front_face(front_face: wgt::FrontFace) -> web_sys::GpuFrontFace {
    use web_sys::GpuFrontFace as ff;
    use wgt::FrontFace;
    match front_face {
        FrontFace::Ccw => ff::Ccw,
        FrontFace::Cw => ff::Cw,
    }
}

fn map_rasterization_state_descriptor(
    primitive: &wgt::PrimitiveState,
    ds: Option<&wgt::DepthStencilState>,
) -> web_sys::GpuRasterizationStateDescriptor {
    let mut mapped = web_sys::GpuRasterizationStateDescriptor::new();
    mapped.front_face(map_front_face(primitive.front_face));
    mapped.cull_mode(map_cull_mode(primitive.cull_mode));
    let bias = ds.map_or(wgt::DepthBiasState::default(), |ds| ds.bias.clone());
    mapped.depth_bias(bias.constant);
    mapped.depth_bias_clamp(bias.clamp);
    mapped.depth_bias_slope_scale(bias.slope_scale);
    mapped
}

fn map_compare_function(compare_fn: wgt::CompareFunction) -> web_sys::GpuCompareFunction {
    use web_sys::GpuCompareFunction as cf;
    use wgt::CompareFunction;
    match compare_fn {
        CompareFunction::Never => cf::Never,
        CompareFunction::Less => cf::Less,
        CompareFunction::Equal => cf::Equal,
        CompareFunction::LessEqual => cf::LessEqual,
        CompareFunction::Greater => cf::Greater,
        CompareFunction::NotEqual => cf::NotEqual,
        CompareFunction::GreaterEqual => cf::GreaterEqual,
        CompareFunction::Always => cf::Always,
    }
}

fn map_stencil_operation(op: wgt::StencilOperation) -> web_sys::GpuStencilOperation {
    use web_sys::GpuStencilOperation as so;
    use wgt::StencilOperation;
    match op {
        StencilOperation::Keep => so::Keep,
        StencilOperation::Zero => so::Zero,
        StencilOperation::Replace => so::Replace,
        StencilOperation::Invert => so::Invert,
        StencilOperation::IncrementClamp => so::IncrementClamp,
        StencilOperation::DecrementClamp => so::DecrementClamp,
        StencilOperation::IncrementWrap => so::IncrementWrap,
        StencilOperation::DecrementWrap => so::DecrementWrap,
    }
}

fn map_stencil_state_face_descriptor(
    desc: &wgt::StencilFaceState,
) -> web_sys::GpuStencilStateFaceDescriptor {
    let mut mapped = web_sys::GpuStencilStateFaceDescriptor::new();
    mapped.compare(map_compare_function(desc.compare));
    mapped.depth_fail_op(map_stencil_operation(desc.depth_fail_op));
    mapped.fail_op(map_stencil_operation(desc.fail_op));
    mapped.pass_op(map_stencil_operation(desc.pass_op));
    mapped
}

fn map_depth_stencil_state_descriptor(
    desc: &wgt::DepthStencilState,
) -> web_sys::GpuDepthStencilStateDescriptor {
    let mut mapped = web_sys::GpuDepthStencilStateDescriptor::new(map_texture_format(desc.format));
    mapped.depth_compare(map_compare_function(desc.depth_compare));
    mapped.depth_write_enabled(desc.depth_write_enabled);
    mapped.stencil_back(&map_stencil_state_face_descriptor(&desc.stencil.back));
    mapped.stencil_front(&map_stencil_state_face_descriptor(&desc.stencil.front));
    mapped.stencil_read_mask(desc.stencil.read_mask);
    mapped.stencil_write_mask(desc.stencil.write_mask);
    mapped
}

fn map_blend_descriptor(desc: &wgt::BlendState) -> web_sys::GpuBlendDescriptor {
    let mut mapped = web_sys::GpuBlendDescriptor::new();
    mapped.dst_factor(map_blend_factor(desc.dst_factor));
    mapped.operation(map_blend_operation(desc.operation));
    mapped.src_factor(map_blend_factor(desc.src_factor));
    mapped
}

fn map_blend_factor(factor: wgt::BlendFactor) -> web_sys::GpuBlendFactor {
    use web_sys::GpuBlendFactor as bf;
    use wgt::BlendFactor;
    match factor {
        BlendFactor::Zero => bf::Zero,
        BlendFactor::One => bf::One,
        BlendFactor::SrcColor => bf::SrcColor,
        BlendFactor::OneMinusSrcColor => bf::OneMinusSrcColor,
        BlendFactor::SrcAlpha => bf::SrcAlpha,
        BlendFactor::OneMinusSrcAlpha => bf::OneMinusSrcAlpha,
        BlendFactor::DstColor => bf::DstColor,
        BlendFactor::OneMinusDstColor => bf::OneMinusDstColor,
        BlendFactor::DstAlpha => bf::DstAlpha,
        BlendFactor::OneMinusDstAlpha => bf::OneMinusDstAlpha,
        BlendFactor::SrcAlphaSaturated => bf::SrcAlphaSaturated,
        BlendFactor::BlendColor => bf::BlendColor,
        BlendFactor::OneMinusBlendColor => bf::OneMinusBlendColor,
    }
}

fn map_blend_operation(op: wgt::BlendOperation) -> web_sys::GpuBlendOperation {
    use web_sys::GpuBlendOperation as bo;
    use wgt::BlendOperation;
    match op {
        BlendOperation::Add => bo::Add,
        BlendOperation::Subtract => bo::Subtract,
        BlendOperation::ReverseSubtract => bo::ReverseSubtract,
        BlendOperation::Min => bo::Min,
        BlendOperation::Max => bo::Max,
    }
}

fn map_index_format(format: wgt::IndexFormat) -> web_sys::GpuIndexFormat {
    use web_sys::GpuIndexFormat as f;
    use wgt::IndexFormat;
    match format {
        IndexFormat::Uint16 => f::Uint16,
        IndexFormat::Uint32 => f::Uint32,
    }
}

fn map_vertex_format(format: wgt::VertexFormat) -> web_sys::GpuVertexFormat {
    use web_sys::GpuVertexFormat as vf;
    use wgt::VertexFormat;
    match format {
        VertexFormat::Uchar2 => vf::Uchar2,
        VertexFormat::Uchar4 => vf::Uchar4,
        VertexFormat::Char2 => vf::Char2,
        VertexFormat::Char4 => vf::Char4,
        VertexFormat::Uchar2Norm => vf::Uchar2norm,
        VertexFormat::Uchar4Norm => vf::Uchar4norm,
        VertexFormat::Char2Norm => vf::Char2norm,
        VertexFormat::Char4Norm => vf::Char4norm,
        VertexFormat::Ushort2 => vf::Ushort2,
        VertexFormat::Ushort4 => vf::Ushort4,
        VertexFormat::Short2 => vf::Short2,
        VertexFormat::Short4 => vf::Short4,
        VertexFormat::Ushort2Norm => vf::Ushort2norm,
        VertexFormat::Ushort4Norm => vf::Ushort4norm,
        VertexFormat::Short2Norm => vf::Short2norm,
        VertexFormat::Short4Norm => vf::Short4norm,
        VertexFormat::Half2 => vf::Half2,
        VertexFormat::Half4 => vf::Half4,
        VertexFormat::Float => vf::Float,
        VertexFormat::Float2 => vf::Float2,
        VertexFormat::Float3 => vf::Float3,
        VertexFormat::Float4 => vf::Float4,
        VertexFormat::Uint => vf::Uint,
        VertexFormat::Uint2 => vf::Uint2,
        VertexFormat::Uint3 => vf::Uint3,
        VertexFormat::Uint4 => vf::Uint4,
        VertexFormat::Int => vf::Int,
        VertexFormat::Int2 => vf::Int2,
        VertexFormat::Int3 => vf::Int3,
        VertexFormat::Int4 => vf::Int4,
        VertexFormat::Double
        | VertexFormat::Double2
        | VertexFormat::Double3
        | VertexFormat::Double4 => {
            panic!("VERTEX_ATTRIBUTE_64BIT feature must be enabled to use Double formats")
        }
    }
}

fn map_input_step_mode(mode: wgt::InputStepMode) -> web_sys::GpuInputStepMode {
    use web_sys::GpuInputStepMode as sm;
    use wgt::InputStepMode;
    match mode {
        InputStepMode::Vertex => sm::Vertex,
        InputStepMode::Instance => sm::Instance,
    }
}

fn map_vertex_state_descriptor(
    desc: &crate::RenderPipelineDescriptor,
) -> web_sys::GpuVertexStateDescriptor {
    let mapped_vertex_buffers = desc
        .vertex
        .buffers
        .iter()
        .map(|vbuf| {
            let mapped_attributes = vbuf
                .attributes
                .iter()
                .map(|attr| {
                    web_sys::GpuVertexAttributeDescriptor::new(
                        map_vertex_format(attr.format),
                        attr.offset as f64,
                        attr.shader_location,
                    )
                })
                .collect::<js_sys::Array>();

            let mut mapped_vbuf = web_sys::GpuVertexBufferLayoutDescriptor::new(
                vbuf.array_stride as f64,
                &mapped_attributes,
            );
            mapped_vbuf.step_mode(map_input_step_mode(vbuf.step_mode));
            mapped_vbuf
        })
        .collect::<js_sys::Array>();

    let mut mapped = web_sys::GpuVertexStateDescriptor::new();
    mapped.index_format(
        desc.primitive
            .strip_index_format
            .map_or(web_sys::GpuIndexFormat::Uint16, map_index_format),
    );
    mapped.vertex_buffers(&mapped_vertex_buffers);
    mapped
}

fn map_extent_3d(extent: wgt::Extent3d) -> web_sys::GpuExtent3dDict {
    let mut mapped = web_sys::GpuExtent3dDict::new();
    mapped.depth(extent.depth);
    mapped.height(extent.height);
    mapped.width(extent.width);
    mapped
}

fn map_origin_3d(origin: wgt::Origin3d) -> web_sys::GpuOrigin3dDict {
    let mut mapped = web_sys::GpuOrigin3dDict::new();
    mapped.x(origin.x);
    mapped.y(origin.y);
    mapped.z(origin.z);
    mapped
}

fn map_texture_dimension(texture_dimension: wgt::TextureDimension) -> web_sys::GpuTextureDimension {
    match texture_dimension {
        wgt::TextureDimension::D1 => web_sys::GpuTextureDimension::N1d,
        wgt::TextureDimension::D2 => web_sys::GpuTextureDimension::N2d,
        wgt::TextureDimension::D3 => web_sys::GpuTextureDimension::N3d,
    }
}

fn map_texture_view_dimension(
    texture_view_dimension: wgt::TextureViewDimension,
) -> web_sys::GpuTextureViewDimension {
    use web_sys::GpuTextureViewDimension as tvd;
    match texture_view_dimension {
        wgt::TextureViewDimension::D1 => tvd::N1d,
        wgt::TextureViewDimension::D2 => tvd::N2d,
        wgt::TextureViewDimension::D2Array => tvd::N2dArray,
        wgt::TextureViewDimension::Cube => tvd::Cube,
        wgt::TextureViewDimension::CubeArray => tvd::CubeArray,
        wgt::TextureViewDimension::D3 => tvd::N3d,
    }
}

fn map_buffer_copy_view(view: crate::BufferCopyView) -> web_sys::GpuBufferCopyView {
    let mut mapped = web_sys::GpuBufferCopyView::new(&view.buffer.id.0);
    mapped.bytes_per_row(view.layout.bytes_per_row);
    mapped.rows_per_image(view.layout.rows_per_image);
    mapped.offset(view.layout.offset as f64);
    mapped
}

fn map_texture_copy_view(view: crate::TextureCopyView) -> web_sys::GpuTextureCopyView {
    let mut mapped = web_sys::GpuTextureCopyView::new(&view.texture.id.0);
    mapped.mip_level(view.mip_level);
    mapped.origin(&map_origin_3d(view.origin));
    mapped
}

fn map_texture_aspect(aspect: wgt::TextureAspect) -> web_sys::GpuTextureAspect {
    match aspect {
        wgt::TextureAspect::All => web_sys::GpuTextureAspect::All,
        wgt::TextureAspect::StencilOnly => web_sys::GpuTextureAspect::StencilOnly,
        wgt::TextureAspect::DepthOnly => web_sys::GpuTextureAspect::DepthOnly,
    }
}

fn map_filter_mode(mode: wgt::FilterMode) -> web_sys::GpuFilterMode {
    match mode {
        wgt::FilterMode::Nearest => web_sys::GpuFilterMode::Nearest,
        wgt::FilterMode::Linear => web_sys::GpuFilterMode::Linear,
    }
}

fn map_address_mode(mode: wgt::AddressMode) -> web_sys::GpuAddressMode {
    match mode {
        wgt::AddressMode::ClampToEdge => web_sys::GpuAddressMode::ClampToEdge,
        wgt::AddressMode::Repeat => web_sys::GpuAddressMode::Repeat,
        wgt::AddressMode::MirrorRepeat => web_sys::GpuAddressMode::MirrorRepeat,
        wgt::AddressMode::ClampToBorder => unimplemented!(),
    }
}

fn map_color(color: wgt::Color) -> web_sys::GpuColorDict {
    web_sys::GpuColorDict::new(color.a, color.b, color.g, color.r)
}

fn map_store_op(store: bool) -> web_sys::GpuStoreOp {
    if store {
        web_sys::GpuStoreOp::Store
    } else {
        web_sys::GpuStoreOp::Clear
    }
}

fn map_map_mode(mode: crate::MapMode) -> u32 {
    match mode {
        crate::MapMode::Read => web_sys::GpuMapMode::READ,
        crate::MapMode::Write => web_sys::GpuMapMode::WRITE,
    }
}

type JsFutureResult = Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;

fn future_request_adapter(result: JsFutureResult) -> Option<Sendable<web_sys::GpuAdapter>> {
    match result {
        Ok(js_value) => Some(Sendable(web_sys::GpuAdapter::from(js_value))),
        Err(_) => None,
    }
}
fn future_request_device(
    result: JsFutureResult,
) -> Result<(Sendable<web_sys::GpuDevice>, Sendable<web_sys::GpuQueue>), crate::RequestDeviceError>
{
    result
        .map(|js_value| {
            let device_id = web_sys::GpuDevice::from(js_value);
            let queue_id = device_id.default_queue();
            (Sendable(device_id), Sendable(queue_id))
        })
        .map_err(|_| crate::RequestDeviceError)
}

fn future_map_async(result: JsFutureResult) -> Result<(), crate::BufferAsyncError> {
    result.map(|_| ()).map_err(|_| crate::BufferAsyncError)
}

impl crate::Context for Context {
    type AdapterId = Sendable<web_sys::GpuAdapter>;
    type DeviceId = Sendable<web_sys::GpuDevice>;
    type QueueId = Sendable<web_sys::GpuQueue>;
    type ShaderModuleId = Sendable<web_sys::GpuShaderModule>;
    type BindGroupLayoutId = Sendable<web_sys::GpuBindGroupLayout>;
    type BindGroupId = Sendable<web_sys::GpuBindGroup>;
    type TextureViewId = Sendable<web_sys::GpuTextureView>;
    type SamplerId = Sendable<web_sys::GpuSampler>;
    type BufferId = Sendable<web_sys::GpuBuffer>;
    type TextureId = Sendable<web_sys::GpuTexture>;
    type QuerySetId = (); //TODO!
    type PipelineLayoutId = Sendable<web_sys::GpuPipelineLayout>;
    type RenderPipelineId = Sendable<web_sys::GpuRenderPipeline>;
    type ComputePipelineId = Sendable<web_sys::GpuComputePipeline>;
    type CommandEncoderId = web_sys::GpuCommandEncoder;
    type ComputePassId = ComputePass;
    type RenderPassId = RenderPass;
    type CommandBufferId = Sendable<web_sys::GpuCommandBuffer>;
    type RenderBundleEncoderId = RenderBundleEncoder;
    type RenderBundleId = Sendable<web_sys::GpuRenderBundle>;
    type SurfaceId = Sendable<web_sys::GpuCanvasContext>;
    type SwapChainId = Sendable<web_sys::GpuSwapChain>;

    type SwapChainOutputDetail = SwapChainOutputDetail;

    type RequestAdapterFuture = MakeSendFuture<
        wasm_bindgen_futures::JsFuture,
        fn(JsFutureResult) -> Option<Self::AdapterId>,
    >;
    type RequestDeviceFuture = MakeSendFuture<
        wasm_bindgen_futures::JsFuture,
        fn(JsFutureResult) -> Result<(Self::DeviceId, Self::QueueId), crate::RequestDeviceError>,
    >;
    type MapAsyncFuture = MakeSendFuture<
        wasm_bindgen_futures::JsFuture,
        fn(JsFutureResult) -> Result<(), crate::BufferAsyncError>,
    >;

    fn init(_backends: wgt::BackendBit) -> Self {
        Context(web_sys::window().unwrap().navigator().gpu())
    }

    fn instance_create_surface(
        &self,
        handle: &impl raw_window_handle::HasRawWindowHandle,
    ) -> Self::SurfaceId {
        let canvas_attribute = match handle.raw_window_handle() {
            raw_window_handle::RawWindowHandle::Web(web_handle) => web_handle.id,
            _ => panic!("expected valid handle for canvas"),
        };
        let canvas_node: wasm_bindgen::JsValue = web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                doc.query_selector_all(&format!("[data-raw-handle=\"{}\"]", canvas_attribute))
                    .ok()
            })
            .and_then(|nodes| nodes.get(0))
            .expect("expected to find single canvas")
            .into();
        let canvas_element: web_sys::HtmlCanvasElement = canvas_node.into();
        let context: wasm_bindgen::JsValue = match canvas_element.get_context("gpupresent") {
            Ok(Some(ctx)) => ctx.into(),
            _ => panic!("expected to get context from canvas"),
        };
        Sendable(context.into())
    }

    fn instance_request_adapter(
        &self,
        options: &crate::RequestAdapterOptions<'_>,
    ) -> Self::RequestAdapterFuture {
        //TODO: support this check, return `None` if the flag is not set.
        // It's not trivial, since we need the Future logic to have this check,
        // and currently the Future her has no room for extra parameter `backends`.
        //assert!(backends.contains(wgt::BackendBit::BROWSER_WEBGPU));
        let mut mapped_options = web_sys::GpuRequestAdapterOptions::new();
        let mapped_power_preference = match options.power_preference {
            wgt::PowerPreference::LowPower => web_sys::GpuPowerPreference::LowPower,
            wgt::PowerPreference::HighPerformance => web_sys::GpuPowerPreference::HighPerformance,
        };
        mapped_options.power_preference(mapped_power_preference);
        let adapter_promise = self.0.request_adapter_with_options(&mapped_options);

        MakeSendFuture::new(
            wasm_bindgen_futures::JsFuture::from(adapter_promise),
            future_request_adapter,
        )
    }

    fn instance_poll_all_devices(&self, _force_wait: bool) {
        // Devices are automatically polled.
    }

    fn adapter_request_device(
        &self,
        adapter: &Self::AdapterId,
        desc: &crate::DeviceDescriptor,
        trace_dir: Option<&std::path::Path>,
    ) -> Self::RequestDeviceFuture {
        if trace_dir.is_some() {
            //Error: Tracing isn't supported on the Web target
        }
        assert!(
            !desc.features.intersects(crate::Features::ALL_NATIVE),
            "The web backend doesn't support any native extensions. Enabled native extensions: {:?}",
            desc.features & crate::Features::ALL_NATIVE
        );
        let mut mapped_desc = web_sys::GpuDeviceDescriptor::new();
        // TODO: label, extensions
        let mut mapped_limits = web_sys::GpuLimits::new();
        mapped_limits.max_bind_groups(desc.limits.max_bind_groups);
        mapped_desc.limits(&mapped_limits);
        let device_promise = adapter.0.request_device_with_descriptor(&mapped_desc);

        MakeSendFuture::new(
            wasm_bindgen_futures::JsFuture::from(device_promise),
            future_request_device,
        )
    }

    fn adapter_get_swap_chain_preferred_format(
        &self,
        _adapter: &Self::AdapterId,
        _surface: &Self::SurfaceId,
    ) -> wgt::TextureFormat {
        // TODO: web-sys bindings need to be updated to not return a promise
        wgt::TextureFormat::Bgra8Unorm
    }

    fn adapter_features(&self, _adapter: &Self::AdapterId) -> wgt::Features {
        // TODO: web-sys has no way of getting extensions on adapters
        wgt::Features::empty()
    }

    fn adapter_limits(&self, _adapter: &Self::AdapterId) -> wgt::Limits {
        // TODO: web-sys has no way of getting limits on adapters
        wgt::Limits::default()
    }

    fn adapter_get_info(&self, _adapter: &Self::AdapterId) -> wgt::AdapterInfo {
        // TODO: web-sys has no way of getting information on adapters
        wgt::AdapterInfo {
            name: String::new(),
            vendor: 0,
            device: 0,
            device_type: wgt::DeviceType::Other,
            backend: wgt::Backend::BrowserWebGpu,
        }
    }

    fn adapter_get_texture_format_features(
        &self,
        _adapter: &Self::AdapterId,
        format: wgt::TextureFormat,
    ) -> wgt::TextureFormatFeatures {
        format.describe().guaranteed_format_features
    }

    fn device_features(&self, _device: &Self::DeviceId) -> wgt::Features {
        // TODO: web-sys has no way of getting extensions on devices
        wgt::Features::empty()
    }

    fn device_limits(&self, _device: &Self::DeviceId) -> wgt::Limits {
        // TODO: web-sys has a method for getting limits on devices, but it returns Object not GpuLimit
        wgt::Limits::default()
    }

    fn device_create_swap_chain(
        &self,
        device: &Self::DeviceId,
        surface: &Self::SurfaceId,
        desc: &wgt::SwapChainDescriptor,
    ) -> Self::SwapChainId {
        let mut mapped =
            web_sys::GpuSwapChainDescriptor::new(&device.0, map_texture_format(desc.format));
        mapped.usage(desc.usage.bits());
        Sendable(surface.0.configure_swap_chain(&mapped))
    }

    fn device_create_shader_module(
        &self,
        device: &Self::DeviceId,
        desc: &crate::ShaderModuleDescriptor,
    ) -> Self::ShaderModuleId {
        let mut descriptor = match desc.source {
            crate::ShaderSource::SpirV(ref spv) => {
                web_sys::GpuShaderModuleDescriptor::new(&js_sys::Uint32Array::from(&**spv))
            }
            crate::ShaderSource::Wgsl(ref code) => {
                use naga::{back::spv, front::wgsl};
                let module = wgsl::parse_str(code).unwrap();
                let mut capabilities = HashSet::default();
                capabilities.insert(spv::Capability::Shader);
                let words = spv::write_vec(&module, spv::WriterFlags::NONE, capabilities).unwrap();
                web_sys::GpuShaderModuleDescriptor::new(&js_sys::Uint32Array::from(&words[..]))
            }
        };
        if let Some(ref label) = desc.label {
            descriptor.label(label);
        }
        Sendable(device.0.create_shader_module(&descriptor))
    }

    fn device_create_bind_group_layout(
        &self,
        device: &Self::DeviceId,
        desc: &crate::BindGroupLayoutDescriptor,
    ) -> Self::BindGroupLayoutId {
        use web_sys::GpuBindingType as bt;

        let mapped_bindings = desc
            .entries
            .iter()
            .map(|bind| {
                let mapped_type = match bind.ty {
                    wgt::BindingType::Buffer {
                        ty: wgt::BufferBindingType::Uniform,
                        ..
                    } => bt::UniformBuffer,
                    wgt::BindingType::Buffer {
                        ty: wgt::BufferBindingType::Storage { read_only: false },
                        ..
                    } => bt::StorageBuffer,
                    wgt::BindingType::Buffer {
                        ty: wgt::BufferBindingType::Storage { read_only: true },
                        ..
                    } => bt::ReadonlyStorageBuffer,
                    wgt::BindingType::Sampler {
                        comparison: false, ..
                    } => bt::Sampler,
                    wgt::BindingType::Sampler { .. } => bt::ComparisonSampler,
                    wgt::BindingType::Texture {
                        multisampled: true, ..
                    } => bt::MultisampledTexture,
                    wgt::BindingType::Texture { .. } => bt::SampledTexture,
                    wgt::BindingType::StorageTexture {
                        access: wgt::StorageTextureAccess::ReadOnly,
                        ..
                    } => bt::ReadonlyStorageTexture,
                    wgt::BindingType::StorageTexture { .. } => bt::WriteonlyStorageTexture,
                };

                assert!(
                    bind.count.is_none(),
                    "The web backend doesn't support arrays of bindings"
                );

                let mut mapped_entry = web_sys::GpuBindGroupLayoutEntry::new(
                    bind.binding,
                    mapped_type,
                    bind.visibility.bits(),
                );

                if let wgt::BindingType::Buffer {
                    has_dynamic_offset, ..
                } = bind.ty
                {
                    mapped_entry.has_dynamic_offset(has_dynamic_offset);
                }

                if let wgt::BindingType::Texture { sample_type, .. } = bind.ty {
                    mapped_entry.texture_component_type(map_texture_component_type(sample_type));
                }

                match bind.ty {
                    wgt::BindingType::Texture { view_dimension, .. }
                    | wgt::BindingType::StorageTexture { view_dimension, .. } => {
                        mapped_entry.view_dimension(map_texture_view_dimension(view_dimension));
                    }
                    _ => {}
                }

                if let wgt::BindingType::StorageTexture { format, .. } = bind.ty {
                    mapped_entry.storage_texture_format(map_texture_format(format));
                }

                mapped_entry
            })
            .collect::<js_sys::Array>();

        let mut mapped_desc = web_sys::GpuBindGroupLayoutDescriptor::new(&mapped_bindings);
        if let Some(ref label) = desc.label {
            mapped_desc.label(label);
        }
        Sendable(device.0.create_bind_group_layout(&mapped_desc))
    }

    fn device_create_bind_group(
        &self,
        device: &Self::DeviceId,
        desc: &crate::BindGroupDescriptor,
    ) -> Self::BindGroupId {
        let mapped_entries = desc
            .entries
            .iter()
            .map(|binding| {
                let mapped_resource = match binding.resource {
                    crate::BindingResource::Buffer {
                        ref buffer,
                        offset,
                        size,
                    } => {
                        let mut mapped_buffer_binding =
                            web_sys::GpuBufferBinding::new(&buffer.id.0);
                        mapped_buffer_binding.offset(offset as f64);
                        if let Some(s) = size {
                            mapped_buffer_binding.size(s.get() as f64);
                        }
                        JsValue::from(mapped_buffer_binding.clone())
                    }
                    crate::BindingResource::Sampler(ref sampler) => {
                        JsValue::from(sampler.id.0.clone())
                    }
                    crate::BindingResource::TextureView(ref texture_view) => {
                        JsValue::from(texture_view.id.0.clone())
                    }
                    crate::BindingResource::TextureViewArray(..) => {
                        panic!("Web backend does not support BINDING_INDEXING extension")
                    }
                };

                web_sys::GpuBindGroupEntry::new(binding.binding, &mapped_resource)
            })
            .collect::<js_sys::Array>();

        let mut mapped_desc =
            web_sys::GpuBindGroupDescriptor::new(&mapped_entries, &desc.layout.id.0);
        if let Some(label) = desc.label {
            mapped_desc.label(label);
        }
        Sendable(device.0.create_bind_group(&mapped_desc))
    }

    fn device_create_pipeline_layout(
        &self,
        device: &Self::DeviceId,
        desc: &crate::PipelineLayoutDescriptor,
    ) -> Self::PipelineLayoutId {
        let temp_layouts = desc
            .bind_group_layouts
            .iter()
            .map(|bgl| bgl.id.0.clone())
            .collect::<js_sys::Array>();
        let mut mapped_desc = web_sys::GpuPipelineLayoutDescriptor::new(&temp_layouts);
        if let Some(label) = desc.label {
            mapped_desc.label(label);
        }
        Sendable(device.0.create_pipeline_layout(&mapped_desc))
    }

    fn device_create_render_pipeline(
        &self,
        device: &Self::DeviceId,
        desc: &crate::RenderPipelineDescriptor,
    ) -> Self::RenderPipelineId {
        use web_sys::GpuPrimitiveTopology as pt;

        let targets = desc.fragment.as_ref().map_or(&[][..], |frag| &frag.targets);
        let mapped_color_states = targets
            .iter()
            .map(|target| {
                let mapped_format = map_texture_format(target.format);
                let mut mapped_color_state_desc =
                    web_sys::GpuColorStateDescriptor::new(mapped_format);
                mapped_color_state_desc.alpha_blend(&map_blend_descriptor(&target.alpha_blend));
                mapped_color_state_desc.color_blend(&map_blend_descriptor(&target.color_blend));
                mapped_color_state_desc.write_mask(target.write_mask.bits());
                mapped_color_state_desc
            })
            .collect::<js_sys::Array>();

        let mapped_primitive_topology = match desc.primitive.topology {
            wgt::PrimitiveTopology::PointList => pt::PointList,
            wgt::PrimitiveTopology::LineList => pt::LineList,
            wgt::PrimitiveTopology::LineStrip => pt::LineStrip,
            wgt::PrimitiveTopology::TriangleList => pt::TriangleList,
            wgt::PrimitiveTopology::TriangleStrip => pt::TriangleStrip,
        };

        let mapped_vertex_stage = web_sys::GpuProgrammableStageDescriptor::new(
            &desc.vertex.entry_point,
            &desc.vertex.module.id.0,
        );

        let mut mapped_desc = web_sys::GpuRenderPipelineDescriptor::new(
            &mapped_color_states,
            mapped_primitive_topology,
            &mapped_vertex_stage,
        );
        if let Some(layout) = desc.layout {
            mapped_desc.layout(&layout.id.0);
        }

        // TODO: label

        if let Some(ref frag) = desc.fragment {
            let mapped_fragment_desc =
                web_sys::GpuProgrammableStageDescriptor::new(&frag.entry_point, &frag.module.id.0);
            mapped_desc.fragment_stage(&mapped_fragment_desc);
        }

        mapped_desc.rasterization_state(&map_rasterization_state_descriptor(
            &desc.primitive,
            desc.depth_stencil.as_ref(),
        ));

        if let Some(ref depth_stencil) = desc.depth_stencil {
            mapped_desc.depth_stencil_state(&map_depth_stencil_state_descriptor(depth_stencil));
        }

        mapped_desc.vertex_state(&map_vertex_state_descriptor(&desc));
        mapped_desc.sample_count(desc.multisample.count);
        mapped_desc.sample_mask(desc.multisample.mask as u32);
        mapped_desc.alpha_to_coverage_enabled(desc.multisample.alpha_to_coverage_enabled);

        Sendable(device.0.create_render_pipeline(&mapped_desc))
    }

    fn device_create_compute_pipeline(
        &self,
        device: &Self::DeviceId,
        desc: &crate::ComputePipelineDescriptor,
    ) -> Self::ComputePipelineId {
        let mapped_compute_stage =
            web_sys::GpuProgrammableStageDescriptor::new(&desc.entry_point, &desc.module.id.0);
        let mut mapped_desc = web_sys::GpuComputePipelineDescriptor::new(&mapped_compute_stage);
        if let Some(layout) = desc.layout {
            mapped_desc.layout(&layout.id.0);
        }
        if let Some(label) = desc.label {
            mapped_desc.label(label);
        }
        Sendable(device.0.create_compute_pipeline(&mapped_desc))
    }

    fn device_create_buffer(
        &self,
        device: &Self::DeviceId,
        desc: &crate::BufferDescriptor,
    ) -> Self::BufferId {
        let mut mapped_desc =
            web_sys::GpuBufferDescriptor::new(desc.size as f64, desc.usage.bits());
        mapped_desc.mapped_at_creation(desc.mapped_at_creation);
        if let Some(ref label) = desc.label {
            mapped_desc.label(label);
        }
        Sendable(device.0.create_buffer(&mapped_desc))
    }

    fn device_create_texture(
        &self,
        device: &Self::DeviceId,
        desc: &crate::TextureDescriptor,
    ) -> Self::TextureId {
        let mut mapped_desc = web_sys::GpuTextureDescriptor::new(
            map_texture_format(desc.format),
            &map_extent_3d(desc.size),
            desc.usage.bits(),
        );
        if let Some(ref label) = desc.label {
            mapped_desc.label(label);
        }
        mapped_desc.dimension(map_texture_dimension(desc.dimension));
        mapped_desc.mip_level_count(desc.mip_level_count);
        mapped_desc.sample_count(desc.sample_count);
        Sendable(device.0.create_texture(&mapped_desc))
    }

    fn device_create_sampler(
        &self,
        device: &Self::DeviceId,
        desc: &crate::SamplerDescriptor,
    ) -> Self::SamplerId {
        let mut mapped_desc = web_sys::GpuSamplerDescriptor::new();
        mapped_desc.address_mode_u(map_address_mode(desc.address_mode_u));
        mapped_desc.address_mode_v(map_address_mode(desc.address_mode_v));
        mapped_desc.address_mode_w(map_address_mode(desc.address_mode_w));
        if let Some(compare) = desc.compare {
            mapped_desc.compare(map_compare_function(compare));
        }
        mapped_desc.lod_max_clamp(desc.lod_max_clamp);
        mapped_desc.lod_min_clamp(desc.lod_min_clamp);
        mapped_desc.mag_filter(map_filter_mode(desc.mag_filter));
        mapped_desc.min_filter(map_filter_mode(desc.min_filter));
        mapped_desc.mipmap_filter(map_filter_mode(desc.mipmap_filter));
        if let Some(label) = desc.label {
            mapped_desc.label(label);
        }
        Sendable(device.0.create_sampler_with_descriptor(&mapped_desc))
    }

    fn device_create_query_set(
        &self,
        _device: &Self::DeviceId,
        _desc: &wgt::QuerySetDescriptor,
    ) -> Self::QuerySetId {
        ()
    }

    fn device_create_command_encoder(
        &self,
        device: &Self::DeviceId,
        desc: &crate::CommandEncoderDescriptor,
    ) -> Self::CommandEncoderId {
        let mut mapped_desc = web_sys::GpuCommandEncoderDescriptor::new();
        if let Some(ref label) = desc.label {
            mapped_desc.label(label);
        }
        device
            .0
            .create_command_encoder_with_descriptor(&mapped_desc)
    }

    fn device_create_render_bundle_encoder(
        &self,
        device: &Self::DeviceId,
        desc: &crate::RenderBundleEncoderDescriptor,
    ) -> Self::RenderBundleEncoderId {
        let mapped_color_formats = desc
            .color_formats
            .iter()
            .map(|cf| wasm_bindgen::JsValue::from(map_texture_format(*cf)))
            .collect::<js_sys::Array>();
        let mut mapped_desc = web_sys::GpuRenderBundleEncoderDescriptor::new(&mapped_color_formats);
        if let Some(ref label) = desc.label {
            mapped_desc.label(label);
        }
        if let Some(dsf) = desc.depth_stencil_format {
            mapped_desc.depth_stencil_format(map_texture_format(dsf));
        }
        mapped_desc.sample_count(desc.sample_count);
        RenderBundleEncoder(device.0.create_render_bundle_encoder(&mapped_desc))
    }

    fn device_drop(&self, _device: &Self::DeviceId) {
        // Device is dropped automatically
    }

    fn device_poll(&self, _device: &Self::DeviceId, _maintain: crate::Maintain) {
        // Device is polled automatically
    }

    fn device_on_uncaptured_error(
        &self,
        _device: &Self::DeviceId,
        _handler: impl crate::UncapturedErrorHandler,
    ) {
        // TODO:
    }

    fn buffer_map_async(
        &self,
        buffer: &Self::BufferId,
        mode: crate::MapMode,
        range: Range<wgt::BufferAddress>,
    ) -> Self::MapAsyncFuture {
        let map_promise = buffer.0.map_async_with_f64_and_f64(
            map_map_mode(mode),
            range.start as f64,
            (range.end - range.start) as f64,
        );

        MakeSendFuture::new(
            wasm_bindgen_futures::JsFuture::from(map_promise),
            future_map_async,
        )
    }

    fn buffer_get_mapped_range(
        &self,
        buffer: &Self::BufferId,
        sub_range: Range<wgt::BufferAddress>,
    ) -> BufferMappedRange {
        let array_buffer = buffer.0.get_mapped_range_with_f64_and_f64(
            sub_range.start as f64,
            (sub_range.end - sub_range.start) as f64,
        );
        let actual_mapping = js_sys::Uint8Array::new(&array_buffer);
        let temporary_mapping = actual_mapping.to_vec();
        BufferMappedRange {
            actual_mapping,
            temporary_mapping,
        }
    }

    fn buffer_unmap(&self, buffer: &Self::BufferId) {
        buffer.0.unmap();
    }

    fn swap_chain_get_current_texture_view(
        &self,
        swap_chain: &Self::SwapChainId,
    ) -> (
        Option<Self::TextureViewId>,
        wgt::SwapChainStatus,
        Self::SwapChainOutputDetail,
    ) {
        // TODO: Should we pass a descriptor here?
        // Or is the default view always correct?
        (
            Some(Sendable(swap_chain.0.get_current_texture().create_view())),
            wgt::SwapChainStatus::Good,
            (),
        )
    }

    fn swap_chain_present(
        &self,
        _view: &Self::TextureViewId,
        _detail: &Self::SwapChainOutputDetail,
    ) {
        // Swapchain is presented automatically
    }

    fn texture_create_view(
        &self,
        texture: &Self::TextureId,
        desc: &crate::TextureViewDescriptor,
    ) -> Self::TextureViewId {
        let mut mapped = web_sys::GpuTextureViewDescriptor::new();
        if let Some(dim) = desc.dimension {
            mapped.dimension(map_texture_view_dimension(dim));
        }
        if let Some(format) = desc.format {
            mapped.format(map_texture_format(format));
        }
        mapped.aspect(map_texture_aspect(desc.aspect));
        mapped.base_array_layer(desc.base_array_layer);
        if let Some(count) = desc.array_layer_count {
            mapped.array_layer_count(count.get());
        }
        mapped.base_mip_level(desc.base_mip_level);
        if let Some(count) = desc.level_count {
            mapped.mip_level_count(count.get());
        }
        if let Some(label) = desc.label {
            mapped.label(label);
        }
        Sendable(texture.0.create_view_with_descriptor(&mapped))
    }

    fn surface_drop(&self, _surface: &Self::SurfaceId) {
        // Dropped automatically
    }

    fn adapter_drop(&self, _adapter: &Self::AdapterId) {
        // Dropped automatically
    }

    fn buffer_destroy(&self, buffer: &Self::BufferId) {
        buffer.0.destroy();
    }

    fn buffer_drop(&self, _buffer: &Self::BufferId) {
        // Dropped automatically
    }

    fn texture_drop(&self, _texture: &Self::TextureId) {
        // Dropped automatically
    }

    fn texture_destroy(&self, texture: &Self::TextureId) {
        texture.0.destroy();
    }

    fn texture_view_drop(&self, _texture_view: &Self::TextureViewId) {
        // Dropped automatically
    }

    fn sampler_drop(&self, _sampler: &Self::SamplerId) {
        // Dropped automatically
    }

    fn query_set_drop(&self, _query_set: &Self::QuerySetId) {
        // Dropped automatically
    }

    fn bind_group_drop(&self, _bind_group: &Self::BindGroupId) {
        // Dropped automatically
    }

    fn bind_group_layout_drop(&self, _bind_group_layout: &Self::BindGroupLayoutId) {
        // Dropped automatically
    }

    fn pipeline_layout_drop(&self, _pipeline_layout: &Self::PipelineLayoutId) {
        // Dropped automatically
    }

    fn shader_module_drop(&self, _shader_module: &Self::ShaderModuleId) {
        // Dropped automatically
    }

    fn command_buffer_drop(&self, _command_buffer: &Self::CommandBufferId) {
        // Dropped automatically
    }

    fn render_bundle_drop(&self, _render_bundle: &Self::RenderBundleId) {
        // Dropped automatically
    }

    fn compute_pipeline_drop(&self, _pipeline: &Self::ComputePipelineId) {
        // Dropped automatically
    }

    fn render_pipeline_drop(&self, _pipeline: &Self::RenderPipelineId) {
        // Dropped automatically
    }

    fn compute_pipeline_get_bind_group_layout(
        &self,
        pipeline: &Self::ComputePipelineId,
        index: u32,
    ) -> Self::BindGroupLayoutId {
        Sendable(pipeline.0.get_bind_group_layout(index))
    }

    fn render_pipeline_get_bind_group_layout(
        &self,
        pipeline: &Self::RenderPipelineId,
        index: u32,
    ) -> Self::BindGroupLayoutId {
        Sendable(pipeline.0.get_bind_group_layout(index))
    }

    fn command_encoder_copy_buffer_to_buffer(
        &self,
        encoder: &Self::CommandEncoderId,
        source: &Self::BufferId,
        source_offset: wgt::BufferAddress,
        destination: &Self::BufferId,
        destination_offset: wgt::BufferAddress,
        copy_size: wgt::BufferAddress,
    ) {
        encoder.copy_buffer_to_buffer_with_f64_and_f64_and_f64(
            &source.0,
            source_offset as f64,
            &destination.0,
            destination_offset as f64,
            copy_size as f64,
        )
    }

    fn command_encoder_copy_buffer_to_texture(
        &self,
        encoder: &Self::CommandEncoderId,
        source: crate::BufferCopyView,
        destination: crate::TextureCopyView,
        copy_size: wgt::Extent3d,
    ) {
        encoder.copy_buffer_to_texture_with_gpu_extent_3d_dict(
            &map_buffer_copy_view(source),
            &map_texture_copy_view(destination),
            &map_extent_3d(copy_size),
        )
    }

    fn command_encoder_copy_texture_to_buffer(
        &self,
        encoder: &Self::CommandEncoderId,
        source: crate::TextureCopyView,
        destination: crate::BufferCopyView,
        copy_size: wgt::Extent3d,
    ) {
        encoder.copy_texture_to_buffer_with_gpu_extent_3d_dict(
            &map_texture_copy_view(source),
            &map_buffer_copy_view(destination),
            &map_extent_3d(copy_size),
        )
    }

    fn command_encoder_copy_texture_to_texture(
        &self,
        encoder: &Self::CommandEncoderId,
        source: crate::TextureCopyView,
        destination: crate::TextureCopyView,
        copy_size: wgt::Extent3d,
    ) {
        encoder.copy_texture_to_texture_with_gpu_extent_3d_dict(
            &map_texture_copy_view(source),
            &map_texture_copy_view(destination),
            &map_extent_3d(copy_size),
        )
    }

    fn command_encoder_begin_compute_pass(
        &self,
        encoder: &Self::CommandEncoderId,
        desc: &crate::ComputePassDescriptor,
    ) -> Self::ComputePassId {
        let mut mapped_desc = web_sys::GpuComputePassDescriptor::new();
        if let Some(ref label) = desc.label {
            mapped_desc.label(label);
        }
        ComputePass(encoder.begin_compute_pass_with_descriptor(&mapped_desc))
    }

    fn command_encoder_end_compute_pass(
        &self,
        _encoder: &Self::CommandEncoderId,
        pass: &mut Self::ComputePassId,
    ) {
        pass.0.end_pass();
    }

    fn command_encoder_begin_render_pass<'a>(
        &self,
        encoder: &Self::CommandEncoderId,
        desc: &crate::RenderPassDescriptor<'a, '_>,
    ) -> Self::RenderPassId {
        let mapped_color_attachments = desc
            .color_attachments
            .iter()
            .map(|ca| {
                let mut mapped_color_attachment =
                    web_sys::GpuRenderPassColorAttachmentDescriptor::new(
                        &ca.attachment.id.0,
                        &match ca.ops.load {
                            crate::LoadOp::Clear(color) => {
                                wasm_bindgen::JsValue::from(map_color(color))
                            }
                            crate::LoadOp::Load => {
                                wasm_bindgen::JsValue::from(web_sys::GpuLoadOp::Load)
                            }
                        },
                    );

                if let Some(rt) = ca.resolve_target {
                    mapped_color_attachment.resolve_target(&rt.id.0);
                }

                mapped_color_attachment.store_op(map_store_op(ca.ops.store));

                mapped_color_attachment
            })
            .collect::<js_sys::Array>();

        let mut mapped_desc = web_sys::GpuRenderPassDescriptor::new(&mapped_color_attachments);

        // TODO: label

        if let Some(dsa) = &desc.depth_stencil_attachment {
            let (depth_load_op, depth_store_op) = match dsa.depth_ops {
                Some(ref ops) => {
                    let load_op = match ops.load {
                        crate::LoadOp::Clear(value) => wasm_bindgen::JsValue::from(value),
                        crate::LoadOp::Load => {
                            wasm_bindgen::JsValue::from(web_sys::GpuLoadOp::Load)
                        }
                    };
                    (load_op, map_store_op(ops.store))
                }
                None => (
                    wasm_bindgen::JsValue::from(web_sys::GpuLoadOp::Load),
                    web_sys::GpuStoreOp::Store,
                ),
            };
            let (stencil_load_op, stencil_store_op) = match dsa.depth_ops {
                Some(ref ops) => {
                    let load_op = match ops.load {
                        crate::LoadOp::Clear(value) => wasm_bindgen::JsValue::from(value),
                        crate::LoadOp::Load => {
                            wasm_bindgen::JsValue::from(web_sys::GpuLoadOp::Load)
                        }
                    };
                    (load_op, map_store_op(ops.store))
                }
                None => (
                    wasm_bindgen::JsValue::from(web_sys::GpuLoadOp::Load),
                    web_sys::GpuStoreOp::Store,
                ),
            };
            let mapped_depth_stencil_attachment =
                web_sys::GpuRenderPassDepthStencilAttachmentDescriptor::new(
                    &dsa.attachment.id.0,
                    &depth_load_op,
                    depth_store_op,
                    &stencil_load_op,
                    stencil_store_op,
                );

            mapped_desc.depth_stencil_attachment(&mapped_depth_stencil_attachment);
        }

        RenderPass(encoder.begin_render_pass(&mapped_desc))
    }

    fn command_encoder_end_render_pass(
        &self,
        _encoder: &Self::CommandEncoderId,
        pass: &mut Self::RenderPassId,
    ) {
        pass.0.end_pass();
    }

    fn command_encoder_finish(&self, encoder: &Self::CommandEncoderId) -> Self::CommandBufferId {
        Sendable(match encoder.label() {
            Some(ref label) => {
                let mut mapped_desc = web_sys::GpuCommandBufferDescriptor::new();
                mapped_desc.label(label);
                encoder.finish_with_descriptor(&mapped_desc)
            }
            None => encoder.finish(),
        })
    }

    fn command_encoder_insert_debug_marker(&self, _encoder: &Self::CommandEncoderId, _label: &str) {
        // Not available in gecko yet
        // encoder.insert_debug_marker(label);
    }

    fn command_encoder_push_debug_group(&self, _encoder: &Self::CommandEncoderId, _label: &str) {
        // Not available in gecko yet
        // encoder.push_debug_group(label);
    }

    fn command_encoder_pop_debug_group(&self, _encoder: &Self::CommandEncoderId) {
        // Not available in gecko yet
        // encoder.pop_debug_group();
    }

    fn command_encoder_write_timestamp(
        &self,
        _encoder: &Self::CommandEncoderId,
        _query_set: &Self::QuerySetId,
        _query_index: u32,
    ) {
        // Not available in gecko yet
    }

    fn command_encoder_resolve_query_set(
        &self,
        _encoder: &Self::CommandEncoderId,
        _query_set: &Self::QuerySetId,
        _first_query: u32,
        _query_count: u32,
        _destination: &Self::BufferId,
        _destination_offset: wgt::BufferAddress,
    ) {
        // Not available in gecko yet
    }

    fn render_bundle_encoder_finish(
        &self,
        encoder: Self::RenderBundleEncoderId,
        desc: &crate::RenderBundleDescriptor,
    ) -> Self::RenderBundleId {
        Sendable(match desc.label {
            Some(label) => {
                let mut mapped_desc = web_sys::GpuRenderBundleDescriptor::new();
                mapped_desc.label(label);
                encoder.0.finish_with_descriptor(&mapped_desc)
            }
            None => encoder.0.finish(),
        })
    }

    fn queue_write_buffer(
        &self,
        queue: &Self::QueueId,
        buffer: &Self::BufferId,
        offset: wgt::BufferAddress,
        data: &[u8],
    ) {
        /* Skip the copy once gecko allows BufferSource instead of ArrayBuffer
        queue.0.write_buffer_with_f64_and_u8_array_and_f64_and_f64(
            &buffer.0,
            offset as f64,
            data,
            0f64,
            data.len() as f64,
        );
        */
        queue
            .0
            .write_buffer_with_f64_and_buffer_source_and_f64_and_f64(
                &buffer.0,
                offset as f64,
                &js_sys::Uint8Array::from(data).buffer(),
                0f64,
                data.len() as f64,
            );
    }

    fn queue_write_texture(
        &self,
        queue: &Self::QueueId,
        texture: crate::TextureCopyView,
        data: &[u8],
        data_layout: wgt::TextureDataLayout,
        size: wgt::Extent3d,
    ) {
        let mut mapped_data_layout = web_sys::GpuTextureDataLayout::new();
        mapped_data_layout.bytes_per_row(data_layout.bytes_per_row);
        mapped_data_layout.rows_per_image(data_layout.rows_per_image);
        mapped_data_layout.offset(data_layout.offset as f64);

        /* Skip the copy once gecko allows BufferSource instead of ArrayBuffer
        queue.0.write_texture_with_u8_array_and_gpu_extent_3d_dict(
            &map_texture_copy_view(texture),
            data,
            &mapped_data_layout,
            &map_extent_3d(size),
        );
        */
        queue
            .0
            .write_texture_with_buffer_source_and_gpu_extent_3d_dict(
                &map_texture_copy_view(texture),
                &js_sys::Uint8Array::from(data).buffer(),
                &mapped_data_layout,
                &map_extent_3d(size),
            );
    }

    fn queue_submit<I: Iterator<Item = Self::CommandBufferId>>(
        &self,
        queue: &Self::QueueId,
        command_buffers: I,
    ) {
        let temp_command_buffers = command_buffers.map(|i| i.0).collect::<js_sys::Array>();

        queue.0.submit(&temp_command_buffers);
    }

    fn queue_get_timestamp_period(&self, _queue: &Self::QueueId) -> f32 {
        1.0 //TODO
    }
}

pub(crate) type SwapChainOutputDetail = ();

#[derive(Debug)]
pub struct BufferMappedRange {
    actual_mapping: js_sys::Uint8Array,
    temporary_mapping: Vec<u8>,
}

impl crate::BufferMappedRangeSlice for BufferMappedRange {
    fn slice(&self) -> &[u8] {
        &self.temporary_mapping
    }

    fn slice_mut(&mut self) -> &mut [u8] {
        &mut self.temporary_mapping
    }
}

impl Drop for BufferMappedRange {
    fn drop(&mut self) {
        // Copy from the temporary mapping back into the array buffer that was
        // originally provided by the browser
        let temporary_mapping_slice = self.temporary_mapping.as_slice();
        unsafe {
            // Note: no allocations can happen between `view` and `set`, or this
            // will break
            self.actual_mapping
                .set(&js_sys::Uint8Array::view(temporary_mapping_slice), 0);
        }
    }
}
