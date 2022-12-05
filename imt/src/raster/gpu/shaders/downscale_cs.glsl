#version 450

layout(local_size_x = 8, local_size_y = 4, local_size_z = 1) in;

layout(set = 0, binding = 0, r8) readonly uniform image2D srcImage;
layout(set = 0, binding = 1) writeonly uniform image2D dstImage;

float pixelValue(ivec2 reqCoords) {
    ivec2 imageExtent = imageSize(srcImage);

    if(reqCoords.x < 0 || reqCoords.x >= imageExtent.x
        || reqCoords.y < 0 || reqCoords.y >= imageExtent.y)
    {
        return 0.0;
    }

    return imageLoad(srcImage, reqCoords).r;
}

float CubicHermite (float A, float B, float C, float D, float t) {
	float t2 = t*t;
    float t3 = t*t*t;
    float a = -A/2.0 + (3.0*B)/2.0 - (3.0*C)/2.0 + D/2.0;
    float b = A - (5.0*B)/2.0 + 2.0*C - D / 2.0;
    float c = -A/2.0 + C/2.0;
   	float d = B;
    return a*t3 + b*t2 + c*t + d;
}

void main() {
    ivec2 srcCoords = ivec2(
        int(gl_GlobalInvocationID.x) * 4,
        int(gl_GlobalInvocationID.y) * 4
    );

    float value = CubicHermite(
        CubicHermite(
            pixelValue(srcCoords + ivec2(0, 0)),
            pixelValue(srcCoords + ivec2(1, 0)),
            pixelValue(srcCoords + ivec2(2, 0)),
            pixelValue(srcCoords + ivec2(3, 0)),
            0.5
        ),
        CubicHermite(
            pixelValue(srcCoords + ivec2(0, 1)),
            pixelValue(srcCoords + ivec2(1, 1)),
            pixelValue(srcCoords + ivec2(2, 1)),
            pixelValue(srcCoords + ivec2(3, 1)),
            0.5
        ),
        CubicHermite(
            pixelValue(srcCoords + ivec2(0, 2)),
            pixelValue(srcCoords + ivec2(1, 2)),
            pixelValue(srcCoords + ivec2(2, 2)),
            pixelValue(srcCoords + ivec2(3, 2)),
            0.5
        ),
        CubicHermite(
            pixelValue(srcCoords + ivec2(0, 3)),
            pixelValue(srcCoords + ivec2(1, 3)),
            pixelValue(srcCoords + ivec2(2, 3)),
            pixelValue(srcCoords + ivec2(3, 3)),
            0.5
        ),
        0.5
    );

    imageStore(dstImage, ivec2(gl_GlobalInvocationID.x, gl_GlobalInvocationID.y), vec4(vec3(value), 1.0));

    /*float valueSum = 0.0;

    for(int i = 0; i < 4; i++) {
        for(int j = 0; j < 4; j++) {
            valueSum += pixelValue(srcCoords + ivec2(i, j));
        }
    }

    imageStore(dstImage, ivec2(gl_GlobalInvocationID.x, gl_GlobalInvocationID.y), vec4(vec3(valueSum / 16.0), 1.0));*/
}
