mod app;
mod camera;
mod chrome;
mod icon_bytes;
mod icons;
mod drive;
mod files;
mod library;
mod overlay;
mod renderer;
mod scene;
mod theme;

use app::PrintCaddyApp;

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        console_error_panic_hook::set_once();
        wasm_bindgen_futures::spawn_local(async {
            let canvas = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("printcaddy"))
                .expect("canvas #printcaddy")
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .expect("canvas element");
            let options = eframe::WebOptions {
                depth_buffer: 24,
                ..Default::default()
            };
            eframe::WebRunner::new()
                .start(
                    canvas,
                    options,
                    Box::new(|cc| Ok(Box::new(PrintCaddyApp::new(cc)))),
                )
                .await
                .expect("eframe failed to start");
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1280.0, 800.0])
                .with_title("PrintCaddy"),
            renderer: eframe::Renderer::Glow,
            depth_buffer: 24,
            ..Default::default()
        };
        eframe::run_native(
            "PrintCaddy",
            options,
            Box::new(|cc| Ok(Box::new(PrintCaddyApp::new(cc)))),
        )
        .expect("eframe failed to start");
    }
}
