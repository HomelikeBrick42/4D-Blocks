use eframe::wgpu;

pub struct Texture<'a> {
    pub descriptor: wgpu::TextureDescriptor<'a>,
    pub texture: wgpu::Texture,
}

impl<'a> Texture<'a> {
    pub fn new(device: &wgpu::Device, descriptor: wgpu::TextureDescriptor<'a>) -> Self {
        let texture = device.create_texture(&descriptor);
        Self {
            descriptor,
            texture,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, new_size: cgmath::Vector2<u32>) -> bool {
        let size = self.size();
        if size.width != new_size.x || size.height != new_size.y {
            self.descriptor.size.width = new_size.x;
            self.descriptor.size.height = new_size.y;
            self.texture = device.create_texture(&self.descriptor);
            true
        } else {
            false
        }
    }
}

impl std::ops::Deref for Texture<'_> {
    type Target = wgpu::Texture;

    fn deref(&self) -> &Self::Target {
        &self.texture
    }
}
