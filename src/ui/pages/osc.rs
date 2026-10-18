use crate::content::factory::{generate_factory_wavetable, FACTORY_TABLES};
use crate::content::wavetables::{export_wav, import_wav, Wavetable, FRAME_SIZE};
use crate::params::NovaParams;
use crate::ui::{theme, widgets, UiShared};
use nih_plug::prelude::{Param, ParamSetter};
use nih_plug_egui::egui::{self, Pos2, Stroke, Ui, Vec2};
use std::sync::atomic::Ordering;

pub fn show(ui: &mut Ui, params: &NovaParams, setter: &ParamSetter<'_>, shared: &UiShared) {
    egui::Frame::group(ui.style())
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0_f32, theme::PANEL_BORDER))
        .corner_radius(egui::CornerRadius::same(theme::PANEL_RADIUS))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("OSC A · WAVETABLE").color(theme::CYAN));
                ui.separator();
                let mut enabled = params.osc_a.enable.value();
                if ui.checkbox(&mut enabled, "On").changed() {
                    setter.begin_set_parameter(&params.osc_a.enable);
                    setter.set_parameter(&params.osc_a.enable, enabled);
                    setter.end_set_parameter(&params.osc_a.enable);
                }
            });

            let loaded = shared.wavetable.load();
            ui.horizontal(|ui| {
                ui.label("Wavetable");
                egui::ComboBox::from_id_salt("osc-a-wavetable")
                    .selected_text(&loaded.name)
                    .width(180.0)
                    .show_ui(ui, |ui| {
                        let mut category = "";
                        for (index, info) in FACTORY_TABLES.iter().enumerate() {
                            if category != info.category {
                                category = info.category;
                                ui.label(
                                    egui::RichText::new(category)
                                        .color(theme::LABEL_TEXT)
                                        .strong(),
                                );
                            }
                            if ui
                                .selectable_label(loaded.name == info.name, info.name)
                                .clicked()
                            {
                                let sample_rate = f32::from_bits(
                                    shared.wavetable_sample_rate.load(Ordering::Relaxed),
                                );
                                start_table_load(
                                    params,
                                    shared,
                                    ui.ctx().clone(),
                                    move || {
                                        generate_factory_wavetable(index, sample_rate)
                                            .map_err(str::to_string)
                                    },
                                    |state, table| {
                                        state.selected_wavetable = table.name.clone();
                                        state.imported_wavetable_frames = None;
                                        state.imported_wavetable_name = None;
                                        state.imported_wavetable_source = None;
                                    },
                                );
                            }
                        }
                    });
                if shared.wavetable_loading.load(Ordering::Relaxed) {
                    ui.spinner();
                    ui.label("Preparing table…");
                }
                if let Ok(status) = shared.wavetable_status.lock() {
                    if let Some(status) = status.as_ref() {
                        let color = if status.starts_with("Exported") {
                            theme::CYAN
                        } else {
                            theme::AMBER
                        };
                        ui.label(egui::RichText::new(status).color(color).small());
                    }
                }
                ui.menu_button("Import ▾", |ui| {
                    let options_id = ui.id().with("wavetable-import-options");
                    let mut frame_size = ui
                        .data(|data| data.get_temp::<usize>(options_id))
                        .unwrap_or(0);
                    let mut auto_pitch = ui
                        .data(|data| data.get_temp::<bool>(options_id.with("auto-pitch")))
                        .unwrap_or(false);
                    let mut normalize = ui
                        .data(|data| data.get_temp::<bool>(options_id.with("normalize")))
                        .unwrap_or(true);
                    egui::ComboBox::from_id_salt(options_id.with("frame-size"))
                        .selected_text(if auto_pitch {
                            "Auto pitch detect"
                        } else {
                            match frame_size {
                                0 => "Auto (CLM / 2048)",
                                256 => "256 samples",
                                512 => "512 samples",
                                1024 => "1024 samples",
                                4096 => "4096 samples",
                                _ => "2048 samples",
                            }
                        })
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_value(&mut frame_size, 0, "Auto (CLM / 2048)")
                                .clicked()
                            {
                                auto_pitch = false;
                            }
                            for size in [256, 512, 1024, 2048, 4096] {
                                if ui
                                    .selectable_value(
                                        &mut frame_size,
                                        size,
                                        format!("{size} samples"),
                                    )
                                    .clicked()
                                {
                                    auto_pitch = false;
                                }
                            }
                            if ui
                                .selectable_label(auto_pitch, "Auto pitch detect")
                                .clicked()
                            {
                                auto_pitch = true;
                            }
                        });
                    ui.checkbox(&mut normalize, "Normalize frames (always remove DC)");
                    ui.data_mut(|data| {
                        data.insert_temp(options_id, frame_size);
                        data.insert_temp(options_id.with("auto-pitch"), auto_pitch);
                        data.insert_temp(options_id.with("normalize"), normalize);
                    });
                    if ui.button("Choose WAV…").clicked() {
                        import_from_dialog(
                            params,
                            shared,
                            ui.ctx().clone(),
                            (frame_size != 0).then_some(frame_size),
                            auto_pitch,
                            normalize,
                        );
                        ui.close_menu();
                    }
                });
                if ui
                    .add_enabled(
                        !shared.wavetable_loading.load(Ordering::Relaxed),
                        egui::Button::new("Export WAV"),
                    )
                    .clicked()
                {
                    if shared.wavetable_loading.swap(true, Ordering::Relaxed) {
                        return;
                    }
                    let table = shared.wavetable.load();
                    let context = ui.ctx().clone();
                    let status = shared.wavetable_status.clone();
                    let loading = shared.wavetable_loading.clone();
                    std::thread::spawn(move || {
                        let result = if let Some(path) = rfd::FileDialog::new()
                            .add_filter("WAV audio", &["wav"])
                            .set_file_name(format!("{}.wav", table.name))
                            .save_file()
                        {
                            std::fs::File::create(&path)
                                .map_err(|error| error.to_string())
                                .and_then(|file| export_wav(&table, FRAME_SIZE, file))
                                .map(|()| {
                                    format!(
                                        "Exported {}",
                                        path.file_name().map_or_else(
                                            || String::from("WAV"),
                                            |name| name.to_string_lossy().into_owned(),
                                        )
                                    )
                                })
                        } else {
                            Err(String::from("WAV export cancelled"))
                        };
                        let message = match result {
                            Ok(message) | Err(message) => message,
                        };
                        set_status(&status, Some(message));
                        loading.store(false, Ordering::Relaxed);
                        context.request_repaint();
                    });
                }
            });

            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width() - 225.0, 148.0),
                    egui::Sense::hover(),
                );
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 6.0, theme::BACKGROUND);
                let position = params.osc_a.wt_position.unmodulated_normalized_value();
                draw_frame(&painter, rect, &loaded, position);
                ui.vertical(|ui| {
                    let _ = widgets::knob::knob(ui, &params.osc_a.wt_position, setter, "WT Pos");
                    let _ = widgets::knob::knob(ui, &params.osc_a.level, setter, "Level");
                    let _ = widgets::knob::knob(ui, &params.osc_a.pan, setter, "Pan");
                });
            });

            ui.horizontal(|ui| {
                ui.label("Interpolation");
                let names = ["Frames", "Smooth", "Step"];
                let mut selected = params.osc_a.interp_mode.value().clamp(0, 2) as usize;
                egui::ComboBox::from_id_salt("osc-a-interpolation")
                    .selected_text(names[selected])
                    .show_ui(ui, |ui| {
                        for (index, name) in names.iter().enumerate() {
                            if ui.selectable_value(&mut selected, index, *name).changed() {
                                setter.begin_set_parameter(&params.osc_a.interp_mode);
                                setter.set_parameter(&params.osc_a.interp_mode, index as i32);
                                setter.end_set_parameter(&params.osc_a.interp_mode);
                            }
                        }
                    });
                ui.label(format!(
                    "Band-limited mipmaps · {} frames · {}",
                    loaded.frames.len(),
                    if loaded.source == "Nova factory" {
                        "Factory"
                    } else {
                        "Imported WAV"
                    }
                ));
            });
        });
}

