#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use crate::egui::Pos2;
use eframe::egui;
use tray_icon::{
    menu::{AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayEvent, TrayIconBuilder,
};

use chrono::{DateTime, Timelike, Local};
use ini::Ini;
use std::fs;
use std::io::BufReader;
use rodio::{Decoder, OutputStream, Sink};

fn main() -> Result<(), eframe::Error> {
    let result = fs::read_to_string("conf.ini");
    if let Err(_) = result {
       fs::write("conf.ini", "[Config]\ntime=:30:,:00:\n#sound=sound.ogg").unwrap()
    }

    let i = Ini::load_from_file("conf.ini").unwrap();
    let mut time_str = "".to_string();
    let mut sound_path = "".to_string();
    for (sec, prop) in i.iter() {
        if let Some(s) = sec {
            if s == "Config" {
                for (k, v) in prop.iter() {
                    if k == "time" {
                        time_str = v.to_string();
                    } else if k == "sound" {
                        sound_path = v.to_string();
                    }
                }
            }
        }
    }

    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icon.png");
    let icon = load_icon(std::path::Path::new(path));

    let tray_menu = Menu::new();
    let quit_i = MenuItem::new("Quit", true, None);
    tray_menu.append_items(&[
        &PredefinedMenuItem::about(
            None,
            Some(AboutMetadata {
                name: Some("Rust clock".to_string()),
                copyright: Some("Copyright Hoothin @ 2023".to_string()),
                ..Default::default()
            }),
        ),
        &PredefinedMenuItem::separator(),
        &quit_i,
    ]);

    let _tray_icon = Some(
        TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("Rust clock")
            .with_icon(icon)
            .build()
            .unwrap(),
    );

    let options = eframe::NativeOptions {
        decorated: false,
        transparent: true,
        always_on_top: true,
        run_and_return: true,
        min_window_size: Some(egui::vec2(320.0, 100.0)),
        initial_window_size: Some(egui::vec2(320.0, 100.0)),
        default_theme: eframe::Theme::Dark,
        ..Default::default()
    };
    eframe::run_native(
        "Rust clock", // unused title
        options,
        Box::new(move |_cc| Box::new(MyApp{
            quit: quit_i.id(),
            visible: true,
            time2show: time_str,
            sound_path: sound_path,
            ..MyApp::default()
        })),
    )
}

#[derive(Default)]
struct MyApp {
    quit: u32,
    time: f32,
    time2show: String,
    tikpop: bool,
    visible: bool,
    last_pos_x: f32,
    last_pos_y: f32,
    last_visible: bool,
    sound_path: String,
    inited: bool
}

impl eframe::App for MyApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let init_x = -310.0;
        let init_y = 50.0;
        if self.inited == false {
            self.inited = true;
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "my_font".to_owned(),
                egui::FontData::from_static(include_bytes!("../assets/font.ttf")),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "my_font".to_owned());
            ctx.set_fonts(fonts);
        }
        clock_window_frame(ctx, frame, self);
        if self.tikpop == true {
            self.time += 2.0;
            frame.set_mouse_passthrough(false);
            if self.time < 100.0 {
                let add_x = (self.time / 200.0 * std::f32::consts::PI).sin() * -init_x;
                frame.set_window_pos(Pos2::new(init_x + add_x, init_y));
            } else if self.time > 250.0 && self.time < 350.0 {
                let add_x = ((self.time - 250.0) / 200.0 * std::f32::consts::PI).sin() * (init_x - 10.0);
                frame.set_window_pos(Pos2::new(add_x, init_y));
            } else if self.time > 350.0 {
                self.tikpop = false;
                self.visible = self.last_visible;
                frame.set_visible(self.visible);
                if self.visible == true {
                    frame.set_window_pos(Pos2::new(self.last_pos_x, self.last_pos_y));
                }
                frame.set_mouse_passthrough(true);
            }
            if self.visible == false {
                self.tikpop = false;
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        } else {
            let now: DateTime<Local> = Local::now();
            let hour = now.hour().to_string();
            let minute = now.minute().to_string();
            let second = now.second().to_string();
            if self.time2show != "" {
                let time2show_arr: Vec<&str> = self.time2show.split(',').collect();
                for x in &time2show_arr {
                    let single_time: Vec<&str> = x.split(':').collect();
                    if (single_time[0] == "" || single_time[0] == hour) &&
                    (single_time[1] == "" || single_time[1] == minute) &&
                    ((single_time[2] == "" && second == "0") || single_time[2] == second) {
                        self.last_visible = self.visible;
                        if let Some(pos) = frame.get_window_pos() {
                            self.last_pos_x = pos.x;
                            self.last_pos_y = pos.y;
                        }
                        self.visible = true;
                        self.tikpop = true;
                        frame.set_visible(true);
                        self.time = 0.0;
                        frame.set_window_pos(Pos2::new(init_x, init_y));
                        if self.sound_path != "" {
                            let result = fs::File::open(&self.sound_path);
                            if let Ok(file) = result {
                                let file = BufReader::new(file);
                                std::thread::spawn(move || {
                                    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                                    let source = Decoder::new(file).unwrap();
                                    let sink = Sink::try_new(&stream_handle).unwrap();
                                    sink.append(source);
                                    sink.sleep_until_end();
                                });
                            }
                        }
                        ctx.request_repaint_after(std::time::Duration::from_millis(16));
                        break;
                    }
                }
            }
        }

        if let Ok(TrayEvent {
            event: tray_icon::ClickEvent::Left,
            ..
        }) = tray_icon::TrayEvent::receiver().try_recv()
        {
            self.visible = !self.visible;
            frame.set_visible(self.visible);
            self.tikpop = false;
            self.time = 0.0;
            if self.visible == true {
                frame.set_window_pos(Pos2::new(0.0, init_y));
                frame.set_mouse_passthrough(true);
            }
        }
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.quit {
                std::process::exit(0)
            }
        }
    }
}

