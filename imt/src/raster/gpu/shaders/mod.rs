pub mod nonzero_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        types_meta: {
            use bytemuck::{Pod, Zeroable};

            #[derive(Clone, Copy, Zeroable, Pod)]
        },
        path: "./src/raster/gpu/shaders/nonzero_cs.glsl"
    }
}

pub mod downscale_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "./src/raster/gpu/shaders/downscale_cs.glsl"
    }
}

pub mod hinting_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "./src/raster/gpu/shaders/hinting_cs.glsl"
    }
}
