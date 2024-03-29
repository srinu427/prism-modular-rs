use glam::Vec4Swizzles;

#[derive(Copy, Clone)]
pub struct Camera3D{
    pub eye: glam::Vec4,
    pub dir: glam::Vec4,
    pub up: glam::Vec4,
    // x: n_plane, y: f_plane, z: fov, w: aspect
    pub info: glam::Vec4,
}

impl Camera3D{
    pub fn get_perspective_matrix(&self) -> glam::Mat4{
        let f = 1f32/(self.info.z/2f32).tan();
        glam::Mat4{
            x_axis: glam::Vec4::new(f/self.info.w, 0f32, 0f32, 0f32),
            y_axis: glam::Vec4::new(0f32, f, 0f32, 0f32),
            z_axis: glam::Vec4::new(
                0f32,
                0f32,
                (self.info.x + self.info.y)/(self.info.x - self.info.y),
                (2f32 * self.info.x * self.info.y)/(self.info.x - self.info.y)
            ),
            w_axis: glam::Vec4::new(0f32, 0f32, -1f32, 0f32),
        }.transpose()
    }

    pub fn get_view_matrix(&self) -> glam::Mat4{
        let proj_front = self.dir.xyz().normalize();
        let proj_up = (self.up.xyz() - self.up.xyz().dot(proj_front) * proj_front)
            .normalize();
        let proj_right = proj_front.cross(proj_up).normalize();

        glam::Mat4{
            x_axis: glam::Vec4::from((proj_right, 0f32)),
            y_axis: glam::Vec4::from((proj_up, 0f32)),
            z_axis: glam::Vec4::from((-proj_front, 0f32)),
            //z_axis: glam::Vec4::new(0f32, 0f32, 0f32, 1f32),
            w_axis: glam::Vec4::new(0f32, 0f32, 0f32, 1f32),
        }.transpose() * glam::Mat4{
            x_axis: glam::Vec4::new(1f32, 0f32, 0f32, -self.eye.x),
            y_axis: glam::Vec4::new(0f32, 1f32, 0f32, -self.eye.y),
            z_axis: glam::Vec4::new(0f32, 0f32, 1f32, -self.eye.z),
            w_axis: glam::Vec4::new(0f32, 0f32, 0f32, 1f32),
        }.transpose()
    }
}

pub struct CameraTransforms{
    pub view: glam::Mat4,
    pub proj: glam::Mat4,
}