fn draw_frame(painter: &egui::Painter, rect: egui::Rect, table: &Wavetable, position: f32) {
    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0_f32, theme::GRID),
    );
    let frames = &table.frames;
    let Some(frame) = frames
        .get((position.clamp(0.0, 1.0) * frames.len().saturating_sub(1) as f32).round() as usize)
    else {
        return;
    };
    let peak = frame
        .iter()
        .fold(0.0f32, |peak, sample| peak.max(sample.abs()))
        .max(1.0e-6);
    let points: Vec<Pos2> = (0..128)
        .map(|index| {
            let sample = frame[index * (FRAME_SIZE / 128)] / peak;
            Pos2::new(
                rect.left() + rect.width() * index as f32 / 127.0,
                rect.center().y - sample * (rect.height() * 0.43),
            )
        })
        .collect();
    painter.add(egui::Shape::line(
        points.clone(),
        Stroke::new(5.0_f32, theme::CYAN.gamma_multiply(0.12)),
    ));
    painter.add(egui::Shape::line(points, Stroke::new(2.0_f32, theme::CYAN)));
}

fn import_from_dialog(
    params: &NovaParams,
    shared: &UiShared,
    context: egui::Context,
    frame_size: Option<usize>,
    auto_pitch: bool,
    normalize: bool,
) {
    let sample_rate = f32::from_bits(shared.wavetable_sample_rate.load(Ordering::Relaxed));
    start_table_load(
        params,
        shared,
        context,
        move || {
            let path = rfd::FileDialog::new()
                .add_filter("WAV audio", &["wav"])
                .pick_file()
                .ok_or_else(|| String::from("WAV import cancelled"))?;
            let file = std::fs::File::open(&path).map_err(|error| error.to_string())?;
            import_wav(
                file,
                path.file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Imported wavetable"),
                path.to_string_lossy().to_string(),
                frame_size,
                auto_pitch,
                normalize,
                sample_rate,
            )
        },
        |state, table| {
            state.selected_wavetable = table.name.clone();
            state.imported_wavetable_name = Some(table.name.clone());
            state.imported_wavetable_source = Some(table.source.clone());
            state.imported_wavetable_frames = Some(
                table
                    .frames
                    .iter()
                    .map(|frame| {
                        frame
                            .iter()
                            .map(|sample| {
                                (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16
                            })
                            .collect()
                    })
                    .collect(),
            );
        },
    );
}

fn start_table_load(
    params: &NovaParams,
    shared: &UiShared,
    context: egui::Context,
    build: impl FnOnce() -> Result<Wavetable, String> + Send + 'static,
    persist: impl FnOnce(&mut crate::state::persisted::VersionedState, &Wavetable) + Send + 'static,
) {
    if shared.wavetable_loading.swap(true, Ordering::Relaxed) {
        return;
    }
    let generation = shared
        .wavetable_generation
        .fetch_add(1, Ordering::Relaxed)
        .wrapping_add(1);
    let generation_counter = shared.wavetable_generation.clone();
    let loading = shared.wavetable_loading.clone();
    let status = shared.wavetable_status.clone();
    let wavetable = shared.wavetable.clone();
    let persisted = params.persisted.clone();
    set_status(&status, None);

    std::thread::spawn(move || {
        let result = build();
        if generation_counter.load(Ordering::Relaxed) == generation {
            match result {
                Ok(table) => {
                    if let Ok(mut state) = persisted.lock() {
                        persist(&mut state, &table);
                    }
                    wavetable.replace(table);
                    set_status(&status, None);
                }
                Err(error) => set_status(&status, Some(error)),
            }
            loading.store(false, Ordering::Relaxed);
        }
        context.request_repaint();
    });
}

fn set_status(status: &std::sync::Mutex<Option<String>>, message: Option<String>) {
    if let Ok(mut status) = status.lock() {
        *status = message.map(|message| message.chars().take(120).collect());
    }
}
