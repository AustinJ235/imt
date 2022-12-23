#version 450

layout(local_size_x = 8, local_size_y = 4, local_size_z = 1) in;

layout(set = 0, binding = 0, r8) readonly uniform image2D srcImage;
layout(set = 0, binding = 1, rgba8) writeonly uniform image2D dstImage;

float pixelValue(ivec2 reqCoords) {
    ivec2 imageExtent = imageSize(srcImage);

    if(reqCoords.x < 0 || reqCoords.x >= imageExtent.x
        || reqCoords.y < 0 || reqCoords.y >= imageExtent.y)
    {
        return 0.0;
    }

    return imageLoad(srcImage, reqCoords).r;
}

const float ONE_THIRD = 1.0 / 3.0;

void main() {
    ivec2 srcCoords = ivec2(
        int(gl_GlobalInvocationID.x) * 3,
        int(gl_GlobalInvocationID.y)
    );

    float a = pixelValue(srcCoords + ivec2(-1, 0)) * ONE_THIRD;
    float b = pixelValue(srcCoords + ivec2(0, 0)) * ONE_THIRD;
    float c = pixelValue(srcCoords + ivec2(1, 0)) * ONE_THIRD;
    float d = pixelValue(srcCoords + ivec2(2, 0)) * ONE_THIRD;
    float e = pixelValue(srcCoords + ivec2(3, 0)) * ONE_THIRD;

    imageStore(
        dstImage,
        ivec2(gl_GlobalInvocationID.x, gl_GlobalInvocationID.y),
        vec4(
            vec3(
                a + b + c,
                b + c + d,
                c + d + e
            ),
            1.0
        )
    );
}
