#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::Path;

use float_pretty_print::PrettyPrintFloat;
use fltk::{enums::Event, image::PngImage, prelude::*, *};
use fltk_theme::{color_themes, ColorTheme, SchemeType, WidgetScheme};
use oxipng::{InFile, OutFile, PngResult};

mod mainview {
    fl2rust_macro::include_ui!("gui/mainview.fl");
}

#[derive(Clone)]
struct PNGItem {
    index: i32,
    filepath: String,
    orignal_size: usize,
    optimized_size: usize,
}

#[derive(Clone)]
enum Message {
    Start(Vec<String>, bool, bool, bool, bool, bool),
    Processing(PNGItem),
    UpdateProgress(i32, i32),
    Done(PNGItem),
    Error(PNGItem),
    AllDone,
}

#[tokio::main]
async fn main() {
    let app = app::App::default().with_scheme(app::Scheme::Gtk);

    let widget_scheme = WidgetScheme::new(SchemeType::Clean);
    widget_scheme.apply();

    let theme = ColorTheme::new(color_themes::BLACK_THEME);
    theme.apply();

    let (s, r) = app::channel::<Message>();

    let mut ui = mainview::UserInterface::make_window();

    let first_column_width = ui.win.width() - 320 - 16;

    ui.b_list
        .set_column_widths(&[first_column_width, 100, 80, 80, 70]);
    ui.b_list.set_column_char('|');
    ui.b_list.add("File|State|Original|Optimized|Percent");
    ui.b_list.handle({
        let sender = s.clone();

        let mut dnd = false;
        let mut released = false;
        let mut drop_in_pathes = Vec::<String>::new();

        move |_, ev| match ev {
            Event::DndEnter => {
                dnd = true;
                true
            }
            Event::DndDrag => true,
            Event::DndRelease => {
                released = true;
                true
            }
            Event::Paste => {
                if dnd && released {
                    let path = app::event_text();
                    let pathes = path.split('\n');

                    drop_in_pathes.clear();

                    // Only accepts png file
                    for p in pathes {
                        if p.ends_with(".png") {
                            drop_in_pathes.push(p.to_string());
                        }
                    }

                    let nb = ui.cb_nb.value();
                    let nc = ui.cb_nc.value();
                    let np = ui.cb_np.value();
                    let ng = ui.cb_np.value();
                    let nx = ui.cb_nx.value();

                    sender.send(Message::Start(drop_in_pathes.clone(), nb, nc, np, ng, nx));

                    dnd = false;
                    released = false;

                    true
                } else {
                    false
                }
            }
            Event::DndLeave => {
                dnd = false;
                released = false;
                true
            }
            _ => false,
        }
    });

    ui.win.set_icon(Some(
        PngImage::from_data(include_bytes!("../asset/icon.png")).unwrap(),
    ));
    ui.win.center_screen();

    while app.wait() {
        if let Some(msg) = r.recv() {
            match msg {
                Message::Start(pathes, nb, nc, np, ng, nx) => {
                    ui.b_list.clear();

                    ui.b_list.add("File|State|Original|Optimized|Percent");

                    for (i, p) in pathes.iter().enumerate() {
                        ui.b_list.add(&format_png_item(
                            &PNGItem {
                                index: i as i32,
                                filepath: p.clone(),
                                orignal_size: 0,
                                optimized_size: 0,
                            },
                            "...",
                            0.0,
                        ));
                    }

                    let sender = s.clone();
                    tokio::spawn(async move {
                        let total = pathes.len();
                        for (i, p) in pathes.iter().enumerate() {
                            let mut item = PNGItem {
                                index: i as i32,
                                filepath: p.clone(),
                                orignal_size: 0,
                                optimized_size: 0,
                            };

                            sender.send(Message::UpdateProgress(i as i32 + 1, total as i32));

                            match get_file_size(p) {
                                Ok(size) => {
                                    item.orignal_size = size;
                                }
                                Err(_) => {
                                    sender.send(Message::Error(item));
                                    continue;
                                }
                            }

                            sender.send(Message::Processing(item.clone()));

                            if optimize_png(p, nb, nc, np, ng, nx).is_err() {
                                sender.send(Message::Error(item));
                                continue;
                            }

                            item.optimized_size = get_file_size(p).unwrap();

                            sender.send(Message::Done(item));
                        }

                        sender.send(Message::AllDone);
                    });
                }
                Message::Processing(item) => {
                    ui.b_list
                        .set_text(item.index + 2, &format_png_item(&item, "Optimizing", 0.0));
                }
                Message::UpdateProgress(current, total) => {
                    ui.lb_info
                        .set_label(&format!("Optimizing {}/{}", current, total));
                }
                Message::Done(item) => {
                    ui.b_list.set_text(
                        item.index + 2,
                        &format_png_item(
                            &item,
                            "Done",
                            1.0 - item.optimized_size as f32 / item.orignal_size as f32,
                        ),
                    );
                }
                Message::Error(item) => {
                    println!("Error: {}", item.filepath);
                    ui.b_list
                        .set_text(item.index + 2, &format_png_item(&item, "Error", 0.0));
                }
                Message::AllDone => {
                    ui.lb_info.set_label("Done!");
                }
            }
        }
    }

    app.run().unwrap();
}

fn format_file_size(size: usize) -> String {
    // KB
    let mut result = size as f32 / 1024.0;
    let mut sizer = "KB";

    if result > 1024.0 {
        result /= 1024.0;
        sizer = "MB";
    }

    if result > 1024.0 {
        result /= 1024.0;
        sizer = "GB";
    }

    format!("{:.2} {}", PrettyPrintFloat(result.into()), sizer)
}

fn format_png_item(item: &PNGItem, state: &str, optimize_percent: f32) -> String {
    let mut short_path = Path::new(&item.filepath)
        .file_name()
        .unwrap()
        .to_str()
        .to_owned()
        .unwrap()
        .to_string();

    if short_path.len() > 40 {
        short_path.truncate(40);
        short_path += "...";
    }

    format!(
        "{}|{}|{}|{}|{}",
        short_path,
        state,
        if item.orignal_size == 0 {
            "".to_string()
        } else {
            format_file_size(item.orignal_size)
        },
        if item.optimized_size == 0 {
            "".to_string()
        } else {
            format_file_size(item.optimized_size)
        },
        if optimize_percent > 0.0 {
            format!("{:.1}%", optimize_percent * 100.0)
        } else {
            "".to_string()
        }
    )
}

fn get_file_size(path: &str) -> Result<usize, Box<dyn std::error::Error>> {
    let meta = std::fs::metadata(path)?;
    Ok(meta.len() as usize)
}

fn optimize_png(path: &str, nb: bool, nc: bool, np: bool, ng: bool, nx: bool) -> PngResult<()> {
    let mut _nb = nb;
    let mut _nc = nc;
    let mut _np = np;
    let mut _ng = ng;

    if nx {
        _nb = true;
        _nc = true;
        _np = true;
        _ng = true;
    }

    let mut opt = oxipng::Options {
        fix_errors: true,
        bit_depth_reduction: !_nb,
        color_type_reduction: !_nc,
        palette_reduction: !_np,
        grayscale_reduction: !_ng,
        ..Default::default()
    };

    if nx {
        opt.interlace = None;
    }

    let file_path = Path::new(path);
    oxipng::optimize(
        &InFile::Path(file_path.to_path_buf()),
        &OutFile::Path {
            path: Some(file_path.to_path_buf()),
            preserve_attrs: true,
        },
        &opt,
    )
}
