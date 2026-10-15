//! Baseline for the startup benchmark: an empty eframe window with the same renderer
//! (`BIBLE_RS_RENDERER`) and window options as bible-rs, painting one label. Its time to first
//! frame is the floor set by the platform (window + GPU context creation, first present);
//! bible-rs's extra cost is the difference.
//!
//! `cargo run --release -p bible-rs --example startup_floor`

use std::time::Instant;

use eframe::egui;

struct Floor {
    t0: Instant,
    frames: u32,
    built: f64,
}

impl eframe::App for Floor {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        egui::CentralPanel::default().show(ui, |ui| {
            ui.label("In the beginning was the Word.");
        });
        self.frames += 1;
        if self.frames == 1 {
            self.built = self.t0.elapsed().as_secs_f64() * 1000.0;
            ctx.request_repaint();
        } else if self.frames == 2 {
            println!(
                "{{\"first_frame_built_ms\":{:.2},\"first_frame_presented_ms\":{:.2}}}",
                self.built,
                self.t0.elapsed().as_secs_f64() * 1000.0
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

fn main() -> eframe::Result {
    let t0 = Instant::now();
    let viewport = egui::ViewportBuilder::default().with_title("floor").with_inner_size([1280.0, 860.0]);
    let options = eframe::NativeOptions {
        viewport,
        renderer: bible_rs::renderer(),
        wgpu_options: bible_rs::wgpu_options(),
        centered: true,
        event_loop_builder: bible_rs::background_launch(),
        ..Default::default()
    };
    eframe::run_native("floor", options, Box::new(move |_| Ok(Box::new(Floor { t0, frames: 0, built: 0.0 }))))
}
