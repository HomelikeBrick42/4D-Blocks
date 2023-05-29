use eframe::wgpu;

pub struct StorageBuffer<'a> {
    pub buffer_descriptor: wgpu::BufferDescriptor<'a>,
    pub buffer: wgpu::Buffer,
}

impl<'a> StorageBuffer<'a> {
    pub fn new(device: &wgpu::Device, buffer_descriptor: wgpu::BufferDescriptor<'a>) -> Self {
        let buffer = device.create_buffer(&buffer_descriptor);
        Self {
            buffer_descriptor,
            buffer,
        }
    }

    // Will reallocate buffer, and loose all previous data in the buffer
    pub fn set_size_lossy(&mut self, device: &wgpu::Device, size: wgpu::BufferAddress) {
        self.buffer_descriptor.size = size;
        self.buffer = device.create_buffer(&self.buffer_descriptor);
    }

    /// Will discard all other data in buffer if it has to reallocate
    ///
    /// Returns whether the buffer had to reallocate
    pub fn set_data_lossy(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
    ) -> bool {
        let reallocated = if self.buffer_descriptor.size < data.len() as wgpu::BufferAddress {
            self.set_size_lossy(device, data.len() as _);
            true
        } else {
            false
        };
        queue.write_buffer(&self.buffer, 0, data);
        reallocated
    }
}

impl std::ops::Deref for StorageBuffer<'_> {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
