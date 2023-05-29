use eframe::{run_native, wgpu};
use std::sync::Arc;
use tesseracts::App;

fn main() -> Result<(), eframe::Error> {
    run_native(
        "4D Game",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Wgpu,
            vsync: false,
            icon_data: None,
            wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
                power_preference: wgpu::PowerPreference::HighPerformance,
                present_mode: wgpu::PresentMode::AutoNoVsync,
                device_descriptor: Arc::new(|_adapter| wgpu::DeviceDescriptor {
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        },
        Box::new(|cc| Box::new(App::new(cc))),
    )
}
