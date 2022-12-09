#version 450

layout(local_size_x = 8, local_size_y = 4, local_size_z = 1) in;

layout(push_constant) uniform Info {
    vec2 extent;
    uint numSegments;
    uint numRays;
} info;

layout(set = 0, binding = 0) readonly buffer RayData {
    vec2 vector[];
} rays;

layout(set = 0, binding = 1) readonly buffer SegmentData {
    vec4 data[];
} segments;

layout(set = 0, binding = 2, r8) writeonly uniform image2D bitmap;

bool ray_intersects(vec2 l1p1, vec2 l1p2, vec2 l2p1, vec2 l2p2) {
    vec2 r = l1p2 - l1p1;
    vec2 s = l2p2 - l2p1;
    float det = r.x * s.y - r.y * s.x;
    float u = ((l2p1.x - l1p1.x) * r.y - (l2p1.y - l1p1.y) * r.x) / det;
    float t = ((l2p1.x - l1p1.x) * s.y - (l2p1.y - l1p1.y) * s.x) / det;
    return (t >= 0.0 && t <= 1.0) && (u >= 0.0 && u <= 1.0);
}

bool sample_filled(vec2 raySrc) {
    // Resources
    // - https://en.wikipedia.org/wiki/Nonzero-rule
    // - https://stackoverflow.com/questions/1560492/how-to-tell-whether-a-point-is-to-the-right-or-left-side-of-a-line

    uint fillCount = 0;

    for(uint i = 0; i < info.numRays; i++) {
        vec2 rayDst = raySrc + (rays.vector[i].xy * 2.0);
        int hitSum = 0;

        for(uint j = 0; j < info.numSegments; j++) {
            if(ray_intersects(raySrc, rayDst, segments.data[j].xy, segments.data[j].zw)) {
                float w = ((segments.data[j].z - segments.data[j].x) * (raySrc.y - segments.data[j].y))
                    - ((segments.data[j].w - segments.data[j].y) * (raySrc.x - segments.data[j].x));
                
                if(w < 0.0) {
                    hitSum += 1;
                } else {
                    hitSum -= 1;
                }
            }
        }

        if(hitSum != 0) {
            fillCount += 1;;
        }
    }

    return fillCount == info.numRays;
}

void main() {
    vec2 raySrc = vec2(
        float(gl_GlobalInvocationID.x) / float(info.extent.x),
        float(gl_GlobalInvocationID.y) / float(info.extent.y)
    );

    uint fillCount = 0;

    for(uint i = 0; i < info.numRays; i++) {
        vec2 rayDst = raySrc + (rays.vector[i].xy * 2.0);
        int hitSum = 0;

        for(uint j = 0; j < info.numSegments; j++) {
            if(ray_intersects(raySrc, rayDst, segments.data[j].xy, segments.data[j].zw)) {
                float w = ((segments.data[j].z - segments.data[j].x) * (raySrc.y - segments.data[j].y))
                    - ((segments.data[j].w - segments.data[j].y) * (raySrc.x - segments.data[j].x));
                
                if(w < 0.0) {
                    hitSum += 1;
                } else {
                    hitSum -= 1;
                }
            }
        }

        if(hitSum != 0) {
            fillCount += 1;;
        }
    }

    if(fillCount == info.numRays) {
        imageStore(bitmap, ivec2(gl_GlobalInvocationID.x, gl_GlobalInvocationID.y), vec4(1.0));
    } else {
        imageStore(bitmap, ivec2(gl_GlobalInvocationID.x, gl_GlobalInvocationID.y), vec4(0.0));
    }
}