fn clock_window_frame(
    ctx: &egui::Context,
    frame: &mut eframe::Frame,
    app: &mut MyApp
) {
    use egui::*;
    let text_color = ctx.style().visuals.text_color();

    CentralPanel::default()
        .frame(Frame::none())
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            let painter = ui.painter();
            let now: DateTime<Local> = Local::now();

            painter.rect(
                rect.shrink(1.0),
                10.0,
                Color32::from_rgba_premultiplied(32, 33, 36, 200),
                Stroke::new(1.0, text_color),
            );

            painter.rect_filled(
                Rect::from_points(&[
                    Pos2::new(105.0, 25.0),
                    Pos2::new(305.0, 75.0)
                ]),
                5.0,
                Color32::from_rgb(0, 0, 0),
            );

            // Paint the title:
            painter.text(
                rect.center_top() + vec2(-40.0, 53.0),
                Align2::LEFT_CENTER,
                now.format("%H:%M:%S"),
                FontId::proportional(50.0),
                text_color,
            );

            painter.circle_filled(
                Pos2::new(55.0, 50.0),
                40.0,
                text_color
            );

            let (_, hour) = now.hour12();
            let minute = now.minute() as f32;
            let second = now.second() as f32;
            let rad = (hour as f32 + minute / 60.0) / 12.0 * std::f32::consts::PI * 2.0;
            //hour
            painter.line_segment(
                [
                    Pos2::new(55.0, 50.0),
                    Pos2::new(55.0, 50.0) + vec2(rad.sin() * 25.0, rad.cos() * -25.0),
                ],
                Stroke::new(3.0, Color32::from_rgb(0, 0, 0)),
            );
            let rad = minute / 60.0 * std::f32::consts::PI * 2.0;
            //minute
            painter.line_segment(
                [
                    Pos2::new(55.0, 50.0),
                    Pos2::new(55.0, 50.0) + vec2(rad.sin() * 35.0, rad.cos() * -35.0),
                ],
                Stroke::new(2.0, Color32::from_rgb(0, 0, 0)),
            );
            let rad = second / 60.0 * std::f32::consts::PI * 2.0;
            //second
            painter.line_segment(
                [
                    Pos2::new(55.0, 50.0),
                    Pos2::new(55.0, 50.0) + vec2(rad.sin() * 38.0, rad.cos() * -38.0),
                ],
                Stroke::new(1.0, Color32::from_rgb(255, 0, 0)),
            );

            let title_bar_response =
                ui.interact(rect, Id::new("title_bar"), Sense::click());
            if title_bar_response.is_pointer_button_down_on() {
                frame.drag_window();
            }

            if app.tikpop == false {
                let close_response = ui.put(
                    Rect::from_min_size(rect.left_top(), Vec2::splat(28.0)),
                    Button::new(RichText::new("❌").size(24.0)).frame(false),
                );
                if close_response.clicked() {
                    frame.set_visible(false);
                    app.visible = false;
                }
            }
        });
    ctx.request_repaint_after(std::time::Duration::from_millis(100));
}

fn load_icon(path: &std::path::Path) -> tray_icon::icon::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tray_icon::icon::Icon::from_rgba(icon_rgba, icon_width, icon_height)
        .expect("Failed to open icon")
}
